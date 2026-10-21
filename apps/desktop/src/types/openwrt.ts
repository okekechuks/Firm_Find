import { z } from "zod";
import { adapterErrorSchema } from "./adapters";

const text = z.string().min(1).max(2048).refine(value => !Array.from(value).some(character => {
  const point = character.codePointAt(0) ?? 0;
  return point < 32 || point === 127;
}), "Control characters are not displayable.");
const uint = z.number().int().nonnegative().safe();
export const openWrtTargetIdSchema = z.string().regex(/^[a-f0-9]{64}$/);
export const hostFingerprintSchema = z.string().regex(/^SHA256:[A-Za-z0-9+/]{43}$/);
export const openWrtIdentitySchema = z.object({
  manufacturer: text.nullable(), model: text, hardwareRevision: text.nullable(),
  firmwareFamily: z.literal("OpenWrt"), firmwareVersion: text, boardName: text,
  buildRevision: text, target: text,
}).strict();

const boardSchema = z.object({model: text, board_name: text, release: z.object({
  distribution: z.literal("OpenWrt"), version: text, revision: text, target: text,
}).strict(), rootfs_type: text.nullable().optional()}).strict();
export const firmwareAssessmentSchema = z.object({
  status: z.enum(["REQUIRES_VERIFICATION", "REQUIRES_TOOL", "UNSUPPORTED", "UNKNOWN"]),
  reason: text, evidence: z.array(text).max(64), missingRequirements: z.array(text).max(64),
  executable: z.literal(false),
}).strict();
const pathSchema = z.object({directConfiguration: firmwareAssessmentSchema, firmwareRebuild: firmwareAssessmentSchema}).strict();
export const firmwareFeasibilitySchema = z.object({
  deviceIdentity: boardSchema, acquisition: firmwareAssessmentSchema, extraction: firmwareAssessmentSchema,
  partitionLayout: firmwareAssessmentSchema,
  partitions: z.array(z.object({name: text, identifier: text, bytes: uint, erase_bytes: uint}).strict()).max(256),
  imageFormat: firmwareAssessmentSchema,
  customization: z.object({dns: pathSchema, wifiUserLimit: pathSchema, apn: pathSchema, ussd: pathSchema}).strict(),
  build: firmwareAssessmentSchema, installation: firmwareAssessmentSchema, recovery: firmwareAssessmentSchema,
  restrictions: z.array(text).max(64), lastObserved: uint, confidence: text,
}).strict();

const nullableText = text.nullable();
const addressesSchema = z.union([text, z.array(text).max(16)]).nullable();
const networkSchema = z.object({name: text, proto: nullableText, dns: addressesSchema,
  peerdns: nullableText, apn: nullableText, auto_apn: nullableText, pdptype: nullableText, auth: nullableText}).strict();
const wirelessSchema = z.object({name: text, radio: nullableText, ssid: z.string().max(32).nullable(), maxassoc: nullableText,
  mode: nullableText, disabled: nullableText}).strict();
export const openWrtInspectionSchema = z.object({
  networks: z.array(networkSchema).max(64), wireless: z.array(wirelessSchema).max(64),
  runtimeDns: z.array(z.object({name: text, up: z.boolean(), proto: nullableText, dns: addressesSchema}).strict()).max(64).nullable(),
  dhcpDns: z.array(z.object({name: text, interface: nullableText, ipv4_options: z.array(text).max(16), ipv6_dns: addressesSchema, ra: nullableText, dhcpv6: nullableText}).strict()).max(64).nullable(),
  radios: z.array(z.object({name: text, driver: nullableText, maxassoc: nullableText, disabled: nullableText}).strict()).max(64).nullable(),
  wirelessRuntime: z.array(z.object({name: text, driver: nullableText, phy: nullableText, status: nullableText, associated_clients: uint.max(4294967295).nullable()}).strict()).max(64).nullable(),
  limitations: z.array(text).max(64),
  ussd: z.object({assessment: z.enum(["UNKNOWN", "UNSUPPORTED"]), reason: text}).strict(),
  firmware: firmwareFeasibilitySchema,
}).strict();
export const openWrtStatusSchema = z.object({
  enabled: z.boolean(), busy: z.boolean(), generation: uint,
  state: z.enum(["idle", "host_observed", "identity_only", "read_authorized", "inspected", "disconnected", "failed"]),
  targets: z.array(z.object({id: openWrtTargetIdSchema, label: z.string().max(120), address: text,
    interfaceId: nullableText, lab: z.boolean()}).strict()).max(256),
  targetId: openWrtTargetIdSchema.nullable(),
  endpoint: z.string().max(2048).regex(/^ssh:\/\/(?:\[[0-9a-fA-F:]+\]|[0-9.]+):[0-9]{1,5}$/).nullable(),
  interfaceId: nullableText, sourceAddress: nullableText, hostFingerprint: hostFingerprintSchema.nullable(), expiresAt: uint.nullable(),
  identity: openWrtIdentitySchema.nullable(), profileQualified: z.boolean(), readApproved: z.boolean(),
  inspection: openWrtInspectionSchema.nullable(), error: adapterErrorSchema.nullable(),
}).strict().superRefine((status, context) => {
  if (status.readApproved && (!status.profileQualified || !status.identity || !status.hostFingerprint || !status.targetId)) {
    context.addIssue({code: "custom", message: "Read approval requires a qualified authenticated target."});
  }
  if (status.inspection && (!status.readApproved || !status.identity)) {
    context.addIssue({code: "custom", message: "Inspection requires separate read approval."});
  }
  if (status.identity && status.inspection && (
    status.identity.model !== status.inspection.firmware.deviceIdentity.model ||
    status.identity.boardName !== status.inspection.firmware.deviceIdentity.board_name ||
    status.identity.firmwareVersion !== status.inspection.firmware.deviceIdentity.release.version ||
    status.identity.target !== status.inspection.firmware.deviceIdentity.release.target
  )) context.addIssue({code: "custom", message: "Firmware report belongs to a different authenticated identity."});
});

export const openWrtObserveSchema = z.object({targetId: openWrtTargetIdSchema, port: z.number().int().min(1).max(65535), approved: z.literal(true)}).strict();
export const openWrtConnectSchema = z.object({fingerprint: hostFingerprintSchema, username: z.string().regex(/^[a-z_][a-z0-9_-]{0,31}$/), approved: z.literal(true)}).strict();

export type FirmwareAssessment = z.infer<typeof firmwareAssessmentSchema>;
export type FirmwareFeasibility = z.infer<typeof firmwareFeasibilitySchema>;
export type OpenWrtStatus = z.infer<typeof openWrtStatusSchema>;
export type OpenWrtInspection = z.infer<typeof openWrtInspectionSchema>;
