import { z } from "zod";

const safeText = z.string().max(2048).refine(value => {
  for (let index = 0; index < value.length; index++) {
    const code = value.charCodeAt(index);
    if (code < 32 || code === 127) return false;
  }
  return true;
}, "Control characters refused.");
const selectedHeaders = z.record(safeText).superRefine((headers, context) => {
  for (const [name, value] of Object.entries(headers)) {
    if (!["content-type", "content-length", "allow"].includes(name)
      || (name === "content-length" && !/^\d{1,20}$/.test(value))
      || (name === "allow" && !/^[A-Z, ]{1,128}$/.test(value))) {
      context.addIssue({ code: z.ZodIssueCode.custom, message: "Only sanitized non-secret header metadata is accepted." });
    }
  }
});
export const webRequestClasses = ["identity_read", "status_read", "authentication", "config_read", "config_write", "config_backup", "config_restore", "firmware_metadata", "firmware_update_check", "firmware_upload", "firmware_download", "file_export", "reboot", "factory_reset", "unknown"] as const;
const endpoint = z.object({
  id: z.string().regex(/^[a-f0-9]{64}$/), origin: safeText, path: safeText,
  method: z.string().regex(/^[A-Z]{1,16}$/), queryNames: z.array(safeText).max(128),
  classification: z.enum(webRequestClasses), evidence: z.enum(["har_observed", "static_candidate", "both"]),
  readOnlyQualified: z.literal(false), status: z.number().int().min(100).max(599).nullable(),
  mimeType: safeText.nullable(), requestContentType: safeText.nullable(),
  requestBodyNames: z.array(safeText).max(128), selectedHeaders, context: safeText.nullable(),
  responseSize: z.number().int().nonnegative().nullable().optional(),
}).strict();
export const webEvidenceSchema = z.object({
  schemaVersion: z.literal(1), deviceId: z.string().regex(/^[a-f0-9]{64}$/),
  entries: z.array(endpoint).max(4096), authCandidates: z.array(z.enum(["none", "basic", "digest", "cookie_session", "form_session", "unknown"])).max(32),
  resources: z.array(z.object({ origin: safeText, path: safeText, kind: z.enum(["html", "javascript", "json", "css"]), referenced: z.literal(true) }).strict()).max(512),
  counts: z.record(z.number().int().nonnegative()), warnings: z.array(safeText).max(128),
  canAcquire: z.literal(false), retention: z.literal("memory_only"),
  strategy: z.object({
    strategy: z.literal("web_management_acquisition"), canAcquire: z.literal(false),
    sources: z.array(z.object({evidenceId:z.string().regex(/^[a-f0-9]{64}$/),sourceType:z.enum(["documented_device_export","observed_firmware_download","observed_fota_package","verified_vendor_download","config_export","unknown"]),state:safeText,readOnlyQualified:z.literal(false),firmwareCompatible:z.literal(false),reason:safeText}).strict()).max(512),
    missingRequirements:z.array(safeText).max(16),
  }).strict(),
  actions: z.object({ importHar: z.boolean(), analyzeLocalBundle: z.boolean(), collectResources: z.literal(false), authenticate: z.literal(false), reason: safeText }).strict(),
}).strict();
export type WebEvidenceReport = z.infer<typeof webEvidenceSchema>;
export type WebEndpointEvidence = WebEvidenceReport["entries"][number];
export const bindingAssessmentSchema = z.object({state:z.enum(["observed","blocked"]),sourceIp:z.string().ip({version:"v4"}).nullable(),interfaceId:safeText.nullable(),reason:safeText,cryptographicallyVerified:z.literal(false),canAuthenticate:z.literal(false)}).strict();
export type BindingAssessment = z.infer<typeof bindingAssessmentSchema>;
