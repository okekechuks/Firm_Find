// Synthetic transport fixtures only. Native acceptance uses real IPC and generated file bytes.
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { App } from "../app/App";
import { PageHelp } from "../components/PageHelp";
import { CustomizationPanel } from "../components/CustomizationPanel";
import { WorkspaceProvider } from "../components/WorkspaceProvider";
import { Dashboard } from "../features/Dashboard";
import { FirmwareLibrary } from "../features/FirmwareLibrary";
import { Backups } from "../features/Backups";
import { Jobs, JobRows } from "../features/Jobs";
import { Preferences } from "../features/Preferences";
import { WorkspaceContext, type WorkspaceValue } from "../lib/workspace-context";
import { deviceStatus, extractionEligibility } from "../lib/device-status";
import { defaultName } from "../lib/format";
import type { Device, Snapshot, DiscoverySnapshot } from "../types/contracts";
import * as backend from "../lib/backend";
import { artifact, backup, desktopState, details, empty, job } from "./libraryFixtures";
vi.mock("../lib/backend", async original => ({ ...await original<typeof import("../lib/backend")>(), getSnapshot:vi.fn(),hasDesktopBackend:vi.fn(),saveSettings:vi.fn(),getDiscovery:vi.fn(),refreshDiscovery:vi.fn(),onDiscoveryChanged:vi.fn(),onLibraryChanged:vi.fn(),getDesktopState:vi.fn(),saveDesktopPreferences:vi.fn(),listLibraryJobs:vi.fn(),getLibraryActivity:vi.fn(),listFirmwareImportHistory:vi.fn(),listFirmwareArtifacts:vi.fn(),getFirmwareArtifact:vi.fn(),listFirmwareBackups:vi.fn(),createFirmwareBackup:vi.fn(),restoreFirmwareBackup:vi.fn(),removeFirmwareItem:vi.fn(),openManagedFolder:vi.fn(),selectLibraryDirectory:vi.fn(),relocateFirmwareLibrary:vi.fn(),getManualRouters:vi.fn() }));
const snapshot: Snapshot = { version:"0.1.0",databaseVersion:5,dataDirectory:"C:\\test\\OBTAINER",settings:{theme:"system"},counts:{devices:30,artifacts:1,runningJobs:0},recentEvents:[{id:1,kind:"application_started",message:"Historical startup event",createdAt:new Date().toISOString()}] };
const feature = {state:"UNKNOWN" as const,controlPlane:null,canRead:false,canWrite:false,canVerify:false,canRollback:false,confidence:0,evidence:[]};
const candidate: Device = { id:"a".repeat(64),displayName:"Synthetic MiFi",connected:true,transports:["usb"],vendorId:"1286",productId:"4E31",containerId:null,interfaces:[],networks:[],category:"router_mifi_candidate",assessment:"Unverified device",confidence:50,evidence:[],capabilities:{inventory:"SUPPORTED",backup:"UNKNOWN",flash:"UNSUPPORTED",features:{apn:feature,wifiUserLimit:feature,customDns:feature,ussd:feature}},lastSeenAt:1 };
const inventory: DiscoverySnapshot = {revision:1,scannedAt:Date.now(),status:"ready",watching:true,warnings:[],devices:[candidate,{...candidate,id:"c".repeat(64),category:"generic_network_interface"}]};
function workspace(overrides: Partial<WorkspaceValue> = {}): WorkspaceValue { return {jobs:[],activity:[],state:desktopState,loading:false,error:null,revision:0,refresh:vi.fn(async()=>{}),cancel:vi.fn(async()=>{}),notify:vi.fn(),...overrides}; }
beforeEach(() => {
  sessionStorage.clear();
  vi.mocked(backend.hasDesktopBackend).mockReturnValue(true);vi.mocked(backend.getSnapshot).mockResolvedValue(snapshot);vi.mocked(backend.getDiscovery).mockResolvedValue(inventory);vi.mocked(backend.refreshDiscovery).mockResolvedValue(inventory);vi.mocked(backend.onDiscoveryChanged).mockResolvedValue(vi.fn());vi.mocked(backend.onLibraryChanged).mockResolvedValue(vi.fn());vi.mocked(backend.getDesktopState).mockResolvedValue(desktopState);vi.mocked(backend.listLibraryJobs).mockResolvedValue([]);vi.mocked(backend.getLibraryActivity).mockResolvedValue([]);vi.mocked(backend.listFirmwareImportHistory).mockResolvedValue([]);vi.mocked(backend.listFirmwareArtifacts).mockResolvedValue({...empty,artifacts:[artifact],total:1});vi.mocked(backend.getFirmwareArtifact).mockResolvedValue(details);vi.mocked(backend.listFirmwareBackups).mockResolvedValue([backup]);vi.mocked(backend.createFirmwareBackup).mockResolvedValue({...job,kind:"firmware_backup"});vi.mocked(backend.restoreFirmwareBackup).mockResolvedValue({...job,kind:"backup_restore"});vi.mocked(backend.removeFirmwareItem).mockResolvedValue();vi.mocked(backend.saveDesktopPreferences).mockResolvedValue(desktopState.preferences);vi.mocked(backend.relocateFirmwareLibrary).mockResolvedValue({...job,kind:"library_relocation"});vi.mocked(backend.getManualRouters).mockResolvedValue([]);
});
it("provides exactly six primary links in the approved order",async()=>{
 render(<App/>);await screen.findByText("Desktop connected");const nav=screen.getByRole("navigation",{name:"Main navigation"});expect(within(nav).getAllByRole("link").map(a=>a.textContent)).toEqual(["Overview","Connected Devices","Library","Backup Manager","Logs","Settings"]);expect(screen.queryByRole("dialog",{name:"How to use this page"})).not.toBeInTheDocument();
});
it.each(["Overview","Library","Backup Manager","Logs","Settings","Connected Devices"])("opens contextual help from %s",async name=>{
 const user=userEvent.setup();render(<App/>);await screen.findByText("Desktop connected");const nav=screen.getByRole("navigation",{name:"Main navigation"});await user.click(within(nav).getByRole("link",{name}));expect(within(nav).getByRole("link",{name})).toHaveAttribute("aria-current","page");await user.click(screen.getByRole("button",{name:"How to use this page"}));expect(await screen.findByRole("dialog",{name:"How to use this page"})).toBeInTheDocument();await user.keyboard("{Escape}");expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});

