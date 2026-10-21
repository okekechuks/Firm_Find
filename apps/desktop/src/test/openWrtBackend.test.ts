import { beforeEach, expect, it, vi } from "vitest";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { openWrtApproveReads, openWrtConnect, openWrtInspect, openWrtObserve, openWrtStatus } from "../lib/openwrt";
import { inspectedOpenWrt, observedOpenWrt, openWrtFixture, openWrtPin, openWrtTarget } from "./openWrtFixtures";
import observedProfile from "../../../../device-profiles/observed-comfi-mg11.json";
import { profileSchema } from "../types/profiles";
vi.mock("@tauri-apps/api/core", () => ({invoke: vi.fn(), isTauri: vi.fn()}));
beforeEach(() => {vi.clearAllMocks(); vi.mocked(isTauri).mockReturnValue(true); vi.mocked(invoke).mockResolvedValue(openWrtFixture());});
it("requires desktop and rejects malformed or simulator-native status", async () => {
  vi.mocked(isTauri).mockReturnValue(false); await expect(openWrtStatus()).rejects.toThrow("Windows desktop"); expect(invoke).not.toHaveBeenCalled();
  vi.mocked(isTauri).mockReturnValue(true);
  for (const invalid of [{...observedOpenWrt(), endpoint: "simulator://native/router"}, {...openWrtFixture(), credentialPath: "C:\\secret"}, {...openWrtFixture(), readApproved: true}, {...observedOpenWrt(), hostFingerprint: "unverified"}]) {
    vi.mocked(invoke).mockResolvedValueOnce(invalid); await expect(openWrtStatus()).rejects.toThrow();
  }
});
it("sends only approved saved target, port and independently checked key fingerprint", async () => {
  vi.mocked(invoke).mockResolvedValueOnce(observedOpenWrt()); await openWrtObserve(openWrtTarget, 22);
  expect(invoke).toHaveBeenLastCalledWith("openwrt_observe", {request: {targetId: openWrtTarget, port: 22, approved: true}});
  await openWrtConnect(openWrtPin, "root");
  expect(invoke).toHaveBeenLastCalledWith("openwrt_connect", {request: {fingerprint: openWrtPin, username: "root", approved: true}});
  await openWrtApproveReads(); expect(invoke).toHaveBeenLastCalledWith("openwrt_approve_reads", {request: {approved: true}});
  await openWrtInspect(); expect(invoke).toHaveBeenLastCalledWith("openwrt_inspect", undefined);
});
it("rejects target substitution and invalid request data before transport", async () => {
  vi.mocked(invoke).mockResolvedValueOnce({...observedOpenWrt(), targetId: "b".repeat(64)}); await expect(openWrtObserve(openWrtTarget, 22)).rejects.toThrow("another target");
  vi.clearAllMocks();
  for (const [target, port] of [["sim-router-001", 22], [openWrtTarget, 0], [openWrtTarget, 65536]] as const) expect(() => openWrtObserve(target, port)).toThrow();
  expect(() => openWrtConnect(openWrtPin, "root; reboot")).toThrow(); expect(() => openWrtConnect("SHA256:bad", "root")).toThrow(); expect(invoke).not.toHaveBeenCalled();
});
it("rejects executable firmware claims, conflicting reports, unapproved and excessive configuration", async () => {
  const good = inspectedOpenWrt();
  vi.mocked(invoke).mockResolvedValueOnce(good); expect((await openWrtStatus()).inspection?.firmware.extraction.executable).toBe(false);
  const cases = [
    {...good, readApproved: false},
    {...good, inspection: {...good.inspection!, firmware: {...good.inspection!.firmware, extraction: {...good.inspection!.firmware.extraction, executable: true}}}},
    {...good, inspection: {...good.inspection!, firmware: {...good.inspection!.firmware, deviceIdentity: {...good.inspection!.firmware.deviceIdentity, model: "Other device"}}}},
    {...good, inspection: {...good.inspection!, networks: Array(65).fill(good.inspection!.networks[0])}},
    {...good, inspection: {...good.inspection!, networks: [{...good.inspection!.networks[0], password: "must never reach React"}]}},
  ];
  for (const invalid of cases) {vi.mocked(invoke).mockResolvedValueOnce(invalid); await expect(openWrtStatus()).rejects.toThrow();}
});
it("keeps safe adapter errors and replaces unstructured failures without leaking secrets", async () => {
  vi.mocked(invoke).mockRejectedValueOnce({code: "AUTHENTICATION_FAILED", message: "SSH key authentication failed.", operation: null, retryable: false, suggestedAction: "Review existing key access."});
  await expect(openWrtConnect(openWrtPin, "root")).rejects.toMatchObject({code: "AUTHENTICATION_FAILED"});
  vi.mocked(invoke).mockRejectedValueOnce("private path or remote secret"); await expect(openWrtInspect()).rejects.toThrow("OpenWrt inspection failed.");
});
it("preserves schema-one profiles and accepts only explicitly scoped schema-two unknown capabilities", () => {
  expect(profileSchema.parse(observedProfile).schemaVersion).toBe(1);
  const profile = {...observedProfile, schemaVersion: 2, inspectionOnly: true, manufacturer: "", hardwareRevisions: []};
  expect(profileSchema.parse(profile).inspectionOnly).toBe(true);
  for (const invalid of [{...profile, inspectionOnly: undefined}, {...profile, schemaVersion: 1}, {...profile, capabilities: {...profile.capabilities, dns: {...profile.capabilities.dns, assessment: "SUPPORTED"}}}]) expect(profileSchema.safeParse(invalid).success).toBe(false);
});
