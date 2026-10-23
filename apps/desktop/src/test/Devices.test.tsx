import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { Devices } from "../features/Devices";
import type { DiscoverySnapshot } from "../types/contracts";

const checks = vi.hoisted(() => ({routes: vi.fn(), cancel: vi.fn(), approve: vi.fn(), export: vi.fn()}));
vi.mock("../lib/backend", async original => ({...await original<typeof import("../lib/backend")>(), approveFirmwareRouteProbe: checks.approve, discoverFirmwareRoutes: checks.routes, cancelFirmwareRouteDiscovery: checks.cancel, exportAcquisitionDiagnostics: checks.export}));
const stageIds = ["detected","exact_identity","management_api","official_image","firmware_export","physical_read","firmware_acquired","filesystem_extracted","customization","rebuild","installation"] as const;
const defaultStages = stageIds.map(id => ({id,state:id === "detected" ? "observed" : "not_implemented",reason:"No implementation is available."}));
function result() {
  return {deviceId:"a".repeat(64),discoveryRevision:4,expiresAt:Date.now()+60000,canObtain:false,identity:{confidence:"unverified",verifiedModel:null,verifiedHardwareRevision:null,verifiedFirmware:null,reportedModel:"Test MiFi"},protocols:[],probe:{attempted:false,code:"not_approved",httpStatus:null},candidates:[{strategy:"official_image",name:"Official firmware image",capability:{state:"unknown",reason:"No qualified source registered.",missingRequirements:["Exact hardware revision"]}}],capabilities:{},stages:defaultStages};
}
beforeEach(() => {
  vi.clearAllMocks();
  checks.approve.mockResolvedValue("c".repeat(32)); checks.routes.mockResolvedValue(result());checks.cancel.mockResolvedValue(undefined);checks.export.mockResolvedValue(true);
});

const unverifiedFeature = {
  state: "UNKNOWN" as const, controlPlane: null, canRead: false, canWrite: false,
  canVerify: false, canRollback: false, confidence: 0, evidence: [],
};
const inventory: DiscoverySnapshot = {
  revision: 4, scannedAt: 1_792_000_000_000, status: "ready", watching: true, warnings: [],
  devices: [{
    id: "a".repeat(64), displayName: "Test MiFi", connected: true, transports: ["usb", "serial", "network"],
    vendorId: "12D1", productId: "14DC", containerId: "11111111-1111-1111-1111-111111111111",
    interfaces: [{
      instanceId: "USB\\VID_12D1&PID_14DC\\FIXTURE", parentInstanceId: null,
      className: "Ports", classGuid: "PORTS-GUID", interfaceClasses: [], hardwareIds: ["USB\\VID_12D1&PID_14DC"],
      portName: "COM7", present: true,
    }],
    networks: [{ id: "fixture-adapter", displayName: "Test network", description: "USB network",
      interfaceIndex: 4, interfaceType: 6, linkUp: true, addresses: ["192.0.2.10"], gateways: ["192.0.2.1"] }],
    category: "router_mifi_candidate", assessment: "Possible router or MiFi", confidence: 70, evidence: ["USB identity match"],
    capabilities: { inventory: "SUPPORTED", backup: "UNKNOWN", flash: "UNSUPPORTED", features: {
      apn: unverifiedFeature, wifiUserLimit: unverifiedFeature, customDns: unverifiedFeature, ussd: unverifiedFeature,
    } }, lastSeenAt: 1_792_000_000_000,
  }, {
    id: "b".repeat(64), displayName: "Disconnected reader", connected: false, transports: ["usb"],
    vendorId: null, productId: null, containerId: null, interfaces: [], networks: [],
    category: "unrelated_device", assessment: "Unknown device", confidence: 0, evidence: ["Windows inventory only"],
    capabilities: { inventory: "SUPPORTED", backup: "UNKNOWN", flash: "UNSUPPORTED", features: {
      apn: unverifiedFeature, wifiUserLimit: unverifiedFeature, customDns: unverifiedFeature, ussd: unverifiedFeature,
    } }, lastSeenAt: 1_792_000_000_000,
  }],
};
const props = { inventory, desktop: true, error: null, refreshing: false, onRefresh: vi.fn(async () => {}) };

