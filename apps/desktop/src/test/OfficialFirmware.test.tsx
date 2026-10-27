import { fireEvent,render,screen,waitFor } from "@testing-library/react";
import { beforeEach,describe,expect,it,vi } from "vitest";
import { OfficialImageDialog } from "../components/OfficialImageDialog";
import { FirmwareAnalysisDialog } from "../components/FirmwareAnalysisDialog";
import { analysisReportSchema,officialSourceSchema } from "../types/official";
import type { JobRecord } from "../types/library";
const api=vi.hoisted(()=>({listOfficialSources:vi.fn(),startOfficialDownload:vi.fn(),cancelLibraryJob:vi.fn(),getFirmwareAnalysis:vi.fn(),getOfficialProvenance:vi.fn(),startFirmwareAnalysis:vi.fn(),getFirmwareExtraction:vi.fn()}));
vi.mock("../lib/backend",()=>({...api,errorMessage:(e:unknown)=>e instanceof Error?e.message:"Request failed"}));
// Synthetic UI fixtures only; no firmware, device identity or download is represented.
const hash="a".repeat(64);
const source={id:"synthetic-source",vendor:"Synthetic",model:"Test board",revision:"r1",profile:"synthetic",target:"test/test",version:"1",filename:"synthetic-sysupgrade.bin",sha256:hash,sizeBytes:1024,url:"https://downloads.openwrt.org/synthetic.bin",manifestUrl:"https://downloads.openwrt.org/sha256sums",manifestSha256:hash,profilesSha256:hash,signingFingerprint:"synthetic",primaryFingerprint:"synthetic",verification:"pinned_catalogue_from_verified_release_signature" as const,installability:"unknown" as const};
const job:JobRecord={id:"b".repeat(32),kind:"official_image_download",status:"pending",createdAt:1,startedAt:null,finishedAt:null,updatedAt:1,progressBytes:0,totalBytes:1024,stage:"queued",errorCode:null,message:"Queued",relatedArtifact:hash,importId:null,cancelRequested:false,duplicate:false,recoveryInfo:null};
const report={reportVersion:1 as const,inputSha256:hash,inputBytes:1024,analyzedAt:100,scannedBytes:1024,scanTruncated:false,format:"Synthetic format",regions:[{offset:0,format:"Synthetic format"}],sampledEntropy:1,entropySampleBytes:1024,recursiveExtraction:false as const,executableContentsRun:false as const,compatibility:"unknown" as const,limitations:[]};
beforeEach(()=>{vi.clearAllMocks();api.listOfficialSources.mockResolvedValue([source]);api.startOfficialDownload.mockResolvedValue(job);api.getFirmwareAnalysis.mockResolvedValue(report);api.getFirmwareExtraction.mockResolvedValue(null);api.getOfficialProvenance.mockResolvedValue([]);api.startFirmwareAnalysis.mockResolvedValue({...job,kind:"firmware_analysis"});});
describe("official source workflow",()=>{
  it("requires explicit exact-target confirmation and exposes the completed Library action",async()=>{
    const open=vi.fn();const close=vi.fn();const view=render(<OfficialImageDialog jobs={[]} onOpen={open} onClose={close}/>);
    await screen.findByRole("combobox",{name:"Official target"});
    const download=screen.getByRole("button",{name:"Download to Library"});expect(download).toBeDisabled();
    fireEvent.click(screen.getByRole("checkbox"));fireEvent.click(download);
    await waitFor(()=>expect(api.startOfficialDownload).toHaveBeenCalledWith("synthetic-source",true));
    view.rerender(<OfficialImageDialog jobs={[{...job,status:"completed",updatedAt:2,progressBytes:1024}]} onOpen={open} onClose={close}/>);
    fireEvent.click(await screen.findByRole("button",{name:"Open in Library"}));expect(open).toHaveBeenCalledWith(hash);
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
  });
  it("shows backend failures and keeps download approval explicit",async()=>{
    api.startOfficialDownload.mockRejectedValue(new Error("Source unavailable"));render(<OfficialImageDialog jobs={[]} onOpen={()=>{}} onClose={()=>{}}/>);
    await screen.findByRole("combobox");fireEvent.click(screen.getByRole("checkbox"));fireEvent.click(screen.getByRole("button",{name:"Download to Library"}));
    expect(await screen.findByRole("alert")).toHaveTextContent("Source unavailable");
  });
});
it("renders a persisted bounded report with a separate extraction action",async()=>{
  render(<FirmwareAnalysisDialog artifact={{sha256:hash,sizeBytes:1024,storageStatus:"present",integrityStatus:"verified",firstImportedAt:1,lastVerifiedAt:1,format:"UNKNOWN",sourceCount:1,originalFilename:"synthetic.bin",sourceCategory:"user_import",artifactClass:"user_imported_unverified",associatedDeviceId:null}} jobs={[]} onClose={()=>{}}/>);
  expect(await screen.findByText("Synthetic format")).toBeInTheDocument();expect(screen.getByText("Hardware compatibility: unverified")).toBeInTheDocument();
  expect(screen.queryByRole("button",{name:/flash|install/i})).not.toBeInTheDocument();
});
it("rejects wire claims of execution, extraction or automatic installation support",()=>{
  expect(analysisReportSchema.safeParse({...report,recursiveExtraction:true}).success).toBe(false);
  expect(analysisReportSchema.safeParse({...report,executableContentsRun:true}).success).toBe(false);
  expect(analysisReportSchema.safeParse({...report,regions:Array(65).fill({offset:0,format:"synthetic"})}).success).toBe(false);
  expect(officialSourceSchema.safeParse({...source,installability:"verified"}).success).toBe(false);
});
