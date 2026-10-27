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