it("shows compact cards and a network summary without diagnostic clutter", async () => {
  const user=userEvent.setup(); render(<Devices {...props}/>);
  const card=screen.getByRole("article", {name:/Test MiFi:/});
  expect(card).toHaveTextContent("USB / Serial / Network");
  expect(screen.queryByText("Possible candidate")).not.toBeInTheDocument();
  expect(screen.queryByRole("button", {name:/Check compatibility/})).not.toBeInTheDocument();
  await user.click(screen.getByRole("button", {name:"More info for Test MiFi"}));
  const dialog=screen.getByRole("dialog", {name:"Test MiFi"});
  expect(dialog).toHaveTextContent("192.0.2.10"); expect(dialog).toHaveTextContent("192.0.2.1");
  expect(dialog).not.toHaveTextContent("COM7"); expect(dialog).not.toHaveTextContent("Identification");
  expect(within(dialog).getByRole("button", {name:"Obtain Firmware"})).toBeDisabled();
  await user.click(within(dialog).getByRole("button", {name:"Close device information"}));
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});

it("discovers backend routes without falsely marking a detected device incompatible", async () => {
  let finish!:(value: unknown)=>void;
  checks.routes.mockReturnValueOnce(new Promise(resolve => {finish=resolve;}));
  const user=userEvent.setup(); render(<Devices {...props}/>);
  await user.click(screen.getByRole("button", {name:"More info for Test MiFi"}));
  await user.click(screen.getByRole("button", {name:"Check Compatibility"}));
  expect(screen.getByRole("progressbar")).not.toHaveAttribute("value");
  expect(screen.getByRole("button", {name:"Check Compatibility"})).toBeDisabled();
  finish(result());
  expect(await screen.findByRole("heading",{name:"No currently qualified acquisition method"})).toBeInTheDocument();
  expect(screen.queryByText("Cannot locate firmware")).not.toBeInTheDocument();
  expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  expect(screen.getByRole("button", {name:"Obtain Firmware"})).toBeDisabled();
  expect(checks.routes).toHaveBeenCalledWith(expect.objectContaining({deviceId:inventory.devices[0].id,approvalToken:null,advancedEndpoint:false}));
  expect(screen.getByRole("article",{name:/Test MiFi:/})).toHaveClass("device-warning");
  expect(screen.getByText(/Official firmware image: unknown/)).toBeInTheDocument();
  await user.click(screen.getByRole("button",{name:"Export Diagnostics"}));
  expect(await screen.findByText("Redacted diagnostics saved.")).toBeInTheDocument();
});

it("discards a check completed after disconnection", async () => {
  let finish!:(value: unknown)=>void;
  checks.routes.mockReturnValueOnce(new Promise(resolve => {finish=resolve;}));
  const user=userEvent.setup(); const view=render(<Devices {...props}/>);
  await user.click(screen.getByRole("button", {name:"More info for Test MiFi"}));
  await user.click(screen.getByRole("button", {name:"Check Compatibility"}));
  view.rerender(<Devices {...props} inventory={{...inventory,revision:5,devices:inventory.devices.map(d=>({...d,connected:false}))}}/>);
  finish(result());
  expect(await screen.findByText(/Device disconnected/)).toBeInTheDocument();
  expect(screen.queryByText("Cannot locate firmware")).not.toBeInTheDocument();
  expect(screen.queryByRole("heading",{name:"No currently qualified acquisition method"})).not.toBeInTheDocument();
  expect(screen.getByRole("button", {name:"Obtain Firmware"})).toBeDisabled();
});

it("requires explicit endpoint approval and forwards only the selected target", async () => {
  const user=userEvent.setup();render(<Devices {...props}/>);
  await user.click(screen.getByRole("button",{name:"More info for Test MiFi"}));
  await user.click(screen.getByText("Device observations and management endpoint"));
  const approval=screen.getByRole("checkbox",{name:/I own or administer this endpoint/});
  expect(approval).not.toBeChecked();await user.click(approval);
  await user.click(screen.getByRole("button",{name:"Check Compatibility"}));
  expect(checks.routes).toHaveBeenCalledWith({deviceId:"a".repeat(64),endpoint:"http://192.0.2.1/",interfaceId:"fixture-adapter",approvalToken:"c".repeat(32),advancedEndpoint:false});
  await user.clear(screen.getByRole("textbox",{name:"Management address"}));
  expect(approval).not.toBeChecked();expect(screen.queryByRole("region",{name:"Firmware acquisition routes"})).not.toBeInTheDocument();
});

