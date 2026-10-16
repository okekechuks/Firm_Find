import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { Compatibility, CompatibilityPanel } from "../features/Compatibility";
import { getDeviceCompatibility, getProfileRegistry, recordDeviceObservation, validateDeviceProfile } from "../lib/backend";
import { report } from "./profileFixtures";
import { profileSchema } from "../types/profiles";
import observed from "../../../../device-profiles/observed-comfi-mg11.json";

vi.mock("../lib/backend", async (original) => ({...await original<typeof import("../lib/backend")>(),getDeviceCompatibility:vi.fn(),getProfileRegistry:vi.fn(),recordDeviceObservation:vi.fn(),validateDeviceProfile:vi.fn()}));
beforeEach(() => {
  vi.mocked(getDeviceCompatibility).mockResolvedValue(report());
  vi.mocked(getProfileRegistry).mockResolvedValue({profiles:[],diagnostics:[],digest:"fixture"});
  vi.mocked(recordDeviceObservation).mockResolvedValue();
});
afterEach(() => vi.useRealTimers());

it("renders four separate unknown decisions with reasons and no Apply action", async () => {
  render(<CompatibilityPanel deviceId="fixture-device" revision={1} desktop/>);
  const table=await screen.findByRole("table",{name:"Four-feature support matrix"});
  expect(within(table).getAllByText("UNKNOWN")).toHaveLength(4);
  for (const name of ["Custom DNS","Wi-Fi user limit","APN","USSD"]) expect(within(table).getByText(name)).toBeInTheDocument();
  expect(screen.getAllByText("Exact model and revision unverified.")).toHaveLength(4);
  expect(screen.getByText("Verified client-limit range: Unknown")).toBeInTheDocument();
  expect(screen.queryByRole("button",{name:/apply/i})).not.toBeInTheDocument();
});
it("separates supported but unimplemented from unsupported and unknown", async () => {
  const r=report();r.features.dns.assessment="SUPPORTED";r.features.ussd.assessment="UNSUPPORTED";
  vi.mocked(getDeviceCompatibility).mockResolvedValue(r);
  render(<CompatibilityPanel deviceId="fixture-device" revision={1} desktop/>);
  const table=await screen.findByRole("table");expect(within(table).getByText("SUPPORTED")).toBeInTheDocument();expect(within(table).getByText("UNSUPPORTED")).toBeInTheDocument();expect(within(table).getAllByText(/NOT IMPLEMENTED/)).toHaveLength(4);
});
it("shows loading and errors then recovers through re-evaluation", async () => {
  vi.mocked(getDeviceCompatibility).mockRejectedValueOnce(new Error("Discovery changed; refresh."));
  render(<CompatibilityPanel deviceId="fixture-device" revision={1} desktop/>);
  expect(screen.getByRole("status")).toHaveTextContent("Checking identity");
  expect(await screen.findByRole("alert")).toHaveTextContent("Discovery changed");
  expect(screen.queryByRole("table")).not.toBeInTheDocument();
  await userEvent.click(screen.getByRole("button",{name:"Re-evaluate support"}));
  expect(await screen.findByRole("table")).toBeInTheDocument();expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});
it("ignores late responses from a previous device or discovery generation", async () => {
  let resolve!: (value: ReturnType<typeof report>) => void;
  vi.mocked(getDeviceCompatibility).mockReturnValueOnce(new Promise((r) => {resolve=r;})).mockResolvedValueOnce(report("second",2));
  const view=render(<CompatibilityPanel deviceId="first" revision={1} desktop/>);
  view.rerender(<CompatibilityPanel deviceId="second" revision={2} desktop/>);
  await screen.findByRole("table");
  await act(async () => resolve({...report("first",1),confidence:"CONFLICTING"}));
  expect(screen.queryByText(/Profile match: CONFLICTING/)).not.toBeInTheDocument();expect(screen.getByRole("table")).toBeInTheDocument();
});
it("withdraws an expired result until fresh evidence is evaluated", async () => {
  vi.useFakeTimers();const r=report();r.expiresAt=Date.now()+50;vi.mocked(getDeviceCompatibility).mockResolvedValue(r);
  render(<CompatibilityPanel deviceId="fixture-device" revision={1} desktop/>);
  await act(async () => {});expect(screen.getByRole("table")).toBeInTheDocument();
  await act(async () => vi.advanceTimersByTime(51));expect(screen.queryByRole("table")).not.toBeInTheDocument();expect(screen.getByText(/Evaluation expired/)).toBeInTheDocument();
});
it("saves reported identity text through IPC and re-evaluates without claiming verification", async () => {
  const user=userEvent.setup();render(<CompatibilityPanel deviceId="fixture-device" revision={1} desktop/>);await screen.findByRole("table");
  await user.click(screen.getByText("Add user-reported identity evidence"));
  await user.type(screen.getByRole("textbox",{name:"Reported value"}),"LT800/");
  await user.click(screen.getByRole("button",{name:"Save reported evidence"}));
  await waitFor(() => expect(recordDeviceObservation).toHaveBeenCalledWith("fixture-device","reported_identifier","LT800/"));
  expect(getDeviceCompatibility).toHaveBeenCalledTimes(2);expect(screen.getByText(/cannot grant capability support/)).toBeInTheDocument();
});
it("exposes local schema validation as an unverified preview only", async () => {
  const p=profileSchema.parse(observed);vi.mocked(validateDeviceProfile).mockResolvedValue({profile:p,origin:"local_unverified",authoritative:false,contentHash:"fixture"});
  const user=userEvent.setup();render(<Compatibility devices={[]} revision={1} desktop/>);
  await user.click(screen.getByText("Validate a local profile document"));await user.type(screen.getByRole("textbox",{name:"Profile JSON"}),"test profile document");await user.click(screen.getByRole("button",{name:"Validate profile"}));
  expect(await screen.findByText(/Local unverified preview only/)).toBeInTheDocument();expect(validateDeviceProfile).toHaveBeenCalledWith("test profile document");
});
it("browser preview never requests device evaluation or registry IPC", () => {
  render(<Compatibility devices={[]} revision={0} desktop={false}/>);expect(getProfileRegistry).not.toHaveBeenCalled();expect(getDeviceCompatibility).not.toHaveBeenCalled();expect(screen.getByRole("combobox")).toBeDisabled();
});
