import { act,render,screen,within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach,beforeEach,expect,it,vi } from "vitest";
import { AdapterStatus } from "../features/AdapterStatus";
import { getDeviceAdapterMatches } from "../lib/backend";
import { evaluation } from "./adapterFixtures";
vi.mock("../lib/backend",()=>({getDeviceAdapterMatches:vi.fn()}));
const id="a".repeat(64);
beforeEach(()=>{vi.mocked(getDeviceAdapterMatches).mockResolvedValue(evaluation(id));});
afterEach(()=>vi.useRealTimers());
it("renders adapter and authentication requirements without any device actions",async()=>{
  const r=evaluation(id);r.selection="candidate";r.selectedAdapter=r.matches[0].adapter.id;r.matches[0].status="exact";r.connectionState="authentication_required";
  vi.mocked(getDeviceAdapterMatches).mockResolvedValue(r);render(<AdapterStatus deviceId={id} revision={1} desktop/>);
  const region=await screen.findByRole("region",{name:"Device adapter status"});
  expect(await within(region).findByText("Exact adapter candidate")).toBeInTheDocument();
  expect(region).toHaveTextContent("ssh key");expect(region).toHaveTextContent("authenticated endpoint handshake");
  expect(screen.queryByRole("button",{name:/connect|apply|extract/i})).not.toBeInTheDocument();
});
it.each(["protected","no_match","ambiguous"] as const)("renders %s conservatively",async selection=>{
  const r=evaluation(id);r.selection=selection;r.protectedDevice=selection==="protected";vi.mocked(getDeviceAdapterMatches).mockResolvedValue(r);
  render(<AdapterStatus deviceId={id} revision={1} desktop/>);
  expect(await screen.findByText(selection==="protected"?"Protected hardware":selection==="ambiguous"?"Ambiguous adapter match":"No compatible adapter")).toBeInTheDocument();
  expect(screen.getByText(/firmware extraction remain unavailable/)).toBeInTheDocument();
});
it("shows safe typed errors and retries independently",async()=>{
  vi.mocked(getDeviceAdapterMatches).mockRejectedValueOnce({code:"DEVICE_DISCONNECTED",message:"Device disconnected.",operation:null,retryable:true,suggestedAction:"Reconnect USB and refresh."});
  render(<AdapterStatus deviceId={id} revision={1} desktop/>);
  expect(await screen.findByRole("alert")).toHaveTextContent("Reconnect USB");
  await userEvent.click(screen.getByRole("button",{name:"Re-evaluate adapter"}));
  expect(await screen.findByText("No compatible adapter")).toBeInTheDocument();
});
it("withdraws expired results and stale discovery revisions",async()=>{
  vi.useFakeTimers();const r=evaluation(id);r.expiresAt=Date.now()+50;vi.mocked(getDeviceAdapterMatches).mockResolvedValue(r);
  const view=render(<AdapterStatus deviceId={id} revision={1} desktop/>);await act(async()=>{});
  expect(screen.getByText("No compatible adapter")).toBeInTheDocument();
  await act(async()=>vi.advanceTimersByTime(51));expect(screen.queryByText("No compatible adapter")).not.toBeInTheDocument();
  expect(screen.getByRole("status")).toHaveTextContent("expired");
  view.rerender(<AdapterStatus deviceId={id} revision={2} desktop/>);await act(async()=>{});
  expect(screen.getByRole("alert")).toHaveTextContent("Discovery changed");
});
it("ignores late responses and does not request hardware in browser preview",async()=>{
  let resolve!:(v:ReturnType<typeof evaluation>)=>void;
  vi.mocked(getDeviceAdapterMatches).mockReturnValueOnce(new Promise(r=>{resolve=r;})).mockResolvedValueOnce(evaluation("b".repeat(64),2));
  const view=render(<AdapterStatus deviceId={id} revision={1} desktop/>);
  view.rerender(<AdapterStatus deviceId={"b".repeat(64)} revision={2} desktop/>);await screen.findByText("No compatible adapter");
  const old=evaluation(id);old.reason="OUTDATED TARGET";await act(async()=>resolve(old));expect(screen.queryByText("OUTDATED TARGET")).not.toBeInTheDocument();
  view.unmount();vi.mocked(getDeviceAdapterMatches).mockClear();
  render(<AdapterStatus deviceId={id} revision={1} desktop={false}/>);expect(getDeviceAdapterMatches).not.toHaveBeenCalled();
});
it("uses passive saved-endpoint query and accepts its backend revision",async()=>{
  render(<AdapterStatus deviceId={id} targetKind="saved_endpoint" revision={null} desktop/>);
  await screen.findByText("No compatible adapter");expect(getDeviceAdapterMatches).toHaveBeenCalledWith(id,"saved_endpoint");
});
