import { z } from "zod";

export const settingsSchema = z
  .object({ theme: z.enum(["light", "dark", "system"]) })
  .strict();
export type Settings = z.infer<typeof settingsSchema>;
export const snapshotSchema = z.object({
  version: z.string(),
  databaseVersion: z.number().int().nonnegative(),
  dataDirectory: z.string(),
  settings: settingsSchema,
  counts: z.object({
    devices: z.number().int().nonnegative(),
    artifacts: z.number().int().nonnegative(),
    runningJobs: z.number().int().nonnegative(),
  }),
  recentEvents: z.array(
    z.object({
      id: z.number().int(),
      kind: z.string(),
      message: z.string(),
      createdAt: z.string(),
    }),
  ),
});
export type Snapshot = z.infer<typeof snapshotSchema>;

const capabilitySchema = z.enum(["SUPPORTED", "UNSUPPORTED", "UNKNOWN"]);
export const discoverySchema = z.object({
  revision: z.number().int().nonnegative(),
  scannedAt: z.number().int().nonnegative().nullable(),
  status: z.enum(["starting", "ready", "degraded", "failed"]),
  watching: z.boolean(),
  warnings: z.array(z.string()),
  devices: z.array(z.object({
    id: z.string().regex(/^[a-f0-9]{64}$/),
    displayName: z.string(),
    connected: z.boolean(),
    transports: z.array(z.enum(["usb", "serial", "network"])),
    vendorId: z.string().regex(/^[A-F0-9]{4}$/).nullable(),
    productId: z.string().regex(/^[A-F0-9]{4}$/).nullable(),
    containerId: z.string().nullable(),
    interfaces: z.array(z.object({
      instanceId: z.string(), parentInstanceId: z.string().nullable(),
      className: z.string(), classGuid: z.string(),
      interfaceClasses: z.array(z.string()), hardwareIds: z.array(z.string()),
      portName: z.string().nullable(), present: z.boolean(),
    })),
    networks: z.array(z.object({
      id: z.string(), displayName: z.string(), description: z.string(),
      interfaceIndex: z.number().int().nonnegative(), interfaceType: z.number().int().nonnegative(),
      linkUp: z.boolean(), addresses: z.array(z.string()), gateways: z.array(z.string()),
    })),
    assessment: z.string(), confidence: z.number().int().min(0).max(100), evidence: z.array(z.string()),
    capabilities: z.object({inventory: capabilitySchema, backup: capabilitySchema, flash: capabilitySchema}),
    lastSeenAt: z.number().int().nonnegative(),
  })),
});
export type DiscoverySnapshot = z.infer<typeof discoverySchema>;
export type Device = DiscoverySnapshot["devices"][number];
