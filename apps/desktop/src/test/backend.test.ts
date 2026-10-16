import { beforeEach, expect, it, vi } from "vitest";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getSnapshot, saveSettings, getManualRouters, probeManagementEndpoint, getDeviceCompatibility, getProfileRegistry, validateDeviceProfile } from "../lib/backend";
import { report } from "./profileFixtures";
import type { Settings } from "../types/contracts";
import observedProfile from "../../../../device-profiles/observed-comfi-mg11.json";
import { profileSchema } from "../types/profiles";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: vi.fn() }));
beforeEach(() => {
  vi.mocked(isTauri).mockReturnValue(true);
});

it("rejects malformed native responses", async () => {
  vi.mocked(invoke).mockResolvedValue({
    version: "0.1.0",
    counts: { devices: -1 },
  });
  await expect(getSnapshot()).rejects.toThrow();
});

it("rejects invalid preferences before invoking Rust", async () => {
  await expect(
    saveSettings({ theme: "invalid" } as unknown as Settings),
  ).rejects.toThrow();
  expect(invoke).not.toHaveBeenCalled();
});

it("never calls IPC from browser preview", async () => {
  vi.mocked(isTauri).mockReturnValue(false);
  await expect(getSnapshot()).rejects.toThrow("Desktop backend unavailable");
  expect(invoke).not.toHaveBeenCalled();
});

it("rejects malformed saved targets and endpoint responses", async () => {
  vi.mocked(invoke).mockResolvedValue([{id: "bad", address: "http://192.168.1.1/"}]);
  await expect(getManualRouters()).rejects.toThrow();
  vi.mocked(invoke).mockResolvedValue({code: "identity_unknown", status: "200"});
  await expect(probeManagementEndpoint("a".repeat(64), true)).rejects.toThrow();
  expect(invoke).toHaveBeenLastCalledWith("probe_management_endpoint", {id: "a".repeat(64), authorized: true});
});

it("rejects malformed compatibility responses and backend claims of writable adapters", async () => {
  vi.mocked(invoke).mockResolvedValue({...report(),features:{...report().features,dns:{...report().features.dns,canApply:true}}});
  await expect(getDeviceCompatibility("fixture")).rejects.toThrow();
  vi.mocked(invoke).mockResolvedValue({profiles:[{profile:{schemaVersion:99}}],digest:"bad",diagnostics:[]});await expect(getProfileRegistry()).rejects.toThrow();
});
it("rejects oversized profile documents before IPC", async () => {
  await expect(validateDeviceProfile("x".repeat(65537))).rejects.toThrow("64 KiB");expect(invoke).not.toHaveBeenCalled();
});

it("accepts legacy numeric and explicit opaque scheme profiles without granting trust", async () => {
  const legacy = profileSchema.parse(observedProfile);
  expect(legacy.firmwareVersionConstraints.scheme).toBeUndefined();
  const opaque = {...legacy,firmwareVersionConstraints:{scheme:"opaque_exact" as const,exact:["MF69_AM802L_ZX_DD_N_241106_EU"],minInclusive:null,maxExclusive:null}};
  const entry = {profile:opaque,origin:"local_unverified",authoritative:false,contentHash:"fixture"};
  vi.mocked(invoke).mockResolvedValue(entry);
  const document=JSON.stringify(opaque);
  const result=await validateDeviceProfile(document);
  expect(result.profile.firmwareVersionConstraints.scheme).toBe("opaque_exact");
  expect(result.authoritative).toBe(false);
  expect(invoke).toHaveBeenCalledWith("validate_device_profile",{document});
});
it("rejects unknown firmware scheme values in native profile responses", async () => {
  const malformed={...observedProfile,firmwareVersionConstraints:{...observedProfile.firmwareVersionConstraints,scheme:"guess_dates"}};
  vi.mocked(invoke).mockResolvedValue({profile:malformed,origin:"local_unverified",authoritative:false,contentHash:"fixture"});
  await expect(validateDeviceProfile("{}" )).rejects.toThrow();
});
