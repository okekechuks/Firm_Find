import { z } from "zod";
import { acquisitionStateSchema } from "./acquisition";
import { evidenceSchema } from "./profiles";
import { discoverySchema } from "./contracts";
import { artifactSchema, jobSchema } from "./library";
import { officialSourceSchema, analysisReportSchema } from "./official";
const hash=z.string().regex(/^[a-f0-9]{64}$/);
export const workflowBindingSchema=z.object({deviceId:hash,revision:z.number().int().nonnegative(),identityDigest:hash,expiresAt:z.number().int().nonnegative()}).strict();
const stage=z.object({id:z.string(),state:acquisitionStateSchema,reasonCode:z.string(),reason:z.string(),evidence:z.array(z.string()).max(256),nextAction:z.string(),executable:z.boolean(),job:jobSchema.optional()}).strict();
export const workflowSchema=z.object({
  device:discoverySchema.shape.devices.element,binding:workflowBindingSchema,
  identity:z.object({confidence:z.enum(["conflicting","verified","unverified"]),verifiedManufacturer:z.string().nullable(),verifiedFirmwareFamily:z.string().nullable(),verifiedModel:z.string().nullable(),verifiedHardwareRevision:z.string().nullable(),verifiedFirmware:z.string().nullable(),reportedModel:z.string().nullable(),evidence:z.array(evidenceSchema).max(256)}).strict(),
  selectedProfile:z.string().nullable(),adapter:z.string().nullable(),stages:z.array(stage).length(9),
  strategies:z.array(z.object({strategy:z.enum(["official_image","documented_export","authorized_storage_read","manufacturer_package","external_laboratory"]),name:z.string(),provenanceKind:z.string(),capability:z.object({state:acquisitionStateSchema,reason:z.string(),evidence:z.array(z.string()),missingRequirements:z.array(z.string()),executable:z.boolean()}).strict()}).strict()).length(5),
  officialSources:z.array(officialSourceSchema).max(1),
  artifacts:z.array(z.object({artifact:artifactSchema,source:officialSourceSchema,association:z.literal("historical_verified_target_official_image_not_device_dump"),analysis:analysisReportSchema.nullable(),extracted:z.boolean(),canAnalyze:z.boolean(),canExtract:z.boolean()}).strict()).max(100),
  jobs:z.array(jobSchema).max(100),configuration:z.array(stage).max(16),
}).strict().superRefine((r,ctx)=>{
  if(r.device.id!==r.binding.deviceId || new Set(r.stages.map(s=>s.id)).size!==9)ctx.addIssue({code:z.ZodIssueCode.custom,message:"Inconsistent device workflow."});
  if(r.stages.some(s=>["customization","rebuild","installation","recovery"].includes(s.id)&&s.executable)||r.configuration.some(s=>s.executable))ctx.addIssue({code:z.ZodIssueCode.custom,message:"Unqualified write/build authority refused."});
  if(r.artifacts.some(a=>a.artifact.sha256!==a.source.sha256 || (a.analysis&&a.analysis.inputSha256!==a.artifact.sha256)))ctx.addIssue({code:z.ZodIssueCode.custom,message:"Workflow artifact provenance mismatch."});
});
export type DeviceWorkflow=z.infer<typeof workflowSchema>;
export type WorkflowBinding=z.infer<typeof workflowBindingSchema>;
