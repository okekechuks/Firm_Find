import { z } from "zod";
const count = z.number().int().nonnegative();
const hash = z.string().regex(/^[a-f0-9]{64}$/);
export const extractionEntrySchema = z.object({
  path: z.string().max(240), kind: z.enum(["file", "directory", "symlink_metadata", "special_omitted"]),
  size: count.max(32 * 1024 * 1024), sha256: hash.nullable(), linkTarget: z.string().max(1024).nullable(), offset: count,
  exportError: z.string().nullable().optional(),
  mode: count.max(0o777).optional(),
});
const legacyReportSchema = z.object({
  unextractedRegions: z.array(z.object({offset:count,length:count})).max(2),
  version: z.literal(1), inputSha256: hash, inputBytes: count, bundleSha256: hash, createdAt: count,
  container: z.enum(["raw", "gzip", "trx"]), expandedBytes: count,
  containerInfo: z.object({format:z.literal("trx"),version:z.literal(1),flags:z.literal(0),length:count,partitionOffsets:z.array(count).min(1).max(3),crc32:z.string(),parser:z.string(),sourceCommit:z.string(),sourceSha256:hash,parserSha256:hash}).optional(),
  region: z.object({format: z.literal("squashfs"), offset: count, length: count, compression: count}),
  extractor: z.string(), toolSha256: hash, entries: z.array(extractionEntrySchema).max(20000), totalBytes: count,
  workerExit: z.literal(0), status: z.enum(["partially_extracted", "extracted_verified"]),
  limitations: z.array(z.string()), isolation: z.object({network:z.literal(false),hostDrives:z.literal(false)}).passthrough(),
});
const completeness = z.enum(["complete", "partial", "unsupported", "encrypted-or-unknown"]);
const layerSchema = z.object({
  id:z.string().max(32),parentId:z.string().max(32).nullable(),space:z.enum(["source","expanded","member"]),
  offset:count,length:count,sourcePath:z.string().max(240),inputSha256:hash,handler:z.string().max(40),
  handlerVersion:count,format:z.string().max(40),confidence:z.enum(["signature","validated"]),decodedBytes:count,
  completeness,reasons:z.array(z.string().max(1024)).max(4096),
}).strict();
const layeredReportSchema = legacyReportSchema.extend({
  version:z.literal(2),container:z.string().max(40),createdAt:z.literal(0),
  region:z.object({format:z.string().max(40),offset:count,length:count,compression:count}).strict(),
  status:z.enum(["extracted_verified","partially_extracted","inspection_only","unsupported","encrypted-or-unknown"]),
  completeness,layers:z.array(layerSchema).min(1).max(256),
  unextractedRegions:z.array(z.object({offset:count,length:count})).max(4096),
  unexplainedRegions:z.array(z.object({layerId:z.string().max(32),offset:count,length:count,reason:z.string().max(1024)}).strict()).max(4096),
  entropyMap:z.array(z.object({offset:count,length:count,sampleBytes:count,bitsPerByte:z.number().finite().min(0).max(8),classification:z.enum(["high_entropy_compressed_or_encrypted","mixed_or_low_entropy"])}).strict()).max(512),
  omittedMembers:z.array(z.object({layerId:z.string().max(32),path:z.string().max(240),declaredBytes:count,reason:z.string().max(1024)}).strict()).max(4096),
  scan:z.object({bytes:count,allOffsetsScanned:z.literal(true),candidateLimit:z.literal(4096),candidatesRetained:count.max(4096),candidatesCapped:z.boolean(),entropySampled:z.literal(true)}).strict(),
  provenance:z.object({engineVersion:z.literal("2.0.0"),engineSha256:hash,pythonVersion:z.string().max(32),tools:z.array(z.object({name:z.string().max(240),version:z.string().max(128),sha256:hash}).strict()).max(16)}).strict(),
  budgets:z.object({maxLayerDepth:z.literal(6),maxLayers:z.literal(256),expandedBytes:count.max(128*1024*1024),maxExpandedBytes:z.literal(128*1024*1024),maxFileBytes:z.literal(32*1024*1024),ratioLimit:z.literal(1024)}).strict(),
  swupdate:z.array(z.object({layerId:z.string().max(32),versions:z.array(z.string().max(128)).max(32),declarations:z.array(z.object({filename:z.string().max(512).optional(),type:z.string().max(512).optional(),sha256:z.string().max(512).optional(),size:z.string().max(512).optional(),name:z.string().max(512).optional(),baseRequirements:z.record(z.string(),z.string().max(512)).optional()}).strict()).max(256),authority:z.literal("package_declarations_unverified"),signatureVerified:z.literal(false)}).strict()).max(256),
});
export const extractionReportSchema = z.discriminatedUnion("version",[legacyReportSchema,layeredReportSchema]);
export type ExtractionReport = z.infer<typeof extractionReportSchema>;
