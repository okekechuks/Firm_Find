import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { DeviceFirmwareWorkflow } from "../components/DeviceFirmwareWorkflow";
import { workflowFixture } from "./workflow-fixture";
import type { Device } from "../types/contracts";
import type { DeviceWorkflow } from "../types/workflow";
import { LibraryRevisionContext } from "../lib/workspace-context";
const api=vi.hoisted(()=>({get:vi.fn(),acquire:vi.fn(),analysis:vi.fn(),cancel:vi.fn()}));
vi.mock("../lib/backend",async original=>({...await original<typeof import("../lib/backend")>(),getDeviceWorkflow:api.get,startDeviceFirmwareAcquisition:api.acquire,startFirmwareAnalysis:api.analysis,cancelLibraryJob:api.cancel}));
const unknown={state:"UNKNOWN" as const,controlPlane:null,canRead:false,canWrite:false,canVerify:false,canRollback:false,confidence:0,evidence:[]};
const device:Device={id:"a".repeat(64),displayName:"Synthetic MiFi",connected:true,transports:["usb"],vendorId:null,productId:null,containerId:null,interfaces:[],networks:[],category:"router_mifi_candidate",assessment:"Candidate",confidence:0,evidence:[],lastSeenAt:1,capabilities:{inventory:"SUPPORTED",backup:"UNKNOWN",flash:"UNSUPPORTED",features:{apn:unknown,wifiUserLimit:unknown,customDns:unknown,ussd:unknown}}};
function qualified():DeviceWorkflow {
  const w=workflowFixture(device);
  w.identity={...w.identity,confidence:"verified",verifiedManufacturer:"TP-Link",verifiedModel:"Archer C7",verifiedHardwareRevision:"v5"};
  w.officialSources=[{id:"qualified-synthetic-contract",vendor:"TP-Link",model:"Archer C7",revision:"v5",profile:"tplink_archer-c7-v5",target:"ath79/generic",version:"25.12.6",filename:"official.bin",sha256:"b".repeat(64),sizeBytes:1024,url:"https://downloads.openwrt.org/fixture",manifestUrl:"https://downloads.openwrt.org/fixture-sums",manifestSha256:"c".repeat(64),profilesSha256:"d".repeat(64),signingFingerprint:"fixture",primaryFingerprint:"fixture",verification:"pinned_catalogue_from_verified_release_signature",installability:"unknown"}];
  w.stages.find(s=>s.id==="acquisition")!.executable=true;return w;
}
beforeEach(()=>{vi.clearAllMocks();api.get.mockResolvedValue(workflowFixture(device));api.acquire.mockResolvedValue({});api.analysis.mockResolvedValue({});});
it("keeps unknown proprietary device useful without allowing acquisition",async()=>{
  render(<DeviceFirmwareWorkflow deviceId={device.id} revision={4} connected/>);
  expect(await screen.findByText(/User-reported model/)).toHaveTextContent("unverified");
  expect(screen.getByRole("button",{name:"Obtain Firmware"})).toBeDisabled();
  expect(screen.getAllByText(/No qualified firmware source for this exact device/).length).toBeGreaterThan(0);expect(api.acquire).not.toHaveBeenCalled();expect(api.analysis).not.toHaveBeenCalled();
});
it("requires explicit approval and submits the exact backend binding/source",async()=>{
  const w=qualified();api.get.mockResolvedValue(w);const user=userEvent.setup();
  render(<DeviceFirmwareWorkflow deviceId={device.id} revision={4} connected/>);
  const approval=await screen.findByRole("checkbox",{name:/Approve this exact target/});
  expect(screen.getByRole("button",{name:"Obtain Firmware"})).toBeDisabled();
  await user.click(approval);await user.click(screen.getByRole("button",{name:"Obtain Firmware"}));
  await waitFor(()=>expect(api.acquire).toHaveBeenCalledWith(w.binding,w.officialSources[0].id,true));
});
it("rejects stale inventory and disconnect even when the previous source was qualified",async()=>{
  api.get.mockResolvedValue(qualified());const user=userEvent.setup();const view=render(<DeviceFirmwareWorkflow deviceId={device.id} revision={4} connected/>);
  await user.click(await screen.findByRole("checkbox",{name:/Approve this exact target/}));
  view.rerender(<DeviceFirmwareWorkflow deviceId={device.id} revision={5} connected={false}/>);
  await waitFor(()=>expect(screen.getByRole("button",{name:"Obtain Firmware"})).toBeDisabled());expect(api.acquire).not.toHaveBeenCalled();
});
it("shows backend errors without claiming success or retrying a download",async()=>{
  api.get.mockResolvedValue(qualified());api.acquire.mockRejectedValue(new Error("Binding expired. No image published."));const user=userEvent.setup();
  render(<DeviceFirmwareWorkflow deviceId={device.id} revision={4} connected/>);
  await user.click(await screen.findByRole("checkbox",{name:/Approve this exact target/}));await user.click(screen.getByRole("button",{name:"Obtain Firmware"}));
  expect(await screen.findByRole("alert")).toHaveTextContent("Binding expired");expect(api.acquire).toHaveBeenCalledTimes(1);
});
it("retains the current view during background refresh and clears consent for a changed binding",async()=>{
  const w=qualified();api.get.mockResolvedValue(w);const user=userEvent.setup();
  const view=render(<LibraryRevisionContext.Provider value={0}><DeviceFirmwareWorkflow deviceId={device.id} revision={4} connected/></LibraryRevisionContext.Provider>);
  const checkbox=await screen.findByRole("checkbox",{name:/Approve this exact target/});await user.click(checkbox);
  let finish!:(value:DeviceWorkflow)=>void;
  api.get.mockReturnValueOnce(new Promise<DeviceWorkflow>(resolve=>{finish=resolve;}));
  view.rerender(<LibraryRevisionContext.Provider value={1}><DeviceFirmwareWorkflow deviceId={device.id} revision={4} connected/></LibraryRevisionContext.Provider>);
  await waitFor(()=>expect(api.get).toHaveBeenCalledTimes(2));
  expect(checkbox).toBeInTheDocument();expect(checkbox).toBeChecked();
  finish({...w,binding:{...w.binding,identityDigest:"e".repeat(64)}});
  await waitFor(()=>expect(checkbox).not.toBeChecked());
  expect(screen.getByRole("button",{name:"Obtain Firmware"})).toBeDisabled();expect(api.acquire).not.toHaveBeenCalled();
});
