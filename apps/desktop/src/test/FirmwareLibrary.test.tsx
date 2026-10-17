import {act,render,screen,waitFor,within} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {beforeEach,expect,it,vi} from "vitest";
import {FirmwareLibrary} from "../features/FirmwareLibrary";
import {Jobs} from "../features/Jobs";
import * as backend from "../lib/backend";
import {artifact,details,empty,job,selection} from "./libraryFixtures";
vi.mock("../lib/useLibraryFeed",()=>({useLibraryFeed:()=>0}));
vi.mock("../lib/backend",async(original)=>({...await original<typeof import("../lib/backend")>(),listFirmwareArtifacts:vi.fn(),listLibraryJobs:vi.fn(),getLibraryActivity:vi.fn(),selectFirmwareFile:vi.fn(),discardFirmwareSelection:vi.fn(),startFirmwareImport:vi.fn(),cancelLibraryJob:vi.fn(),getFirmwareArtifact:vi.fn(),verifyFirmwareIntegrity:vi.fn(),exportArtifactManifest:vi.fn()}));
beforeEach(()=>{
 vi.mocked(backend.listFirmwareArtifacts).mockResolvedValue(empty);vi.mocked(backend.listLibraryJobs).mockResolvedValue([]);vi.mocked(backend.getLibraryActivity).mockResolvedValue([]);
 vi.mocked(backend.selectFirmwareFile).mockResolvedValue(selection);vi.mocked(backend.discardFirmwareSelection).mockResolvedValue();vi.mocked(backend.getFirmwareArtifact).mockResolvedValue(details);
 vi.mocked(backend.startFirmwareImport).mockResolvedValue(job);vi.mocked(backend.verifyFirmwareIntegrity).mockResolvedValue({...job,kind:"integrity_verify"});vi.mocked(backend.exportArtifactManifest).mockResolvedValue(true);
});
it("shows a real empty library and native file selection confirmation",async()=>{
 const user=userEvent.setup();render(<FirmwareLibrary desktop/>);await screen.findByText("No firmware files imported");expect(screen.queryByText(artifact.sha256)).not.toBeInTheDocument();
 await user.click(screen.getByRole("button",{name:"Import Firmware"}));const dialog=await screen.findByRole("dialog");expect(dialog).toHaveTextContent("harmless.bin");expect(dialog).toHaveTextContent("1,024 bytes");expect(backend.startFirmwareImport).not.toHaveBeenCalled();
 await user.click(within(dialog).getByRole("button",{name:"Cancel selection"}));await waitFor(()=>expect(backend.discardFirmwareSelection).toHaveBeenCalledWith(selection.ticket));
});
it("requires confirmation and displays persisted progress and cancellation",async()=>{
 const user=userEvent.setup();render(<FirmwareLibrary desktop/>);await user.click(screen.getByRole("button",{name:"Import Firmware"}));await screen.findByRole("dialog");
 vi.mocked(backend.listLibraryJobs).mockResolvedValue([job]);await user.click(screen.getByRole("button",{name:"Confirm import"}));
 expect(backend.startFirmwareImport).toHaveBeenCalledWith({ticket:selection.ticket,description:null,associatedDeviceId:null});expect(await screen.findByRole("progressbar")).toHaveAttribute("value","512");expect(screen.getByText(/512 \/ 1,024 bytes/)).toBeInTheDocument();await user.click(screen.getByRole("button",{name:"Cancel job"}));expect(backend.cancelLibraryJob).toHaveBeenCalledWith(job.id);
});
it("renders backend success, duplicate occurrence and persisted artifact details",async()=>{
 const user=userEvent.setup();vi.mocked(backend.listLibraryJobs).mockResolvedValue([{...job,status:"completed",duplicate:true,progressBytes:1024,relatedArtifact:artifact.sha256,message:"Duplicate stored safely."}]);vi.mocked(backend.listFirmwareArtifacts).mockResolvedValue({...empty,total:1,uniqueBytes:1024,artifacts:[artifact]});
 render(<FirmwareLibrary desktop/>);await user.click(await screen.findByRole("button",{name:/harmless.bin/}));const region=await screen.findByRole("region",{name:"Artifact details"});await waitFor(()=>expect(within(region).getByText(/SHA-256:/)).toHaveTextContent(artifact.sha256));expect(region).toHaveTextContent("unverified");
 await user.click(screen.getByRole("button",{name:"Import Firmware"}));await screen.findByRole("dialog");await user.click(screen.getByRole("button",{name:"Confirm import"}));expect(await screen.findByText(/Duplicate recognized/)).toBeInTheDocument();
});
it("shows explicit backend errors and does not invent successful import entries",async()=>{
 vi.mocked(backend.selectFirmwareFile).mockRejectedValue({code:"ACCESS_DENIED",message:"Windows denied source access."});render(<FirmwareLibrary desktop/>);await userEvent.click(screen.getByRole("button",{name:"Import Firmware"}));expect(await screen.findByRole("alert")).toHaveTextContent("Windows denied source access.");expect(backend.startFirmwareImport).not.toHaveBeenCalled();
});
it("verifies integrity and exports a manifest through backend operations",async()=>{
 vi.mocked(backend.listFirmwareArtifacts).mockResolvedValue({...empty,total:1,artifacts:[artifact]});const user=userEvent.setup();render(<FirmwareLibrary desktop/>);await user.click(await screen.findByRole("button",{name:/harmless.bin/}));await screen.findByRole("button",{name:"Verify Integrity"});await user.click(screen.getByRole("button",{name:"Verify Integrity"}));expect(backend.verifyFirmwareIntegrity).toHaveBeenCalledWith(artifact.sha256);
 await user.click(screen.getByRole("button",{name:"Export Manifest"}));expect(backend.exportArtifactManifest).toHaveBeenCalledWith(artifact.sha256);expect(await screen.findByText(/Manifest saved/)).toBeInTheDocument();
});
it("sends filename/hash, provenance and integrity filters to bounded backend queries",async()=>{
 const user=userEvent.setup();render(<FirmwareLibrary desktop/>);await screen.findByText("No firmware files imported");await user.type(screen.getByRole("textbox",{name:"Search filename or SHA-256"}),"device");await user.selectOptions(screen.getByRole("combobox",{name:"Integrity"}),"missing");await user.selectOptions(screen.getByRole("combobox",{name:"Source"}),"user_import");
 await waitFor(()=>expect(backend.listFirmwareArtifacts).toHaveBeenLastCalledWith(expect.objectContaining({search:"device",integrity:"missing",source:"user_import",page:0})));
});
it("rejects a late library response after the search changed",async()=>{
 let resolve!:(value:typeof empty)=>void;vi.mocked(backend.listFirmwareArtifacts).mockReturnValueOnce(new Promise(r=>{resolve=r;}));render(<FirmwareLibrary desktop/>);await waitFor(()=>expect(backend.listFirmwareArtifacts).toHaveBeenCalledOnce());await userEvent.type(screen.getByRole("textbox",{name:"Search filename or SHA-256"}),"new");await screen.findByText("No matching artifacts");await act(async()=>resolve({...empty,total:1,artifacts:[artifact]}));expect(screen.queryByRole("button",{name:/harmless.bin/})).not.toBeInTheDocument();
});
it("shows persisted failed/interrupted jobs and structured activity",async()=>{
 vi.mocked(backend.listLibraryJobs).mockResolvedValue([{...job,status:"failed",errorCode:"INTERRUPTED",message:"Interrupted import retained for recovery."}]);vi.mocked(backend.getLibraryActivity).mockResolvedValue([{id:1,kind:"library_failed",message:"Import interrupted at shutdown.",createdAt:new Date().toISOString(),jobId:job.id,artifactSha256:null,severity:"warning"}]);render(<Jobs desktop/>);expect(await screen.findByRole("alert")).toHaveTextContent("Interrupted import");expect(screen.getByText("Import interrupted at shutdown.")).toBeInTheDocument();expect(screen.getByText("INTERRUPTED")).toBeInTheDocument();
});
it("browser preview performs no library IPC and disables import",()=>{render(<FirmwareLibrary desktop={false}/>);expect(screen.getByRole("button",{name:"Import Firmware"})).toBeDisabled();expect(backend.listFirmwareArtifacts).not.toHaveBeenCalled();});
