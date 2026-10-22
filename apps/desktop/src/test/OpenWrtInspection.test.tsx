// Mocked IPC UI regressions only; genuine SSH and OS verification run separately.
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { OpenWrtInspection } from "../features/openwrt/OpenWrtInspection";
import * as backend from "../lib/openwrt";
import { authenticatedOpenWrt, inspectedOpenWrt, observedOpenWrt, openWrtFixture, openWrtPin, openWrtTarget } from "./openWrtFixtures";
import type { Device } from "../types/contracts";
import type { OpenWrtStatus } from "../types/openwrt";
vi.mock("../lib/openwrt", () => ({openWrtStatus: vi.fn(), openWrtObserve: vi.fn(), openWrtConnect: vi.fn(), openWrtApproveReads: vi.fn(), openWrtInspect: vi.fn(), openWrtCancel: vi.fn(), openWrtDisconnect: vi.fn()}));
const devices = [{connected: true, networks: [{id: "fixture-net", displayName: "USB router interface", linkUp: true, virtualInterface: false}]}] as Device[];
beforeEach(() => {
  vi.clearAllMocks(); vi.mocked(backend.openWrtStatus).mockResolvedValue(openWrtFixture()); vi.mocked(backend.openWrtObserve).mockResolvedValue(observedOpenWrt());
  vi.mocked(backend.openWrtConnect).mockResolvedValue(authenticatedOpenWrt()); vi.mocked(backend.openWrtApproveReads).mockResolvedValue({...authenticatedOpenWrt(), state: "read_authorized", readApproved: true});
  vi.mocked(backend.openWrtInspect).mockResolvedValue(inspectedOpenWrt()); vi.mocked(backend.openWrtCancel).mockResolvedValue(openWrtFixture({state: "disconnected"})); vi.mocked(backend.openWrtDisconnect).mockResolvedValue(openWrtFixture({state: "disconnected"}));
});
it("requires endpoint observation approval, independent host trust, native key selection and separate reads", async () => {
  const user = userEvent.setup(); render(<OpenWrtInspection devices={devices} onClose={vi.fn()}/>);
  expect(await screen.findByText("REAL DEVICE — INSPECTION MODE")).toBeInTheDocument(); await user.selectOptions(await screen.findByLabelText("OpenWrt target"), openWrtTarget);
  expect(screen.getByRole("button", {name: "Observe SSH host key"})).toBeDisabled();
  await user.click(screen.getByRole("checkbox", {name: /I own or administer/})); await user.click(screen.getByRole("button", {name: "Observe SSH host key"}));
  expect(backend.openWrtObserve).toHaveBeenCalledWith(openWrtTarget, 22); await screen.findByLabelText("Independently verified SSH fingerprint"); expect(screen.queryByText(openWrtPin)).not.toBeInTheDocument();
  const connect = screen.getByRole("button", {name: "Select SSH key and authenticate"}); expect(connect).toBeDisabled();
  await user.type(screen.getByLabelText("Independently verified SSH fingerprint"), openWrtPin); await user.click(screen.getByRole("checkbox", {name: /I independently checked/})); await user.click(connect);
  expect(backend.openWrtConnect).toHaveBeenCalledWith(openWrtPin, "root"); expect(await screen.findByRole("region", {name: "Authenticated OpenWrt identity"})).toHaveTextContent("25.12.6");
  expect(screen.queryByRole("button", {name: "Read configuration and firmware feasibility"})).not.toBeInTheDocument();
  const approve = screen.getByRole("button", {name: "Authorize read-only inspection"}); expect(approve).toBeDisabled(); await user.click(screen.getByRole("checkbox", {name: /I separately approve/})); await user.click(approve);
  await user.click(await screen.findByRole("button", {name: "Read configuration and firmware feasibility"}));
  expect(await screen.findByRole("region", {name: "OpenWrt DNS"})).toHaveTextContent("2606:4700:4700::1111");
  const report = screen.getByRole("region", {name: "Firmware Operations"}); expect(report).toHaveTextContent("No qualified physical extraction method."); expect(report).toHaveTextContent("REQUIRES TOOL");
  expect(screen.queryByRole("button", {name: /Apply|Flash|Extract firmware|Build firmware/i})).not.toBeInTheDocument();
});
it("keeps unknown firmware and regular MiFi restricted to identity inspection", async () => {
  vi.mocked(backend.openWrtStatus).mockResolvedValue(authenticatedOpenWrt(false)); render(<OpenWrtInspection devices={devices} onClose={vi.fn()}/>);
  expect(await screen.findByText(/Identity inspection only/)).toBeInTheDocument(); expect(screen.queryByRole("button", {name: "Authorize read-only inspection"})).not.toBeInTheDocument();
  expect(screen.getByText(/regular proprietary MiFi does not qualify/)).toBeInTheDocument(); expect(backend.openWrtInspect).not.toHaveBeenCalled();
});
it("rejects missing or protected interfaces and discloses privileged account restrictions", async () => {
  const user = userEvent.setup(); render(<OpenWrtInspection devices={[]} onClose={vi.fn()}/>); await user.selectOptions(await screen.findByLabelText("OpenWrt target"), openWrtTarget);
  expect(screen.getByRole("checkbox", {name: /I own or administer/})).toBeDisabled(); expect(screen.getByRole("button", {name: "Observe SSH host key"})).toBeDisabled();
  expect(screen.getByText(/Select a saved router with an active physical/)).toBeInTheDocument(); await user.click(screen.getByText("What this connection permits")); expect(screen.getByText(/privileged SSH account can write outside/)).toBeInTheDocument();
});
it("never permits observed or mismatched fingerprint alone to authenticate", async () => {
  vi.mocked(backend.openWrtStatus).mockResolvedValue(observedOpenWrt()); const user = userEvent.setup(); render(<OpenWrtInspection devices={devices} onClose={vi.fn()}/>); const field = await screen.findByLabelText("Independently verified SSH fingerprint");
  await user.type(field, `SHA256:${"b".repeat(43)}`); await user.click(screen.getByRole("checkbox", {name: /I independently checked/})); expect(screen.getByRole("button", {name: "Select SSH key and authenticate"})).toBeDisabled();
  expect(backend.openWrtConnect).not.toHaveBeenCalled(); expect(screen.queryByLabelText(/password|private key path/i)).not.toBeInTheDocument();
  expect(screen.queryByText(openWrtPin)).not.toBeInTheDocument();
  expect(screen.getByText(/observed value stays hidden/)).toBeInTheDocument();
});
it("cancels pending work, suppresses late results and permits a fresh request", async () => {
  const connected = {...authenticatedOpenWrt(), state: "read_authorized" as const, readApproved: true}; vi.mocked(backend.openWrtStatus).mockResolvedValue(connected);
  let resolve!: (value: OpenWrtStatus) => void; vi.mocked(backend.openWrtInspect).mockReturnValue(new Promise(done => {resolve = done;}));
  const user = userEvent.setup(); render(<OpenWrtInspection devices={devices} onClose={vi.fn()}/>); await user.click(await screen.findByRole("button", {name: "Read configuration and firmware feasibility"}));
  await user.click(screen.getByRole("button", {name: "Cancel OpenWrt operation"})); await waitFor(() => expect(screen.queryByText("Running bounded OpenWrt operation…")).not.toBeInTheDocument());
  await act(async () => resolve(inspectedOpenWrt())); expect(screen.queryByRole("region", {name: "Firmware Operations"})).not.toBeInTheDocument();
  await user.selectOptions(screen.getByLabelText("OpenWrt target"), openWrtTarget); await user.click(screen.getByRole("checkbox", {name: /I own or administer/})); await user.click(screen.getByRole("button", {name: "Observe SSH host key"})); expect(backend.openWrtObserve).toHaveBeenCalledWith(openWrtTarget, 22);
});
it("recovers controls after timeout and disconnects before closing", async () => {
  vi.mocked(backend.openWrtStatus).mockResolvedValueOnce({...authenticatedOpenWrt(), state: "read_authorized", readApproved: true}).mockResolvedValue(openWrtFixture({state: "failed"}));
  vi.mocked(backend.openWrtInspect).mockRejectedValue({code: "OPERATION_TIMEOUT", message: "SSH inspection timed out.", operation: null, retryable: true, suggestedAction: "Reconnect."});
  const user = userEvent.setup(); const close = vi.fn(); render(<OpenWrtInspection devices={devices} onClose={close}/>); await user.click(await screen.findByRole("button", {name: "Read configuration and firmware feasibility"}));
  expect(await screen.findByRole("alert")).toHaveTextContent("OPERATION_TIMEOUT"); await waitFor(() => expect(screen.queryByText("Running bounded OpenWrt operation…")).not.toBeInTheDocument());
  await user.selectOptions(screen.getByLabelText("OpenWrt target"), openWrtTarget); await user.click(screen.getByRole("checkbox", {name: /I own or administer/})); await user.click(screen.getByRole("button", {name: "Observe SSH host key"}));
  await screen.findByRole("region", {name: "SSH host identity"}); await user.click(screen.getByRole("button", {name: "Close OpenWrt inspection"})); expect(backend.openWrtDisconnect).toHaveBeenCalled(); expect(close).toHaveBeenCalledOnce();
});
it("does not count controlled OS testing as physical hardware and prevents expired approval", async () => {
  const status = observedOpenWrt(); vi.mocked(backend.openWrtStatus).mockResolvedValue({...status, targets: status.targets.map(target => ({...target, lab: true})), expiresAt: Date.now() - 1});
  render(<OpenWrtInspection devices={[]} onClose={vi.fn()}/>); expect(await screen.findByText("CONTROLLED OPENWRT VM — NOT PHYSICAL HARDWARE")).toBeInTheDocument();
  expect(screen.getByText(/approval expired/)).toBeInTheDocument(); expect(screen.getByLabelText("Independently verified SSH fingerprint")).toBeDisabled();
});
it("keeps loading and empty targets truthful and cancels session authority on navigation", async () => {
  let resolve!: (value: OpenWrtStatus) => void; vi.mocked(backend.openWrtStatus).mockReturnValue(new Promise(done => {resolve = done;}));
  const view = render(<OpenWrtInspection devices={devices} onClose={vi.fn()}/>); expect(screen.getByRole("status")).toHaveTextContent("Loading OpenWrt connection status");
  await act(async () => resolve(openWrtFixture({targets: []}))); expect(screen.getByText(/No saved router targets/)).toBeInTheDocument(); view.unmount();
  vi.mocked(backend.openWrtStatus).mockResolvedValue(authenticatedOpenWrt()); const next = render(<OpenWrtInspection devices={devices} onClose={vi.fn()}/>); await screen.findByRole("region", {name: "Authenticated OpenWrt identity"}); next.unmount(); expect(backend.openWrtCancel).toHaveBeenCalledOnce();
});
it("renders evidence-backed feasibility without introducing firmware action buttons", async () => {
  vi.mocked(backend.openWrtStatus).mockResolvedValue(inspectedOpenWrt()); const user = userEvent.setup(); render(<OpenWrtInspection devices={devices} onClose={vi.fn()}/>);
  const report = await screen.findByRole("region", {name: "Firmware Operations"}); await user.click(within(report).getByText("Firmware acquisition evidence and requirements")); expect(within(report).getByText("Official release directory")).toBeInTheDocument(); expect(report).toHaveTextContent("no firmware image or backup created"); expect(within(report).queryByRole("button")).not.toBeInTheDocument();
});
it("keeps configured, runtime and LAN DNS distinct and separates radio and BSS limit observations", async () => {
  const status = inspectedOpenWrt(); status.inspection = {...status.inspection!,
    runtimeDns: [{name: "wan", up: true, proto: "dhcp", dns: ["9.9.9.9"]}],
    dhcpDns: [{name: "lan", interface: "lan", ipv4_options: ["6,192.168.8.1"], ipv6_dns: ["fd00::1"], ra: "server", dhcpv6: "server"}],
    radios: [{name: "radio0", driver: "mac80211", maxassoc: "64", disabled: null}],
    wireless: [{name: "guest", radio: "radio0", ssid: "Fixture Guest", maxassoc: "12", mode: "ap", disabled: null}],
    wirelessRuntime: [{name: "hostapd.wlan0", driver: "nl80211", phy: "phy0", status: "ENABLED", associated_clients: 0}],
  };
  vi.mocked(backend.openWrtStatus).mockResolvedValue(status); render(<OpenWrtInspection devices={devices} onClose={vi.fn()}/>);
  const dns = await screen.findByRole("region", {name: "OpenWrt DNS"}); const tables = within(dns).getAllByRole("table");
  expect(tables[0]).toHaveTextContent("1.1.1.1"); expect(tables[1]).toHaveTextContent("9.9.9.9"); expect(tables[2]).toHaveTextContent("192.168.8.1"); expect(tables[2]).toHaveTextContent("fd00::1");
  const wifi = screen.getByRole("region", {name: "OpenWrt Wi-Fi"}); const wifiTables = within(wifi).getAllByRole("table");
  expect(wifiTables[0]).toHaveTextContent("64"); expect(wifiTables[1]).toHaveTextContent("Fixture Guest"); expect(wifiTables[1]).toHaveTextContent("12"); expect(wifiTables[2]).toHaveTextContent("nl80211"); expect(wifiTables[2]).toHaveTextContent("0"); expect(wifi).toHaveTextContent("does not establish a verified driver capacity");
});
it("requires a fresh read approval for every repeat inspection", async () => {
  vi.mocked(backend.openWrtStatus).mockResolvedValue(inspectedOpenWrt()); const user = userEvent.setup(); render(<OpenWrtInspection devices={devices} onClose={vi.fn()}/>);
  const approve = await screen.findByRole("button", {name: "Authorize read-only inspection"}); expect(approve).toBeDisabled(); expect(screen.queryByRole("button", {name: "Read configuration and firmware feasibility"})).not.toBeInTheDocument();
  await user.click(screen.getByRole("checkbox", {name: /I separately approve/})); await user.click(approve); await user.click(await screen.findByRole("button", {name: "Read configuration and firmware feasibility"})); expect(backend.openWrtApproveReads).toHaveBeenCalledOnce(); expect(backend.openWrtInspect).toHaveBeenCalledOnce();
});
