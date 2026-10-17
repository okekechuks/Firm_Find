import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { jobSchema, libraryPageSchema, detailsSchema, selectionSchema, activitySchema, manifestExportSchema, type LibraryQuery } from "../types/library";
import { compatibilitySchema, registryEntrySchema, registrySchema } from "../types/profiles";
import {
  settingsSchema,
  endpointResultSchema, savedRouterSchema, type ManualRouterInput,
  discoverySchema,
  snapshotSchema,
  type Settings,
} from "../types/contracts";

export function hasDesktopBackend(): boolean {
  return isTauri();
}

export async function getSnapshot() {
  if (!hasDesktopBackend())
    throw new Error(
      "Desktop backend unavailable. Launch OBTAINER with npm run tauri dev to connect to SQLite.",
    );
  return snapshotSchema.parse(await invoke("get_snapshot"));
}

export async function saveSettings(settings: Settings) {
  if (!hasDesktopBackend())
    throw new Error(
      "Preferences can only be saved in the Windows desktop app.",
    );
  return settingsSchema.parse(
    await invoke("save_settings", { settings: settingsSchema.parse(settings) }),
  );
}

export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  )
    return error.message;
  return "OBTAINER could not complete this request. Try again.";
}

export async function getDiscovery() {
  if (!hasDesktopBackend()) throw new Error("Device inventory is available in the Windows app.");
  return discoverySchema.parse(await invoke("get_discovery"));
}

export async function refreshDiscovery() {
  if (!hasDesktopBackend()) throw new Error("Device inventory is available in the Windows app.");
  return discoverySchema.parse(await invoke("refresh_discovery"));
}

export async function onDiscoveryChanged(changed: () => void) {
  if (!hasDesktopBackend()) throw new Error("Live device updates require the Windows app.");
  return listen<number>("discovery-changed", (event) => {
    if (Number.isSafeInteger(event.payload) && event.payload >= 0) changed();
  });
}

function requireDesktop() { if (!hasDesktopBackend()) throw new Error("Saved router checks require the Windows app."); }
export async function getManualRouters() {
  requireDesktop(); return savedRouterSchema.array().parse(await invoke("get_manual_routers"));
}
export async function addManualRouter(input: ManualRouterInput) {
  requireDesktop(); return savedRouterSchema.array().parse(await invoke("add_manual_router", {input}));
}
export async function removeManualRouter(id: string) {
  requireDesktop(); return savedRouterSchema.array().parse(await invoke("remove_manual_router", {id}));
}
export async function probeManagementEndpoint(id: string, authorized: boolean) {
  requireDesktop(); return endpointResultSchema.parse(await invoke("probe_management_endpoint", {id, authorized}));
}
export async function cancelEndpointProbe(id: string) { requireDesktop(); await invoke("cancel_endpoint_probe", {id}); }
export async function cancelDiscovery() { requireDesktop(); await invoke("cancel_discovery"); }
export async function getProfileRegistry() { requireDesktop(); return registrySchema.parse(await invoke("get_profile_registry")); }
export async function getDeviceCompatibility(deviceId: string) { requireDesktop(); return compatibilitySchema.parse(await invoke("get_device_compatibility", {deviceId})); }
export async function recordDeviceObservation(deviceId: string, attribute: string, value: string) { requireDesktop(); await invoke("record_device_observation", {deviceId, attribute, value}); }
export async function selectFirmwareFile() {requireDesktop();return selectionSchema.nullable().parse(await invoke("select_firmware_file"));}
export async function discardFirmwareSelection(ticket:string) {requireDesktop();await invoke("discard_firmware_selection",{ticket});}
export async function startFirmwareImport(input:{ticket:string;description:string|null;associatedDeviceId:string|null}) {requireDesktop();return jobSchema.parse(await invoke("start_firmware_import",{input}));}
export async function listFirmwareArtifacts(query:LibraryQuery) {requireDesktop();return libraryPageSchema.parse(await invoke("list_firmware_artifacts",{query}));}
export async function getFirmwareArtifact(sha256:string) {requireDesktop();return detailsSchema.parse(await invoke("get_firmware_artifact",{sha256}));}
export async function verifyFirmwareIntegrity(sha256:string) {requireDesktop();return jobSchema.parse(await invoke("verify_firmware_integrity",{sha256}));}
export async function exportArtifactManifest(sha256:string) {requireDesktop();return manifestExportSchema.parse(await invoke("export_artifact_manifest",{sha256}));}
export async function listLibraryJobs() {requireDesktop();return jobSchema.array().max(100).parse(await invoke("list_library_jobs"));}
export async function getLibraryActivity() {requireDesktop();return activitySchema.array().max(100).parse(await invoke("get_library_activity"));}
export async function cancelLibraryJob(id:string) {requireDesktop();return jobSchema.parse(await invoke("cancel_library_job",{id}));}
export async function onLibraryChanged(changed:(job:import("../types/library").JobRecord)=>void) {requireDesktop();return listen("library-job-changed",(e)=>{const result=jobSchema.safeParse(e.payload);if(result.success)changed(result.data);});}
export async function validateDeviceProfile(document: string) {
  requireDesktop();
  if (new TextEncoder().encode(document).length > 65536) throw new Error("Profile exceeds 64 KiB.");
  return registryEntrySchema.parse(await invoke("validate_device_profile", {document}));
}
