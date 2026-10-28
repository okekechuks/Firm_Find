import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { FirmwareExtraction } from "../components/FirmwareExtraction";
const api = vi.hoisted(() => ({ getFirmwareExtraction: vi.fn(), startFirmwareExtraction: vi.fn(), readExtractedText: vi.fn(), exportExtractedFiles: vi.fn(), cancelLibraryJob: vi.fn() }));
vi.mock("../lib/backend", () => ({ ...api, errorMessage: (e: Error) => e.message }));
// Synthetic React fixtures; no firmware/device evidence.
const hash = "a".repeat(64);
const report = {entries:[{path:"etc",kind:"directory",size:0},{path:"etc/config",kind:"file",size:10}],totalBytes:10,status:"partially_extracted",extractor:"Synthetic",region:{offset:0},limitations:["Synthetic fixture"]};
const job = {id:"synthetic",kind:"firmware_extraction",status:"running" as const,relatedArtifact:hash,stage:"extracting",message:"Extracting",createdAt:1,startedAt:1,finishedAt:null,updatedAt:1,progressBytes:0,totalBytes:1,errorCode:null,importId:null,cancelRequested:false,duplicate:false,recoveryInfo:null};
beforeEach(() => {vi.clearAllMocks();api.getFirmwareExtraction.mockResolvedValue(null);api.startFirmwareExtraction.mockResolvedValue(job);api.cancelLibraryJob.mockResolvedValue({...job,status:"cancelled"});});
it("starts actual backend operation and exposes cancellation", async () => {
  render(<FirmwareExtraction sha256={hash} jobs={[]}/>);
  fireEvent.click(screen.getByRole("button", {name:"Extract Contents"}));
  fireEvent.click(await screen.findByRole("button",{name:"Cancel extraction"}));
  await waitFor(() => expect(api.cancelLibraryJob).toHaveBeenCalledWith("synthetic"));
  expect(await screen.findByRole("button",{name:"Extract Contents"})).toBeEnabled();
});
it("browses folders and never exports files without explicit selection and review", async () => {
  api.getFirmwareExtraction.mockResolvedValue(report);
  api.readExtractedText.mockResolvedValue("<script>not executed</script>");
  api.exportExtractedFiles.mockResolvedValue(true);
  render(<FirmwareExtraction sha256={hash} jobs={[]}/>);
  fireEvent.click(await screen.findByRole("button",{name:"etc/"}));
  const exportButton = screen.getByRole("button",{name:"Export Extracted Files"});expect(exportButton).toBeDisabled();
  fireEvent.click(screen.getByRole("button",{name:"config"}));
  expect(await screen.findByText("<script>not executed</script>")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("checkbox",{name:"Select etc/config for export"}));expect(exportButton).toBeDisabled();
  fireEvent.click(screen.getByRole("checkbox",{name:/I reviewed/}));fireEvent.click(exportButton);
  await waitFor(() => expect(api.exportExtractedFiles).toHaveBeenCalledWith(hash,["etc/config"]));
});
it("reports unavailable workers without inventing a successful tree", async () => {
  api.startFirmwareExtraction.mockRejectedValue(new Error("Worker unavailable"));
  render(<FirmwareExtraction sha256={hash} jobs={[]}/>);
  fireEvent.click(screen.getByRole("button",{name:"Extract Contents"}));
  expect(await screen.findByRole("alert")).toHaveTextContent("Worker unavailable");
  expect(screen.queryByText("Browse Extracted Files")).not.toBeInTheDocument();
});
it("shows actionable dependency failures from persistent jobs",async()=>{
  render(<FirmwareExtraction sha256={hash} jobs={[{...job,status:"failed",errorCode:"WORKER_UNAVAILABLE",message:"Set up WSL Ubuntu with qualified extraction tools."}]}/>);
  expect(await screen.findByRole("alert")).toHaveTextContent("Set up WSL Ubuntu");
  expect(screen.queryByText("Browse Extracted Files")).not.toBeInTheDocument();
});
it("identifies individual unexportable names and keeps selection disabled",async()=>{
  api.getFirmwareExtraction.mockResolvedValue({...report,entries:[{path:"café",kind:"file",size:1,exportError:"Cannot export this Unicode filename as USTAR."}]});
  render(<FirmwareExtraction sha256={hash} jobs={[]}/>);
  expect(await screen.findByRole("checkbox",{name:"Select café for export"})).toBeDisabled();
  expect(screen.getByText("Cannot export this Unicode filename as USTAR.")).toBeVisible();
  expect(screen.getByRole("button",{name:"Export Extracted Files"})).toBeDisabled();
});
it("shows layer coverage and unknown ranges without calling a delta a filesystem",async()=>{
  api.getFirmwareExtraction.mockResolvedValue({...report,version:2,status:"partially_extracted",completeness:"partial",
    layers:[{id:"layer-0000",parentId:null,format:"cpio_crc",handler:"cpio",handlerVersion:1,offset:0,length:1024,space:"source",completeness:"partial",reasons:["delta, base required"],sourcePath:"<root>"}],
    unexplainedRegions:[{layerId:"layer-0000",offset:900,length:124,reason:"unknown trailer"}],
    entropyMap:[{offset:0,length:1024,bitsPerByte:7.9,classification:"high_entropy_compressed_or_encrypted"}],
    omittedMembers:[],provenance:{engineVersion:"2.0.0",tools:[]}});
  render(<FirmwareExtraction sha256={hash} jobs={[]}/>);
  expect(await screen.findByText(/Partial extraction/)).toBeInTheDocument();
  fireEvent.click(screen.getByText("Layer tree"));
  expect(screen.getByText("cpio_crc")).toBeVisible();
  expect(screen.getByText("delta, base required")).toBeVisible();
  fireEvent.click(screen.getByText("Unexplained regions (1)"));
  expect(screen.getByText(/unknown trailer/)).toBeVisible();
  expect(screen.queryByText("Filesystem extracted and verified")).not.toBeInTheDocument();
});
it("unknown metadata-only inspection never shows export controls or extracted success",async()=>{
  api.getFirmwareExtraction.mockResolvedValue({...report,version:2,entries:[],totalBytes:0,status:"encrypted-or-unknown",completeness:"encrypted-or-unknown",layers:[],unexplainedRegions:[],entropyMap:[],omittedMembers:[],provenance:{engineVersion:"2.0.0",tools:[]}});
  render(<FirmwareExtraction sha256={hash} jobs={[]}/>);
  expect(await screen.findByText(/Encrypted or unknown/)).toBeInTheDocument();
  expect(screen.queryByRole("button",{name:"Export Extracted Files"})).not.toBeInTheDocument();
  expect(screen.queryByText("Browse Extracted Files")).not.toBeInTheDocument();
});