it("opens help only on demand and returns focus after close or Escape",async()=>{
  const user=userEvent.setup();render(<PageHelp page="dashboard"/>);const trigger=screen.getByRole("button",{name:"How to use this page"});expect(screen.queryByRole("dialog")).not.toBeInTheDocument();await user.click(trigger);expect(screen.getByRole("dialog")).toHaveTextContent("Completed and failed work moves to Logs");expect(screen.getByRole("button",{name:"Close page help"})).toHaveFocus();await user.keyboard("{Escape}");expect(trigger).toHaveFocus();await user.click(trigger);await user.click(screen.getByRole("button",{name:"Close page help"}));expect(trigger).toHaveFocus();
});
it("dismisses help outside and changes instructions with the page",async()=>{
  const user=userEvent.setup();const {rerender}=render(<><button>Outside</button><PageHelp key="devices" page="devices"/></>);await user.click(screen.getByRole("button",{name:"How to use this page"}));expect(screen.getByRole("dialog")).toHaveTextContent("Color never grants extraction permission");await user.click(screen.getByRole("button",{name:"Outside"}));expect(screen.queryByRole("dialog")).not.toBeInTheDocument();rerender(<PageHelp key="jobs" page="jobs"/>);await user.click(screen.getByRole("button",{name:"How to use this page"}));expect(screen.getByRole("dialog")).toHaveTextContent("Logs retains real operations");
});
it("shows four genuine statistics, only active jobs and no Overview history or extra device panel",()=>{
  const jobs=[{...job,id:"1",originalFilename:"active.bin"},{...job,id:"2",status:"pending" as const,originalFilename:"queued.bin"},{...job,id:"3",status:"completed" as const,originalFilename:"finished.bin"}];
  const value=workspace({jobs,state:{...desktopState,extracted:4,customized:2}});const {container}=render(<WorkspaceContext.Provider value={value}><Dashboard snapshot={snapshot} inventory={inventory} navigate={vi.fn()}/></WorkspaceContext.Provider>);
  const cards=container.querySelectorAll(".stat");expect(cards).toHaveLength(4);expect([...cards].map(c=>c.querySelector("strong")?.textContent)).toEqual(["1","2","2","4"]);expect(screen.getByText("active.bin")).toBeInTheDocument();expect(screen.getByText("queued.bin")).toBeInTheDocument();expect(screen.queryByText("finished.bin")).not.toBeInTheDocument();expect(screen.queryByText("Historical startup event")).not.toBeInTheDocument();expect(screen.queryByText(/Recent Activity/i)).not.toBeInTheDocument();expect(screen.queryByRole("button",{name:/Know your device/})).not.toBeInTheDocument();
});
it("uses indeterminate queued progress and exposes cancellation only for active jobs",()=>{
  render(<JobRows jobs={[{...job,status:"pending",stage:"queued",progressBytes:0}, {...job,id:"complete",status:"completed"}]} onCancel={vi.fn()}/>);expect(screen.getByRole("progressbar")).not.toHaveAttribute("value");expect(screen.getAllByRole("button",{name:"Cancel job"})).toHaveLength(1);expect(screen.queryByText("0%")).not.toBeInTheDocument();
});
it("tracks completion across navigation, removes active work and keeps its result in Logs",async()=>{
  const listeners:((j:typeof job)=>void)[]=[];vi.mocked(backend.onLibraryChanged).mockImplementation(async callback=>{listeners.push(callback);return ()=>{};});const current={...job,createdAt:Date.now(),originalFilename:"live-native-shape.bin"};vi.mocked(backend.listLibraryJobs).mockResolvedValue([current]);
  const user=userEvent.setup();render(<App/>);await screen.findByText(current.originalFilename);await user.click(within(screen.getByRole("navigation")).getByRole("link",{name:"Library"}));const complete={...current,status:"completed" as const,message:"Real persisted test outcome"};vi.mocked(backend.listLibraryJobs).mockResolvedValue([complete]);await act(async()=>listeners.forEach(fn=>fn(complete)));
  expect(await screen.findByText(/Firmware import complete/)).toBeInTheDocument();await user.click(within(screen.getByRole("navigation")).getByRole("link",{name:"Overview"}));expect(await screen.findByText("No active jobs")).toBeInTheDocument();await user.click(within(screen.getByRole("navigation")).getByRole("link",{name:"Logs"}));expect(await screen.findByText(/Real persisted test outcome/)).toBeInTheDocument();
});
it.each(["candidate","verified","protected","generic"])("classifies %s identity conservatively without granting extraction",kind=>{
  const device:Device=kind==="verified"?{...candidate,identity:{confidence:"verified",manufacturer:"Test",model:"Test router",firmware:null,explanation:"Independent fixture evidence",observations:[]}}:kind==="protected"?{...candidate,category:"unrelated_device"}:kind==="generic"?{...candidate,category:"generic_network_interface"}:candidate;
  expect(deviceStatus(device).tone).toBe(kind==="verified"?"success":kind==="candidate"?"warning":"error");expect(extractionEligibility(device).allowed).toBe(false);
});
it("protected PC classes override a misleading router category and verified label",()=>{
 const device={...candidate,interfaces:[{instanceId:"internal",parentInstanceId:null,className:"DiskDrive",classGuid:"",interfaceClasses:[],hardwareIds:[],portName:null,present:true}]};expect(deviceStatus(device).tone).toBe("error");expect(extractionEligibility({...device,connected:false}).reason).toContain("disconnected");
});
it("never enables unsupported customization toggles or a proprietary rebuild",async()=>{
 const user=userEvent.setup();const close=vi.fn();render(<CustomizationPanel details={details} onClose={close}/>);expect(screen.getAllByRole("switch")).toHaveLength(4);for(const toggle of screen.getAllByRole("switch")){expect(toggle).toBeDisabled();await user.click(toggle);expect(toggle).not.toBeChecked();}expect(screen.getByRole("button",{name:/Rebuild firmware/})).toBeDisabled();await user.click(screen.getByRole("button",{name:"Cancel"}));expect(close).toHaveBeenCalledOnce();
});
it("tags user imports correctly and opens the capability-aware customization panel",async()=>{
 const user=userEvent.setup();render(<FirmwareLibrary desktop/>);const card=await screen.findByRole("button",{name:/harmless.bin/});expect(card).toHaveTextContent("Imported Original");expect(card).not.toHaveTextContent("Extracted");await user.click(card);await user.click(await screen.findByRole("button",{name:"Customize"}));expect(screen.getByRole("region",{name:"Customize firmware"})).toBeInTheDocument();
});
it("requires explicit per-file delete confirmation and preserves data when cancelled",async()=>{
 const user=userEvent.setup();render(<FirmwareLibrary desktop/>);await user.click(await screen.findByRole("button",{name:/harmless.bin/}));await user.click(await screen.findByRole("button",{name:"Delete"}));let dialog=screen.getByRole("dialog");expect(dialog).toHaveTextContent("harmless.bin");expect(dialog).toHaveTextContent("stored bytes");expect(backend.removeFirmwareItem).not.toHaveBeenCalled();await user.click(within(dialog).getByRole("button",{name:"Cancel"}));expect(backend.removeFirmwareItem).not.toHaveBeenCalled();await user.click(screen.getByRole("button",{name:"Delete"}));dialog=screen.getByRole("dialog");await user.click(within(dialog).getByRole("button",{name:"Delete from Library"}));expect(backend.removeFirmwareItem).toHaveBeenCalledWith(artifact.sha256,false);
});
it("creates a backup through real command contract rather than a cosmetic button",async()=>{
 const user=userEvent.setup();render(<FirmwareLibrary desktop/>);await user.click(await screen.findByRole("button",{name:/harmless.bin/}));await user.click(await screen.findByRole("button",{name:"Create Backup"}));expect(backend.createFirmwareBackup).toHaveBeenCalledWith(artifact.sha256);
});
it("restores only the selected backup after confirmation and keeps the original visible",async()=>{
 const user=userEvent.setup();render(<Backups desktop/>);await user.click(await screen.findByRole("button",{name:/harmless.bin/}));await user.click(screen.getByRole("button",{name:"Restore to Library"}));const dialog=screen.getByRole("dialog");expect(dialog).toHaveTextContent("harmless_DEFAULT.bin");expect(dialog).toHaveTextContent("does not flash");expect(backend.restoreFirmwareBackup).not.toHaveBeenCalled();await user.click(within(dialog).getByRole("button",{name:"Restore to Library"}));expect(backend.restoreFirmwareBackup).toHaveBeenCalledWith(backup.id);expect(screen.getByRole("button",{name:/harmless.bin/})).toBeInTheDocument();
});
it("names restored copies without losing extensions or duplicating DEFAULT",()=>{expect(defaultName("Huawei_E5577.bin")).toBe("Huawei_E5577_DEFAULT.bin");expect(defaultName("router_DEFAULT.img")).toBe("router_DEFAULT.img");expect(defaultName("router")).toBe("router_DEFAULT");});
it("requires backup-delete confirmation and identifies the preserved copy",async()=>{
 const user=userEvent.setup();render(<Backups desktop/>);await user.click(await screen.findByRole("button",{name:/harmless.bin/}));await user.click(screen.getByRole("button",{name:"Delete backup"}));const dialog=screen.getByRole("dialog");expect(dialog).toHaveTextContent("only preserved original");await user.click(within(dialog).getByRole("button",{name:"Delete backup"}));expect(backend.removeFirmwareItem).toHaveBeenCalledWith(backup.id,true);
});
it("keeps unsaved refresh settings through metadata updates and persists a real change",async()=>{
 const user=userEvent.setup();const value=workspace();const view=render(<WorkspaceContext.Provider value={value}><Preferences snapshot={snapshot} saving={false} onSave={vi.fn()} previewTheme="system" onPreviewTheme={vi.fn()}/></WorkspaceContext.Provider>);await user.click(screen.getByRole("switch",{name:/Automatic device refresh/}));view.rerender(<WorkspaceContext.Provider value={{...value,state:{...desktopState,preferences:{...desktopState.preferences}}}}><Preferences snapshot={snapshot} saving={false} onSave={vi.fn()} previewTheme="system" onPreviewTheme={vi.fn()}/></WorkspaceContext.Provider>);expect(screen.getByRole("switch",{name:/Automatic device refresh/})).not.toBeChecked();await user.click(screen.getByRole("button",{name:"Save workspace preferences"}));expect(backend.saveDesktopPreferences).toHaveBeenCalledWith(expect.objectContaining({autoRefresh:false}));expect(await screen.findByText("Device and notification preferences saved.")).toBeInTheDocument();
});
it("uses native folder selection and a reviewable copy plan before storage changes",async()=>{
 const user=userEvent.setup();vi.mocked(backend.selectLibraryDirectory).mockResolvedValue({ticket:"a".repeat(32),source:desktopState.libraryDirectory,destination:"C:\\test\\new-library",bytes:1024,fileCount:1,expiresAt:Date.now()+300000});render(<WorkspaceContext.Provider value={workspace()}><Preferences snapshot={snapshot} saving={false} onSave={vi.fn()} previewTheme="system" onPreviewTheme={vi.fn()}/></WorkspaceContext.Provider>);await user.click(screen.getByRole("button",{name:/Change folder/}));const dialog=screen.getByRole("dialog");expect(dialog).toHaveTextContent("1 stored files");expect(dialog).toHaveTextContent("restart");expect(backend.relocateFirmwareLibrary).not.toHaveBeenCalled();await user.click(within(dialog).getByRole("button",{name:"Copy and verify Library"}));expect(backend.relocateFirmwareLibrary).toHaveBeenCalledWith("a".repeat(32));
});
it("filters persisted Logs and exposes structured recovery details",async()=>{
 const failure={...job,id:"fail",status:"failed" as const,message:"Copy interrupted",errorCode:"INTERRUPTED",recoveryInfo:"Reselect the preserved original."};const value=workspace({jobs:[{...job,status:"completed",message:"Stored safely"},failure]});const user=userEvent.setup();render(<WorkspaceContext.Provider value={value}><Jobs desktop/></WorkspaceContext.Provider>);await user.selectOptions(screen.getByRole("combobox",{name:"Filter job status"}),"failed");expect(screen.queryByText("Stored safely")).not.toBeInTheDocument();await user.click(screen.getByRole("button",{name:/Copy interrupted/}));expect(screen.getByRole("region",{name:"Event details"})).toHaveTextContent("INTERRUPTED");expect(screen.getByText("Reselect the preserved original.")).toBeInTheDocument();
});
it("does not emit historical-completion notifications on startup",async()=>{
 vi.mocked(backend.listLibraryJobs).mockResolvedValue([{...job,status:"completed",createdAt:1}]);render(<WorkspaceProvider desktop><div>Ready</div></WorkspaceProvider>);await waitFor(()=>expect(backend.listLibraryJobs).toHaveBeenCalled());expect(screen.queryByText(/Firmware import complete/)).not.toBeInTheDocument();
});
