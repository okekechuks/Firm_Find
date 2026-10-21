// Synthetic IPC fixtures. These do not constitute SSH, OpenWrt OS or hardware evidence.
import type { FirmwareAssessment, OpenWrtStatus } from "../types/openwrt";
export const openWrtTarget = "a".repeat(64);
export const openWrtPin = `SHA256:${"a".repeat(43)}`;
const unknown = (reason: string): FirmwareAssessment => ({status: "UNKNOWN", reason, evidence: [], missingRequirements: ["Independent qualification required"], executable: false});
const pathway = {directConfiguration: unknown("No configuration transaction qualified."), firmwareRebuild: unknown("No feature-specific image built.")};
export function openWrtFixture(patch: Partial<OpenWrtStatus> = {}): OpenWrtStatus {
  return {enabled: true, busy: false, generation: 1, state: "idle",
    targets: [{id: openWrtTarget, label: "Fixture OpenWrt", address: "http://192.168.8.1/", interfaceId: "fixture-net", lab: false}],
    targetId: null, endpoint: null, interfaceId: null, sourceAddress: null, hostFingerprint: null, expiresAt: null,
    identity: null, profileQualified: false, readApproved: false, inspection: null, error: null, ...patch};
}
export function observedOpenWrt(): OpenWrtStatus {
  return openWrtFixture({state: "host_observed", targetId: openWrtTarget, endpoint: "ssh://192.168.8.1:22", interfaceId: "fixture-net", sourceAddress: "192.168.8.2", hostFingerprint: openWrtPin, expiresAt: Date.now() + 60000});
}
export function authenticatedOpenWrt(qualified = true): OpenWrtStatus {
  return {...observedOpenWrt(), state: "identity_only", profileQualified: qualified,
    identity: {manufacturer: null, model: "QEMU fixture", hardwareRevision: null, firmwareFamily: "OpenWrt", firmwareVersion: "25.12.6", boardName: "fixture,board", buildRevision: "r-fixture", target: "x86/64"}};
}
export function inspectedOpenWrt(): OpenWrtStatus {
  const status = authenticatedOpenWrt();
  return {...status, state: "inspected", readApproved: true, inspection: {
    networks: [{name: "wan", proto: "dhcp", dns: ["1.1.1.1", "2606:4700:4700::1111"], peerdns: "0", apn: null, auto_apn: null, pdptype: null, auth: null}],
    runtimeDns: null, dhcpDns: null, radios: null, wirelessRuntime: null, limitations: [],
    wireless: [], ussd: {assessment: "UNKNOWN", reason: "No verified modem interface observed."}, firmware: {
      deviceIdentity: {model: status.identity!.model, board_name: status.identity!.boardName, rootfs_type: "squashfs", release: {distribution: "OpenWrt", version: "25.12.6", revision: "r-fixture", target: "x86/64"}},
      acquisition: {...unknown("Official route requires exact image verification."), status: "REQUIRES_VERIFICATION", evidence: ["Official release directory"]},
      extraction: unknown("No qualified physical extraction method."), partitionLayout: unknown("MTD metadata unavailable."), partitions: [],
      imageFormat: unknown("Image format requires independent validation."), customization: {dns: pathway, wifiUserLimit: pathway, apn: pathway, ussd: pathway},
      build: {...unknown("ImageBuilder candidate; build engine pending."), status: "REQUIRES_TOOL"}, installation: unknown("Installation engine not implemented."), recovery: unknown("Recovery not hardware tested."),
      restrictions: ["Inspection only; no writes, extraction or flashing."], lastObserved: Date.now(), confidence: "Authenticated metadata; physical qualification pending",
    },
  }};
}
