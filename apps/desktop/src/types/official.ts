import { z } from "zod";
const hash=z.string().regex(/^[a-f0-9]{64}$/);
const count=z.number().int().nonnegative();
export const officialSourceSchema=z.object({id:z.string(),vendor:z.string(),model:z.string(),revision:z.string(),profile:z.string(),target:z.string(),version:z.string(),filename:z.string(),sha256:hash,sizeBytes:count.max(128*1024*1024),url:z.string().url(),manifestUrl:z.string().url(),manifestSha256:hash,profilesSha256:hash,signingFingerprint:z.string(),primaryFingerprint:z.string(),verification:z.literal("pinned_catalogue_from_verified_release_signature"),installability:z.literal("unknown")});
export const analysisReportSchema=z.object({reportVersion:z.literal(1),inputSha256:hash,inputBytes:count,analyzedAt:count,scannedBytes:count.max(32*1024*1024),scanTruncated:z.boolean(),format:z.string(),regions:z.array(z.object({offset:count,format:z.string()})).max(64),sampledEntropy:z.number().min(0).max(8),entropySampleBytes:count.max(256*1024),recursiveExtraction:z.literal(false),executableContentsRun:z.literal(false),compatibility:z.literal("unknown"),limitations:z.array(z.string()).max(8)});
export type OfficialSource=z.infer<typeof officialSourceSchema>;
export type AnalysisReport=z.infer<typeof analysisReportSchema>;
