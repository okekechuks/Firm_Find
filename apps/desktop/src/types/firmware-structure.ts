import { z } from "zod";

const count = z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER);
const hash = z.string().regex(/^[a-f0-9]{64}$/);
const readiness = z.object({ state: z.enum(["observed", "unknown", "unsupported"]), executable: z.literal(false), reason: z.string().max(2048) }).strict();
const region = z.object({
  id: z.string().max(40), parentId: z.string().max(40).nullable(), children: z.array(z.string().max(40)).max(4352),
  format: z.string().max(40), kind: z.enum(["artifact", "container", "filesystem", "compression", "delta", "unknown"]),
  coordinateSpace: z.enum(["source", "expanded", "member"]), relativeOffset: count, length: count.positive(), sourceAbsoluteOffset: count.nullable(), sha256: hash.nullable(),
  confidence: z.enum(["signature", "validated", "unknown"]),
  states: z.array(z.enum(["detected", "bounded", "validated", "extractable", "extracted", "unsupported", "corrupt", "encrypted_or_high_entropy", "unknown"])).max(9),
  extractor: z.string().max(256).nullable(), extractorVersion: count.nullable(), reasons: z.array(z.string().max(2048)).max(4096),
  heuristics: z.array(z.object({ kind: z.enum(["kernel_candidate", "rootfs_candidate"]), reason: z.string().max(2048), verified: z.literal(false) }).strict()).max(2),
}).strict();
export const firmwareStructureSchema = z.object({
  version: z.literal(1), inputSha256: hash, inputBytes: count.positive(), bundleSha256: hash, sourceReportVersion: z.union([z.literal(1), z.literal(2)]), sourceStatus: z.string().max(40), fileCount: count, totalExtractedBytes: count,
  provenance: z.object({ kind: z.literal("library_image_analysis_not_physical_acquisition"), extractor: z.string().max(256), toolSha256: hash, engineVersion: z.string().max(40).nullable(), engineSha256: hash.nullable(), tools: z.array(z.object({ name: z.string().max(240), version: z.string().max(128), sha256: hash }).strict()).max(16) }).strict(),
  regions: z.array(region).min(1).max(4352),
  entropyObservations: z.array(z.object({ offset: count, length: count.positive(), sampleBytes: count, bitsPerByte: z.number().finite().min(0).max(8), classification: z.enum(["high_entropy_compressed_or_encrypted", "mixed_or_low_entropy"]) }).strict()).max(512),
  packageMetadata: z.array(z.object({ layerId: z.string().max(40), versions: z.array(z.string().max(128)).max(32), authority: z.literal("package_declarations_unverified"), signatureVerified: z.literal(false) }).strict()).max(256),
  readiness: z.object({ extracted: readiness, modifiable: readiness, repackable: readiness, signatureCompatible: readiness, installable: readiness }).strict(),
  limitations: z.array(z.string().max(2048)).max(4096),
}).strict().superRefine((value, context) => {
  const regions = new Map<string, z.infer<typeof region>>();
  const children = new Map<string, string[]>();
  const depths = new Map<string, number>();
  for (const item of value.regions) {
    const parent = item.parentId ? regions.get(item.parentId) : null;
    const depth = parent ? (depths.get(parent.id) ?? 0) + 1 : 0;
    const expected = item.coordinateSpace === "source" ? parent ? parent.sourceAbsoluteOffset === null ? null : parent.sourceAbsoluteOffset + item.relativeOffset : item.relativeOffset : null;
    if (regions.has(item.id) || (item.parentId && !parent) || item.sourceAbsoluteOffset !== expected
      || depth > 9 || !Number.isSafeInteger(item.relativeOffset + item.length)
      || (!item.parentId && (regions.size !== 0 || item.coordinateSpace !== "source" || item.relativeOffset !== 0 || item.length !== value.inputBytes || item.sha256 !== value.inputSha256))
      || (parent && item.coordinateSpace === "source" && item.relativeOffset + item.length > parent.length)
      || (expected !== null && (!Number.isSafeInteger(expected + item.length) || expected + item.length > value.inputBytes))) {
      context.addIssue({ code: z.ZodIssueCode.custom, message: "Inconsistent firmware structure coordinates or topology." });
    }
    regions.set(item.id, item);
    depths.set(item.id, depth);
    if (item.parentId) {
      const siblings = children.get(item.parentId);
      if (siblings) siblings.push(item.id);
      else children.set(item.parentId, [item.id]);
    }
  }
  for (const item of value.regions) {
    const actual = children.get(item.id) ?? [];
    if (item.children.length !== actual.length || item.children.some((id, index) => id !== actual[index])) context.addIssue({ code: z.ZodIssueCode.custom, message: "Inconsistent firmware structure children." });
  }
});
export type FirmwareStructureReport = z.infer<typeof firmwareStructureSchema>;
export type FirmwareRegion = FirmwareStructureReport["regions"][number];
