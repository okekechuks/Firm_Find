import { z } from "zod";
import { evidenceSchema } from "./profiles";
export const acquisitionStateSchema = z.enum(["not_checked", "observed", "not_implemented", "unknown", "candidate", "requires_authentication", "requires_approval", "requires_external_tool", "available_verified", "unsupported", "failed", "completed_verified"]);
export const compatibilityStageIds = ["detected","exact_identity","management_api","official_image","firmware_export","physical_read","firmware_acquired","filesystem_extracted","customization","rebuild","installation"] as const;
export const compatibilityStageStateSchema = z.enum(["verified","observed","unknown","not_checked","not_supported","not_implemented"]);
const compatibilityStageSchema = z.object({id:z.enum(compatibilityStageIds),state:compatibilityStageStateSchema,reason:z.string().max(2048)}).strict();
const capability = z.object({state:acquisitionStateSchema,reason:z.string().max(2048),evidence:z.array(z.string()),missingRequirements:z.array(z.string()),executable:z.literal(false)}).strict();
const challengeSchema=z.object({code:z.enum(["observed","challenge_not_usable","no_recognized_family","unexpected_status","redirect_blocked","timeout","cancelled","subnet_conflict","transport_failed"]),httpStatus:z.number().int().min(100).max(999).nullable(),realm:z.string().regex(/^[\x20-\x7e]{1,64}$/).nullable(),qop:z.string().max(32).regex(/^[A-Za-z0-9_-]+(?:, *[A-Za-z0-9_-]+)*$/).nullable(),algorithm:z.string().max(16).regex(/^[A-Za-z0-9_-]+$/).nullable(),noncePresent:z.boolean(),setCookiePresent:z.boolean(),otherSchemesPresent:z.boolean(),source:z.literal("endpoint_response"),reliability:z.literal("observed"),verification:z.literal("observed"),label:z.literal("authentication challenge observed; unverified")}).strict().superRefine((r,ctx)=>{
  if(r.code==="observed" ? r.httpStatus!==401||!r.realm||!r.qop||!r.noncePresent||r.otherSchemesPresent : r.realm!==null||r.qop!==null||r.algorithm!==null)ctx.addIssue({code:z.ZodIssueCode.custom,message:"Only a valid 401 Digest challenge may contain sanitized parameters."});
});
export const acquisitionReportSchema = z.object({
  authChallenge:challengeSchema.nullable().optional().default(null),
  staticMenu:z.object({code:z.enum(["observed","menu_not_readable_without_login","forbidden","redirect_blocked","wrong_content_type","truncated_metadata","metadata_too_large","no_unique_dashboard_entry","cancelled","timeout","subnet_conflict","transport_failed","no_recognized_family"]),observedMediaType:z.string().regex(/^[a-z0-9.+/-]{1,64}$/).nullable().optional().default(null),fileName:z.string().regex(/^[A-Za-z0-9_-]{1,64}$/).nullable(),source:z.literal("endpoint_response"),reliability:z.literal("observed"),verification:z.literal("observed"),label:z.literal("dashboard file name observed from the static menu; unverified")}).strict().nullable().optional().default(null),
  schemaVersion:z.literal(1),deviceId:z.string().regex(/^[a-f0-9]{64}$/),discoveryRevision:z.number().int(),evaluatedAt:z.number(),expiresAt:z.number(),
  fingerprint:z.object({deviceId:z.string(),windowsName:z.string(),transports:z.array(z.string()),usbVid:z.string().nullable(),usbPid:z.string().nullable(),interfaceClasses:z.array(z.string()),endpoint:z.string().nullable(),interfaceId:z.string().nullable(),targetKind:z.enum(["local_endpoint","usb_or_network_device"])}).strict(),
  identity:z.object({confidence:z.enum(["conflicting","verified","unverified"]),verifiedManufacturer:z.string().nullable(),verifiedFirmwareFamily:z.string().nullable(),verifiedModel:z.string().nullable(),verifiedHardwareRevision:z.string().nullable(),verifiedFirmware:z.string().nullable(),reportedModel:z.string().nullable(),evidence:z.array(evidenceSchema).max(256)}).strict(),
  protocols:z.array(z.object({family:z.string(),confidence:z.literal("candidate"),evidence:z.array(z.string()),limitations:z.string()}).strict()).max(8),
  candidates:z.array(z.object({strategy:z.enum(["official_image","documented_export","authorized_storage_read","manufacturer_package","external_laboratory"]),name:z.string(),capability,provenanceKind:z.string()}).strict()).max(16),
  capabilities:z.record(capability),stages:z.array(compatibilityStageSchema).length(11),probe:z.object({attempted:z.boolean(),code:z.string(),httpStatus:z.number().int().nullable(),metadataTruncated:z.boolean(),advancedEndpoint:z.boolean().default(false),message:z.string().default("")}).strict(),canObtain:z.literal(false),
}).strict().superRefine((report,ctx)=>{
  if(report.staticMenu && ((report.staticMenu.code==="observed") !== (report.staticMenu.fileName!==null)))ctx.addIssue({code:z.ZodIssueCode.custom,path:["staticMenu"],message:"Only an observed menu result may include a filename."});
  if(report.staticMenu?.observedMediaType && report.staticMenu.code!=="wrong_content_type")ctx.addIssue({code:z.ZodIssueCode.custom,path:["staticMenu"],message:"Media type is only a rejected-menu limitation."});
  if(report.stages.map(stage=>stage.id).some((id,index)=>id!==compatibilityStageIds[index]))ctx.addIssue({code:z.ZodIssueCode.custom,path:["stages"],message:"Compatibility stages must keep their fixed independent order."});
});
export type AcquisitionReport = z.infer<typeof acquisitionReportSchema>;
export type CompatibilityStageId = typeof compatibilityStageIds[number];
export type CompatibilityStage = z.infer<typeof compatibilityStageSchema>;
export type AcquisitionRequest = {deviceId:string;endpoint:string|null;interfaceId:string|null;approvalToken:string|null;advancedEndpoint:boolean};