it("rejects an expired backend result and leaves obtain disabled", async () => {
  checks.routes.mockResolvedValueOnce({...result(),expiresAt:Date.now()-1});
  const user=userEvent.setup();render(<Devices {...props}/>);
  await user.click(screen.getByRole("button",{name:"More info for Test MiFi"}));
  await user.click(screen.getByRole("button",{name:"Check Compatibility"}));
  expect(await screen.findByRole("alert")).toHaveTextContent("Device information changed");
  expect(screen.getByRole("button",{name:"Obtain Firmware"})).toBeDisabled();
});

it("labels Ethernet/Wi-Fi as transport to a remote router, not a verified router identity", () => {
  const gateway={...inventory.devices[0],transports:["network" as const],category:"generic_network_interface" as const,displayName:"PC adapter"};
  render(<Devices {...props} inventory={{...inventory,devices:[gateway]}}/>);
  expect(screen.getByRole("heading",{name:"Router via Test network"})).toBeInTheDocument();
  expect(screen.queryByRole("heading",{name:"PC adapter"})).not.toBeInTheDocument();
});

it("filters disconnected devices and connection types", async () => {
  const user = userEvent.setup();
  render(<Devices {...props} />);
  const search = screen.getByRole("textbox", { name: "Search devices" });
  await user.click(screen.getByRole("checkbox", {name: "Show advanced Windows diagnostics and unrelated devices"}));
  await user.click(screen.getByRole("checkbox", {name:"Show disconnected"}));
  await user.type(search, "reader");
  expect(screen.getByRole("heading", { name: "Disconnected reader" })).toBeInTheDocument();
  await user.click(screen.getByRole("checkbox", { name: "Show disconnected" }));
  expect(screen.queryByRole("heading", { name: "Disconnected reader" })).not.toBeInTheDocument();
  await user.clear(search);
  await user.selectOptions(screen.getByRole("combobox", { name: "Filter by connection" }), "serial");
  expect(screen.getByRole("heading", { name: "Test MiFi" })).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Disconnected reader" })).not.toBeInTheDocument();
});

it("explains inventory scan errors and leaves browser preview hardware-free", () => {
  const { rerender } = render(<Devices {...props} error="Windows inventory could not be refreshed."/>);
  expect(screen.getByRole("alert")).toHaveTextContent("Windows inventory could not be refreshed.");
  rerender(<Devices {...props} inventory={null} desktop={false}/>);
  expect(screen.getByRole("heading", { name: "Open the Windows app to discover devices" })).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Refresh devices" })).not.toBeInTheDocument();
});

it("allows manual recovery when the initial inventory read fails", async () => {
  const onRefresh = vi.fn(async () => {});
  render(<Devices {...props} inventory={null} error="Initial read failed" onRefresh={onRefresh} />);
  expect(screen.getByRole("heading", { name: "Inventory unavailable" })).toBeInTheDocument();
  const refresh = screen.getByRole("button", { name: "Refresh devices" });
  expect(refresh).toBeEnabled();
  await userEvent.click(refresh);
  expect(onRefresh).toHaveBeenCalledOnce();
});

