import { z } from 'zod';

export const settingsSchema = z.object({ theme: z.enum(['light', 'dark', 'system']) }).strict();
export type Settings = z.infer<typeof settingsSchema>;
export const snapshotSchema = z.object({
  version: z.string(),
  databaseVersion: z.number().int().nonnegative(),
  dataDirectory: z.string(),
  settings: settingsSchema,
  counts: z.object({ devices: z.number().int().nonnegative(), artifacts: z.number().int().nonnegative(), runningJobs: z.number().int().nonnegative() }),
  recentEvents: z.array(z.object({ id: z.number().int(), kind: z.string(), message: z.string(), createdAt: z.string() })),
});
export type Snapshot = z.infer<typeof snapshotSchema>;
