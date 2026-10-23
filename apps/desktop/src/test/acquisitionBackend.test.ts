import { beforeEach, expect, it, vi } from "vitest";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { approveAuthChallenge, readAuthChallenge, approveStaticMenu, readStaticMenu, discoverFirmwareRoutes } from "../lib/backend";
import { acquisitionReportSchema, compatibilityStageIds } from "../types/acquisition";
vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn(),isTauri:vi.fn()}));
const id="a".repeat(64);
const report=()=>({schemaVersion:1,deviceId:id,discoveryRevision:3,evaluatedAt:Date.now(),expiresAt:Date.now()+60000,fingerprint:{deviceId:id,windowsName:"USB Composite Device",transports:["usb","network"],usbVid:"1286",usbPid:"4E31",interfaceClasses:["Net"],endpoint:null,interfaceId:null,targetKind:"usb_or_network_device"},identity:{confidence:"unverified",verifiedManufacturer:null,verifiedFirmwareFamily:null,verifiedModel:null,verifiedHardwareRevision:null,verifiedFirmware:null,reportedModel:null,evidence:[]},protocols:[],candidates:[],capabilities:{},probe:{attempted:false,code:"not_approved",httpStatus:null,metadataTruncated:false},stages:compatibilityStageIds.map(id=>({id,state:id==="detected"?"observed":"not_implemented",reason:"No implementation"})),canObtain:false});
beforeEach(()=>{vi.resetAllMocks();vi.mocked(isTauri).mockReturnValue(true);});
it("uses separate challenge commands and rejects missing or cross-device results",async()=>{
  const input={deviceId:id,endpoint:"http://192.168.100.1/",interfaceId:"synthetic",approvalToken:null as string|null,advancedEndpoint:false};
  const authChallenge={code:"observed",httpStatus:401,realm:"Synthetic",qop:"auth",algorithm:null,noncePresent:true,setCookiePresent:false,otherSchemesPresent:false,source:"endpoint_response",reliability:"observed",verification:"observed",label:"authentication challenge observed; unverified"};
  vi.mocked(invoke).mockResolvedValueOnce("e".repeat(32)).mockResolvedValueOnce({...report(),authChallenge});input.approvalToken=await approveAuthChallenge(input);await readAuthChallenge(input);
  expect(invoke).toHaveBeenNthCalledWith(1,"approve_auth_challenge",{input});expect(invoke).toHaveBeenNthCalledWith(2,"read_auth_challenge",{input});
  vi.mocked(invoke).mockResolvedValueOnce({...report(),authChallenge,deviceId:"b".repeat(64)});await expect(readAuthChallenge(input)).rejects.toThrow(/another target/);
  vi.mocked(invoke).mockResolvedValueOnce(report());await expect(readAuthChallenge(input)).rejects.toThrow(/missing/);
});
it("accepts bounded challenge observations but never nonce values or authority",()=>{
  const authChallenge={code:"observed",httpStatus:401,realm:"Synthetic realm",qop:"auth",algorithm:null,noncePresent:true,setCookiePresent:false,otherSchemesPresent:false,source:"endpoint_response",reliability:"observed",verification:"observed",label:"authentication challenge observed; unverified"};
  expect(acquisitionReportSchema.safeParse({...report(),authChallenge}).success).toBe(true);
  for(const change of [{nonce:"private_nonce"},{realm:"x".repeat(65)},{qop:"x".repeat(33)},{algorithm:"x".repeat(17)},{verification:"verified"},{code:"timeout"},{httpStatus:200},{otherSchemesPresent:true}])expect(acquisitionReportSchema.safeParse({...report(),authChallenge:{...authChallenge,...change}}).success).toBe(false);
});
it("accepts only bounded sanitized media types as static-menu limitations",()=>{
  const staticMenu={code:"wrong_content_type",fileName:null,observedMediaType:"text/plain",source:"endpoint_response",reliability:"observed",verification:"observed",label:"dashboard file name observed from the static menu; unverified"};
  expect(acquisitionReportSchema.safeParse({...report(),staticMenu}).success).toBe(true);
  for(const value of ["unavailable","text/html","application/problem+xml"])expect(acquisitionReportSchema.safeParse({...report(),staticMenu:{...staticMenu,observedMediaType:value}}).success).toBe(true);
  for(const value of ["x".repeat(65),"text/html; secret=x","text/html@private", "Text/HTML"])expect(acquisitionReportSchema.safeParse({...report(),staticMenu:{...staticMenu,observedMediaType:value}}).success).toBe(false);
  expect(acquisitionReportSchema.safeParse({...report(),staticMenu:{...staticMenu,code:"timeout"}}).success).toBe(false);
});
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

it("accepts only observed static-menu evidence without granting another stage",()=>{
  const staticMenu={code:"observed",fileName:"synthetic_dashboard",source:"endpoint_response",reliability:"observed",verification:"observed",label:"dashboard file name observed from the static menu; unverified"};
  expect(acquisitionReportSchema.safeParse({...report(),staticMenu}).success).toBe(true);
  expect(acquisitionReportSchema.safeParse({...report(),staticMenu:{...staticMenu,verification:"verified"}}).success).toBe(false);
  expect(acquisitionReportSchema.safeParse({...report(),staticMenu:{...staticMenu,fileName:"../login.cgi"}}).success).toBe(false);
});

it("uses separate menu IPC with a returned receipt and rejects another target",async()=>{
  const input={deviceId:id,endpoint:"http://192.168.100.1/",interfaceId:"synthetic",approvalToken:null as string|null,advancedEndpoint:false};
  vi.mocked(invoke).mockResolvedValueOnce("f".repeat(32)).mockResolvedValueOnce({...report(),staticMenu:null});
  input.approvalToken=await approveStaticMenu(input);await readStaticMenu(input);
  expect(invoke).toHaveBeenNthCalledWith(1,"approve_static_menu",{input});expect(invoke).toHaveBeenNthCalledWith(2,"read_static_menu",{input});
  vi.mocked(invoke).mockResolvedValueOnce({...report(),deviceId:"b".repeat(64)});
  await expect(readStaticMenu(input)).rejects.toThrow(/another target/);
});