it("distinguishes a pending first scan from a successful empty inventory", () => {
  const { rerender } = render(<Devices {...props} inventory={null} />);
  expect(screen.getByText("Reading Windows device inventory…")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Refresh devices" })).toBeDisabled();
  rerender(<Devices {...props} inventory={{ ...inventory, devices: [] }} />);
  expect(screen.getByRole("heading", { name: "No devices to show" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Refresh devices" })).toBeEnabled();
});

it("keeps manual and OpenWrt controls out of the discovery toolbar", () => {
  render(<Devices {...props}/>);
  for (const name of ["Add Router", "Manage saved connections", "Inspect OpenWrt router"])
    expect(screen.queryByRole("button", {name})).not.toBeInTheDocument();
});

it("shows bounded metadata as a fact and preserves separately sourced firmware versions", async () => {
  const r=result();checks.routes.mockResolvedValueOnce({...r,probe:{...r.probe,metadataTruncated:true},identity:{...r.identity,evidence:[{id:"one",source:"endpoint_response",attribute:"firmware_version",verification:"observed",value:"REL_MRVL_UAPXC_v2.6.0"},{id:"two",source:"user_supplied",attribute:"firmware_version",verification:"user_reported",value:"MF69_AM802L_ZX_DD_N_241106_EU"}]}});
  const user=userEvent.setup();render(<Devices {...props}/>);await user.click(screen.getByRole("button",{name:"More info for Test MiFi"}));await user.click(screen.getByRole("button",{name:"Check Compatibility"}));
  expect(await screen.findByText("Metadata limited to first 16 KiB.")).toBeInTheDocument();
  expect(screen.getByText(/Endpoint-observed version: REL_MRVL/)).toBeInTheDocument();expect(screen.getByText(/User-reported firmware: MF69/)).toBeInTheDocument();expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});

it("does not dispatch discovery after native approval is declined", async()=>{
  checks.approve.mockRejectedValueOnce(new Error("Endpoint check not approved."));const user=userEvent.setup();render(<Devices {...props}/>);await user.click(screen.getByRole("button",{name:"More info for Test MiFi"}));await user.click(screen.getByText("Device observations and management endpoint"));await user.click(screen.getByRole("checkbox",{name:/I own or administer/}));await user.click(screen.getByRole("button",{name:"Check Compatibility"}));expect(await screen.findByRole("alert")).toHaveTextContent("Endpoint check not approved.");expect(checks.routes).not.toHaveBeenCalled();expect(screen.getByRole("button",{name:"Check Compatibility"})).toBeEnabled();
});
it("shows eleven compatibility stages with the existing five summary rows first", async()=>{
  const user=userEvent.setup();render(<Devices {...props}/>);await user.click(screen.getByRole("button",{name:"More info for Test MiFi"}));await user.click(screen.getByRole("button",{name:"Check Compatibility"}));
  const summary=await screen.findByLabelText("Compatibility summary");
  expect([...summary.querySelectorAll("dt")].map(node=>node.textContent?.split(":")[0])).toEqual(["Detected","Exact identity","Management API","Official image","Firmware export"]);
  const rest=screen.getByText("Remaining compatibility stages").closest("details")!;
  expect(rest).not.toHaveAttribute("open");await user.click(screen.getByText("Remaining compatibility stages"));
  expect(within(rest).getAllByRole("term")).toHaveLength(6);
});

it("renders every compatibility state neutrally and keeps stages independent", async()=>{
  const states=["observed","verified","unknown","not_checked","not_supported","not_implemented","not_implemented","not_implemented","not_implemented","not_implemented","not_implemented"] as const;
  const r=result();checks.routes.mockResolvedValueOnce({...r,stages:r.stages.map((stage,index)=>({...stage,state:states[index]}))});
  const user=userEvent.setup();render(<Devices {...props}/>);await user.click(screen.getByRole("button",{name:"More info for Test MiFi"}));await user.click(screen.getByRole("button",{name:"Check Compatibility"}));
  const summary=await screen.findByLabelText("Compatibility summary");
  for(const state of ["observed","verified","unknown","not_checked","not_supported","not_implemented"]) expect(summary.closest(".acquisition-results")).toHaveTextContent(state.replaceAll("_"," "));
  expect(summary.closest(".acquisition-results")?.querySelectorAll(".error, .device-error, [data-tone='error']")).toHaveLength(0);
  await user.click(screen.getByText("Remaining compatibility stages"));
  expect(screen.getByText("Exact identity: verified")).toBeInTheDocument();
  expect(screen.getByText("Official image: not checked")).toBeInTheDocument();
  expect(screen.getByText("Physical read: not implemented")).toBeInTheDocument();
  expect(screen.getByRole("button",{name:"Obtain Firmware"})).toBeDisabled();
});
