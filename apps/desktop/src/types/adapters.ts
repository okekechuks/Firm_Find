import { z } from "zod";
import { assessmentSchema, readinessSchema } from "./profiles";

const text = z.string().min(1).max(1024);
const id = z.string().regex(/^[a-z0-9][a-z0-9._-]{0,95}$/);
export const adapterOperationSchema = z.enum([
  "discovery","capability_evaluation","authorized_identity_probe","read_configuration","backup_configuration",
  "modify_configuration","firmware_acquisition","firmware_build","firmware_installation","ussd_session",
  "inspect_capabilities","read_dns","read_apn","read_wifi_limit","read_ussd_capability","verify_configuration","restore_configuration",
]);
export const authenticationMethodSchema = z.enum(["none","password","ssh_key","challenge_response","session_token","documented_vendor"]);
export const adapterDescriptorSchema = z.object({
  id,name:text,version:z.string().regex(/^\d+\.\d+\.\d+$/),
  implementationStatus:z.enum(["built_in","declared","unavailable","test_only","simulator"]),
  transports:z.array(z.enum(["usb","serial","network"])).min(1).max(3),authentication:authenticationMethodSchema,
  operations:z.array(adapterOperationSchema).max(20),dependencies:z.array(text).max(32),limitations:z.array(text).max(32),
  profileIds:z.array(id).max(64),firmwareFamilies:z.array(text).max(64),
}).strict().refine(d=>!["test_only","simulator"].includes(d.implementationStatus),"Test or simulator adapter cannot appear in physical-device IPC.");
export const adapterCapabilitySchema=z.object({
  operation:adapterOperationSchema,exists:z.boolean(),canRead:z.boolean(),canPlan:z.literal(false),canApply:z.literal(false),
  canVerify:z.boolean(),canRestore:z.literal(false),authenticationRequired:z.boolean(),reason:text,
}).strict();
const feature=z.object({assessment:assessmentSchema,readiness:z.array(readinessSchema),canRead:z.literal(false),
  canWrite:z.literal(false),canVerify:z.literal(false),canRollback:z.literal(false),evidenceSources:z.array(text),reason:text}).strict();
export const connectionStateSchema=z.enum(["discovered","adapter_candidate","identity_verification_required","authorization_required",
  "authentication_required","connecting","connected_read_only","disconnected","stale","failed","unsupported"]);
export const adapterEvaluationSchema=z.object({
  deviceId:z.string().regex(/^[a-f0-9]{64}$/),discoveryRevision:z.number().int().nonnegative(),protectedDevice:z.boolean(),
  selection:z.enum(["candidate","no_match","ambiguous","protected"]),selectedAdapter:id.nullable(),
  connectionState:connectionStateSchema,reason:text,evaluatedAt:z.number().int().nonnegative(),expiresAt:z.number().int().nonnegative(),
  matches:z.array(z.object({
    adapter:adapterDescriptorSchema,status:z.enum(["exact","requires_verification","not_applicable","rejected"]),
    confidence:z.number().int().min(0).max(100),reason:text,matchedEvidence:z.array(text),missingEvidence:z.array(text),
    capabilities:z.array(adapterCapabilitySchema),features:z.object({dns:feature,wifiUserLimit:feature,apn:feature,ussd:feature}).strict(),
  }).strict()).max(64),
}).strict().superRefine((r,ctx)=>{
  if(r.connectionState==="connected_read_only")ctx.addIssue({code:"custom",message:"Passive evaluation cannot establish a connection."});
  if(r.selectedAdapter&&!r.matches.some(m=>m.adapter.id===r.selectedAdapter&&m.status==="exact"))
    ctx.addIssue({code:"custom",message:"Selected adapter lacks exact evidence."});
  if(r.protectedDevice&&r.selectedAdapter)ctx.addIssue({code:"custom",message:"Protected device cannot select an adapter."});
});
export const adapterErrorSchema=z.object({
  code:z.enum(["DEVICE_NOT_FOUND","DEVICE_DISCONNECTED","PROTECTED_DEVICE","ADAPTER_NOT_FOUND","ADAPTER_NOT_COMPATIBLE",
    "IDENTITY_UNVERIFIED","IDENTITY_CONFLICT","AUTHORIZATION_REQUIRED","AUTHENTICATION_REQUIRED","AUTHENTICATION_FAILED",
    "ENDPOINT_UNTRUSTED","CAPABILITY_UNKNOWN","OPERATION_UNSUPPORTED","OPERATION_NOT_IMPLEMENTED","OPERATION_NOT_AUTHORIZED",
    "DEPENDENCY_MISSING","CONNECTION_TIMEOUT","OPERATION_TIMEOUT","OPERATION_CANCELLED","TRANSPORT_ERROR","SESSION_EXPIRED","INTERNAL_ADAPTER_ERROR"]),
  message:text,operation:adapterOperationSchema.nullable(),retryable:z.boolean(),suggestedAction:text,
}).strict();
export type AdapterEvaluation=z.infer<typeof adapterEvaluationSchema>;
export type AdapterDescriptor=z.infer<typeof adapterDescriptorSchema>;
export type AdapterError=z.infer<typeof adapterErrorSchema>;
export type AdapterTargetKind="discovered"|"saved_endpoint";
