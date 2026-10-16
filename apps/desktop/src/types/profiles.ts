import { z } from "zod";

export const assessmentSchema = z.enum(["SUPPORTED", "UNSUPPORTED", "UNKNOWN"]);
export const readinessSchema = z.enum(["READY_FOR_FUTURE_ADAPTER", "REQUIRES_AUTHENTICATION", "REQUIRES_PACKAGE", "REQUIRES_FIRMWARE_BUILD", "REQUIRES_MANUAL_VERIFICATION", "NOT_IMPLEMENTED", "UNAVAILABLE", "EXPERIMENTAL"]);
export const matchConfidenceSchema = z.enum(["VERIFIED", "PROBABLE", "CANDIDATE", "AMBIGUOUS", "NO_MATCH", "CONFLICTING"]);
const attributeSchema = z.enum(["manufacturer", "model", "hardware_revision", "firmware_family", "firmware_version", "usb_vid", "usb_pid", "usb_product", "physical_identity", "management_identity", "modem_manufacturer", "modem_model", "windows_manufacturer", "gateway", "user_label", "http_status", "wireless_driver", "reported_identifier"]);
const sourceCategorySchema = z.enum(["manufacturer_documentation", "upstream_documentation", "hardware_observation", "hardware_tested", "experimental", "local_unverified"]);
const protocolSchema = z.enum(["http", "https", "ssh", "ubus", "modem_manager", "documented_modem"]);
const strings = z.array(z.string());
const rangeSchema = z.object({min: z.number().int().positive(), max: z.number().int().positive()}).nullable();
const subSchema = z.object({assessment: assessmentSchema, reason: z.string(), evidenceSources: strings}).strict();
const featureSpecSchema = z.object({assessment: assessmentSchema, reason: z.string(), requirements: z.array(readinessSchema), evidenceSources: strings, subCapabilities: z.record(z.string(), subSchema), verifiedRange: rangeSchema}).strict();
export const profileSchema = z.object({
  schemaVersion: z.literal(1), profileId: z.string(), profileVersion: z.string(), manufacturer: z.string(), model: z.string(), modelAliases: strings, deviceCategory: strings,
  hardwareRevisions: strings, firmwareFamilies: strings,
  firmwareVersionConstraints: z.object({scheme: z.enum(["numeric_triplet", "opaque_exact"]).optional(), exact: strings, minInclusive: z.string().nullable(), maxExclusive: z.string().nullable()}).strict(),
  matchRules: z.array(z.object({attribute: attributeSchema, values: strings, purpose: z.enum(["identity", "corroboration", "candidate_hint"]), explanation: z.string()}).strict()),
  managementInterfaces: z.array(z.object({protocol: protocolSchema, authenticationRequired: z.boolean(), firmwareFamilies: strings, operations: strings, security: z.string(), limitations: z.string(), evidenceSources: strings}).strict()),
  modemInformation: z.object({integratedModem: z.boolean().nullable(), manufacturers: strings, models: strings, interfaces: z.array(protocolSchema), notes: z.string()}).strict(),
  capabilities: z.object({dns: featureSpecSchema, wifiUserLimit: featureSpecSchema, apn: featureSpecSchema, ussd: featureSpecSchema}).strict(),
  knownLimitations: strings, evidenceSources: z.array(z.object({id: z.string(), category: sourceCategorySchema, reference: z.string(), summary: z.string()}).strict()),
  lastReviewed: z.string(), provenance: z.object({author: z.string(), category: sourceCategorySchema, notes: z.string()}).strict(),
}).strict();
export const registryEntrySchema = z.object({profile: profileSchema, origin: z.string(), authoritative: z.boolean(), contentHash: z.string()});
export const registrySchema = z.object({profiles: z.array(registryEntrySchema), diagnostics: strings, digest: z.string()});
const evidenceSchema = z.object({
  id: z.string(), subjectId: z.string(), attribute: attributeSchema, value: z.string(), source: z.enum(["windows_pnp", "physical_relationship", "network_topology", "authenticated_management", "supported_modem", "user_supplied", "endpoint_response"]),
  collectionMethod: z.string(), collectedAt: z.number(), reliability: z.enum(["strong", "observed", "hint"]), verification: z.enum(["verified", "observed", "user_reported", "unverified"]),
  interfaceId: z.string().nullable(), endpoint: z.string().nullable(), reference: z.string().nullable(), hardwareApplicability: z.string().nullable(), firmwareApplicability: z.string().nullable(), conflicts: strings, stale: z.boolean(),
});
const featureDecisionSchema = z.object({assessment: assessmentSchema, readiness: z.array(readinessSchema), quality: z.string(), reason: z.string(), nextAction: z.string(), evidenceSources: strings,
  subCapabilities: z.record(z.string(), subSchema), verifiedRange: rangeSchema, adapterImplemented: z.literal(false), canApply: z.literal(false)});
export const compatibilitySchema = z.object({
  deviceId: z.string(), evaluatedAt: z.number(), expiresAt: z.number(), engineVersion: z.string(), registryDigest: z.string(), inputDigest: z.string(), discoveryRevision: z.number(),
  confidence: matchConfidenceSchema, selectedProfile: z.string().nullable(), profileVersion: z.string().nullable(), profileContentHash: z.string().nullable(),
  candidates: z.array(z.object({profileId: z.string(), profileVersion: z.string(), name: z.string(), origin: z.string(), confidence: matchConfidenceSchema, authoritative: z.boolean(), matchedFields: z.array(attributeSchema), missingFields: z.array(attributeSchema), conflictingFields: z.array(attributeSchema), reasons: strings})),
  evidence: z.array(evidenceSchema), conflicts: z.array(attributeSchema), warnings: strings,
  features: z.object({dns: featureDecisionSchema, wifiUserLimit: featureDecisionSchema, apn: featureDecisionSchema, ussd: featureDecisionSchema}),
  change: z.object({previousEvaluatedAt: z.number(), previousProfile: z.string().nullable(), previousRegistryDigest: z.string(), reasons: strings}).nullable(),
});
export type Registry = z.infer<typeof registrySchema>;
export type CompatibilityReport = z.infer<typeof compatibilitySchema>;
