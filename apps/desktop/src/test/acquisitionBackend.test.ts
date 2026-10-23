import { beforeEach, expect, it, vi } from "vitest";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { discoverFirmwareRoutes } from "../lib/backend";
import { acquisitionReportSchema, compatibilityStageIds } from "../types/acquisition";
vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn(),isTauri:vi.fn()}));
const id="a".repeat(64);
const report=()=>({schemaVersion:1,deviceId:id,discoveryRevision:3,evaluatedAt:Date.now(),expiresAt:Date.now()+60000,fingerprint:{deviceId:id,windowsName:"USB Composite Device",transports:["usb","network"],usbVid:"1286",usbPid:"4E31",interfaceClasses:["Net"],endpoint:null,interfaceId:null,targetKind:"usb_or_network_device"},identity:{confidence:"unverified",verifiedManufacturer:null,verifiedFirmwareFamily:null,verifiedModel:null,verifiedHardwareRevision:null,verifiedFirmware:null,reportedModel:null,evidence:[]},protocols:[],candidates:[],capabilities:{},probe:{attempted:false,code:"not_approved",httpStatus:null,metadataTruncated:false},stages:compatibilityStageIds.map(id=>({id,state:id==="detected"?"observed":"not_implemented",reason:"No implementation"})),canObtain:false});
beforeEach(()=>{vi.resetAllMocks();vi.mocked(isTauri).mockReturnValue(true);});
it("uses the route-discovery IPC contract without creating implicit endpoint approval",async()=>{
  vi.mocked(invoke).mockResolvedValue(report());
  await discoverFirmwareRoutes({deviceId:id,endpoint:null,interfaceId:null,approvalToken:null,advancedEndpoint:false});
  expect(invoke).toHaveBeenCalledWith("discover_firmware_routes",{input:{deviceId:id,endpoint:null,interfaceId:null,approvalToken:null,advancedEndpoint:false}});
});
it("rejects cross-device responses and fabricated acquisition enablement",async()=>{
  vi.mocked(invoke).mockResolvedValueOnce({...report(),deviceId:"b".repeat(64)});
  await expect(discoverFirmwareRoutes({deviceId:id,endpoint:null,interfaceId:null,approvalToken:null,advancedEndpoint:false})).rejects.toThrow(/another target/);
  expect(acquisitionReportSchema.safeParse({...report(),canObtain:true}).success).toBe(false);
  expect(acquisitionReportSchema.safeParse({...report(),stages:report().stages.map((stage,index)=>index===1?{...stage,state:"verified",executable:true}:stage)}).success).toBe(false);
  expect(acquisitionReportSchema.safeParse({...report(),capabilities:{physicalRead:{state:"available_verified",reason:"claimed",evidence:[],missingRequirements:[],executable:true}}}).success).toBe(false);
});
it("rejects browser preview hardware calls",async()=>{
  vi.mocked(isTauri).mockReturnValue(false);
  await expect(discoverFirmwareRoutes({deviceId:id,endpoint:null,interfaceId:null,approvalToken:"a".repeat(32),advancedEndpoint:false})).rejects.toThrow(/Windows app/);
  expect(invoke).not.toHaveBeenCalled();
});
