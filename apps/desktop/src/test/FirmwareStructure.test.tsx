import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { FirmwareStructure } from "../components/FirmwareStructure";
import { firmwareStructureSchema } from "../types/firmware-structure";

const api=vi.hoisted(()=>({getFirmwareStructure:vi.fn()}));
vi.mock("../lib/backend",()=>({...api,errorMessage:(error:Error)=>error.message}));
const hash="a".repeat(64);
// Synthetic immutable-report projection. No physical device evidence.
function fixture(){return {version:1,inputSha256:hash,inputBytes:2048,bundleSha256:"b".repeat(64),sourceReportVersion:2,sourceStatus:"partially_extracted",fileCount:1,totalExtractedBytes:4,
  provenance:{kind:"library_image_analysis_not_physical_acquisition",extractor:"Synthetic bounded engine",toolSha256:"c".repeat(64),engineVersion:"2.0.0",engineSha256:"c".repeat(64),tools:[]},
  regions:[{id:"layer-0000",parentId:null,children:["layer-0001","unknown-0000"],format:"gzip",kind:"compression",coordinateSpace:"source",relativeOffset:0,length:2048,sourceAbsoluteOffset:0,sha256:hash,confidence:"validated",states:["detected","bounded","validated","extracted"],extractor:"compression",extractorVersion:1,reasons:[],heuristics:[]},
    {id:"layer-0001",parentId:"layer-0000",children:[],format:"squashfs",kind:"filesystem",coordinateSpace:"expanded",relativeOffset:0,length:4096,sourceAbsoluteOffset:null,sha256:"d".repeat(64),confidence:"validated",states:["detected","bounded","validated","extracted"],extractor:"squashfs",extractorVersion:1,reasons:[],heuristics:[{kind:"rootfs_candidate",reason:"Filesystem signature only; rootfs role unverified.",verified:false}]},
    {id:"unknown-0000",parentId:"layer-0000",children:[],format:"unknown",kind:"unknown",coordinateSpace:"source",relativeOffset:1900,length:148,sourceAbsoluteOffset:1900,sha256:null,confidence:"unknown",states:["bounded","unknown"],extractor:null,extractorVersion:null,reasons:["Unknown trailer remains visible."],heuristics:[]}],
  entropyObservations:[],packageMetadata:[],readiness:Object.fromEntries(["extracted","modifiable","repackable","signatureCompatible","installable"].map(key=>[key,{state:key==="extracted"?"observed":"unknown",executable:false,reason:key==="extracted"?"One ordinary file retained.":"No qualified build, signature or installation adapter."}])),limitations:["No claim of physical firmware acquisition."]};}
beforeEach(()=>{vi.clearAllMocks();api.getFirmwareStructure.mockResolvedValue(fixture());});

it("shows structural tree and exact unknown ranges without inventing expanded physical offsets",async()=>{
  render(<FirmwareStructure sha256={hash}/>);
  fireEvent.click(screen.getByText("Artifact Structure"));
  expect(await screen.findByText("Unknown trailer remains visible.")).toBeInTheDocument();
  expect(screen.getByText(/Source bytes 1,900–2,048/)).toBeInTheDocument();
  expect(screen.getByText(/Source offset unavailable.*expanded or member/)).toBeInTheDocument();
  expect(screen.getByText(/rootfs role unverified/)).toBeInTheDocument();
  expect(screen.getByText(/Hash unavailable in immutable report/)).toBeInTheDocument();
});
it("shows independent extraction and rebuild readiness without enabling installation",async()=>{
  render(<FirmwareStructure sha256={hash}/>);fireEvent.click(screen.getByText("Artifact Structure"));
  const status=await screen.findByRole("region",{name:"Rebuild readiness"});
  expect(within(status).getByText("EXTRACTED: observed")).toBeInTheDocument();
  for(const label of ["MODIFIABLE","REPACKABLE","SIGNATURE COMPATIBLE","INSTALLABLE"])expect(within(status).getByText(`${label}: unknown`)).toBeInTheDocument();
  expect(screen.queryByRole("button",{name:/install|rebuild|modify/i})).not.toBeInTheDocument();
  expect(screen.getByText("No claim of physical firmware acquisition.")).toBeInTheDocument();
});
it("loads structural evidence on demand and preserves honest errors and retry",async()=>{
  api.getFirmwareStructure.mockRejectedValueOnce(new Error("Stored evidence is unavailable."));
  render(<FirmwareStructure sha256={hash}/>);expect(api.getFirmwareStructure).not.toHaveBeenCalled();fireEvent.click(screen.getByText("Artifact Structure"));
  expect(await screen.findByRole("alert")).toHaveTextContent("Stored evidence is unavailable.");
  expect(screen.queryByRole("tree")).not.toBeInTheDocument();fireEvent.click(screen.getByRole("button",{name:"Refresh structure"}));
  await waitFor(()=>expect(api.getFirmwareStructure).toHaveBeenCalledTimes(2));expect(await screen.findByRole("tree")).toBeInTheDocument();
});
it("rejects fabricated physical offsets and rebuild authority in native structure responses",()=>{
  expect(firmwareStructureSchema.safeParse(fixture()).success).toBe(true);
  const offset=fixture();offset.regions[1].sourceAbsoluteOffset=128;
  expect(firmwareStructureSchema.safeParse(offset).success).toBe(false);
  const authority=fixture();authority.readiness.installable.executable=true;
  expect(firmwareStructureSchema.safeParse(authority).success).toBe(false);
});
