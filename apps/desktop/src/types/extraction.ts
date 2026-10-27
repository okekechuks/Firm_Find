import { z } from "zod";
const count = z.number().int().nonnegative();
const hash = z.string().regex(/^[a-f0-9]{64}$/);
export const extractionEntrySchema = z.object({
  path: z.string().max(240), kind: z.enum(["file", "directory", "symlink_metadata", "special_omitted"]),
  size: count.max(32 * 1024 * 1024), sha256: hash.nullable(), linkTarget: z.string().max(1024).nullable(), offset: count,
  exportError: z.string().nullable().optional(),
});
export const extractionReportSchema = z.object({
  unextractedRegions: z.array(z.object({offset:count,length:count})).max(2),
  version: z.literal(1), inputSha256: hash, inputBytes: count, bundleSha256: hash, createdAt: count,
  container: z.enum(["raw", "gzip", "trx"]), expandedBytes: count,
  containerInfo: z.object({format:z.literal("trx"),version:z.literal(1),flags:z.literal(0),length:count,partitionOffsets:z.array(count).min(1).max(3),crc32:z.string(),parser:z.string(),sourceCommit:z.string(),sourceSha256:hash,parserSha256:hash}).optional(),
  region: z.object({format: z.literal("squashfs"), offset: count, length: count, compression: count}),
  extractor: z.string(), toolSha256: hash, entries: z.array(extractionEntrySchema).max(20000), totalBytes: count,
  workerExit: z.literal(0), status: z.enum(["partially_extracted", "extracted_verified"]),
  limitations: z.array(z.string()), isolation: z.object({network:z.literal(false),hostDrives:z.literal(false)}).passthrough(),
});
export type ExtractionReport = z.infer<typeof extractionReportSchema>;
