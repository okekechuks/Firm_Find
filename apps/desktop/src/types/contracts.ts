import { z } from "zod";

export const settingsSchema = z
  .object({ theme: z.enum(["light", "dark", "system"]) })
  .strict();
export type Settings = z.infer<typeof settingsSchema>;
export const snapshotSchema = z.object({
  version: z.string(),
  databaseVersion: z.number().int().nonnegative(),
  dataDirectory: z.string(),
  testProfile: z.object({runId:z.string().regex(/^[a-f0-9]{32}$/),discoverySource:z.literal("windows_observed"),simulatedAdapters:z.literal(false),simulationAvailable:z.boolean().optional()}).optional(),
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

const capabilitySchema = z.enum([
  "SUPPORTED", "UNSUPPORTED", "UNKNOWN", "REQUIRES_AUTH", "REQUIRES_TOOL", "EXPERIMENTAL",
]);
const featureCapabilitySchema = z.object({
  state: capabilitySchema,
  controlPlane: z.string().nullable(),
  canRead: z.boolean(),
  canWrite: z.boolean(),
  canVerify: z.boolean(),
  canRollback: z.boolean(),
  confidence: z.number().int().min(0).max(100),
  evidence: z.array(z.string()),
});
const identitySchema = z.object({
  confidence: z.enum(["verified", "probable", "candidate", "unknown", "conflicting"]),
  manufacturer: z.string().nullable(), model: z.string().nullable(), firmware: z.string().nullable(),
  explanation: z.string(), observations: z.array(z.object({kind: z.string(), value: z.string(), source: z.string(), reliability: z.string(), observedAt: z.number()})),
});
export const discoverySchema = z.object({
  revision: z.number().int().nonnegative(),
  scannedAt: z.number().int().nonnegative().nullable(),
  status: z.enum(["starting", "ready", "degraded", "failed"]),
  watching: z.boolean(),
  cancelled: z.boolean().optional(), durationMs: z.number().nonnegative().optional(),
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
      manufacturer: z.string().nullable().optional(), compatibleIds: z.array(z.string()).optional(), interfacePaths: z.array(z.string()).optional(),
    })),
    networks: z.array(z.object({
      id: z.string(), displayName: z.string(), description: z.string(),
      interfaceIndex: z.number().int().nonnegative(), interfaceType: z.number().int().nonnegative(),
      linkUp: z.boolean(), addresses: z.array(z.string()), gateways: z.array(z.string()),
      prefixes: z.array(z.object({address: z.string(), length: z.number().int().min(0).max(128)})).optional(),
      dnsServers: z.array(z.string()).optional(), luid: z.string().optional(), ipv6Index: z.number().int().optional(),
      dhcpServer: z.string().ip({version:"v4"}).nullable().optional(),
      virtualInterface: z.boolean().optional(), defaultRoutes: z.array(z.object({gateway: z.string(), metric: z.number().int(), family: z.enum(["ipv4", "ipv6"])})).optional(),
    })),
    identity: identitySchema.optional(),
    category: z.enum(["router_mifi_candidate", "usb_modem_candidate", "network_gateway_candidate", "generic_network_interface", "unrelated_device", "unknown"]).optional(),
    lifecycle: z.string().optional(),
    assessment: z.string(), confidence: z.number().int().min(0).max(100), evidence: z.array(z.string()),
    capabilities: z.object({
      inventory: capabilitySchema, backup: capabilitySchema, flash: capabilitySchema,
      features: z.object({
        apn: featureCapabilitySchema, wifiUserLimit: featureCapabilitySchema,
        customDns: featureCapabilitySchema, ussd: featureCapabilitySchema,
      }),
    }),
    lastSeenAt: z.number().int().nonnegative(),
  })),
});
export type DiscoverySnapshot = z.infer<typeof discoverySchema>;
export type Device = DiscoverySnapshot["devices"][number];

export const endpointResultSchema = z.object({
  code: z.string(), message: z.string(), state: z.string(), status: z.number().int().nullable(),
  server: z.string().nullable(), checkedAt: z.number().nonnegative(), elapsedMs: z.number().nonnegative(),
  resolvedAddress: z.string().nullable(), tls: z.enum(["not_checked", "not_used", "validated", "untrusted"]),
});
export type EndpointResult = z.infer<typeof endpointResultSchema>;
export const savedRouterSchema = z.object({
  id: z.string().regex(/^[a-f0-9]{64}$/), address: z.string(), label: z.string(),
  interfaceId: z.string().nullable(), deviceId: z.string().nullable(), createdAt: z.number().nonnegative(),
  lastResult: endpointResultSchema.nullable(), state: z.string(),
});
export type SavedRouter = z.infer<typeof savedRouterSchema>;
export type ManualRouterInput = {address: string; label: string; interfaceId: string | null};
