import { desktopStateSchema,desktopPreferencesSchema,backupSchema,storagePlanSchema,importSchema } from "../types/library";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { z } from "zod";
import { acquisitionReportSchema, type AcquisitionRequest } from "../types/acquisition";
import { listen } from "@tauri-apps/api/event";
import { jobSchema, libraryPageSchema, detailsSchema, selectionSchema, activitySchema, manifestExportSchema, type LibraryQuery } from "../types/library";
import { compatibilitySchema, registryEntrySchema, registrySchema } from "../types/profiles";
import { adapterDescriptorSchema, adapterErrorSchema, adapterEvaluationSchema, type AdapterTargetKind } from "../types/adapters";
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

function requireDesktop() { if (!hasDesktopBackend()) throw new Error("This operation is available in the Windows app."); }
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
export async function discoverFirmwareRoutes(input: AcquisitionRequest) {
  requireDesktop();
  const report = acquisitionReportSchema.parse(await invoke("discover_firmware_routes", {input}));
  if (report.deviceId !== input.deviceId || report.fingerprint.deviceId !== input.deviceId) throw new Error("Route report belongs to another target.");
  return report;
}
export async function cancelFirmwareRouteDiscovery(deviceId:string) {requireDesktop();await invoke("cancel_firmware_route_discovery",{deviceId});}
export async function exportAcquisitionDiagnostics(deviceId:string) {requireDesktop();return z.boolean().parse(await invoke("export_acquisition_diagnostics",{deviceId}));}
export async function listAvailableAdapters() { requireDesktop(); return adapterDescriptorSchema.array().parse(await invoke("list_available_adapters")); }
export async function getDeviceAdapterMatches(deviceId: string, targetKind: AdapterTargetKind = "discovered") {
  requireDesktop();
  if (!/^[a-f0-9]{64}$/.test(deviceId)) throw new Error("Select a current Windows device.");
  let value: unknown;
  try { value = await invoke("get_device_adapter_matches", {input:{deviceId,targetKind}}); }
  catch (error) { const safe = adapterErrorSchema.safeParse(error); if (safe.success) throw safe.data; throw new Error("Adapter status is unavailable. Refresh devices and try again."); }
  const result=adapterEvaluationSchema.parse(value);
  if(result.deviceId!==deviceId)throw new Error("Adapter response belongs to another target. Refresh devices.");
  return result;
}
export async function recordDeviceObservation(deviceId: string, attribute: string, value: string) { requireDesktop(); await invoke("record_device_observation", {deviceId, attribute, value}); }
export async function selectFirmwareFile() {requireDesktop();return selectionSchema.nullable().parse(await invoke("select_firmware_file"));}
export async function discardFirmwareSelection(ticket:string) {requireDesktop();await invoke("discard_firmware_selection",{ticket});}
export async function startFirmwareImport(input:{ticket:string;description:string|null;associatedDeviceId:string|null}) {requireDesktop();return jobSchema.parse(await invoke("start_firmware_import",{input}));}
export async function listFirmwareArtifacts(query:LibraryQuery) {requireDesktop();return libraryPageSchema.parse(await invoke("list_firmware_artifacts",{query}));}
export async function getFirmwareArtifact(sha256:string) {requireDesktop();return detailsSchema.parse(await invoke("get_firmware_artifact",{sha256}));}
export async function listFirmwareImportHistory() {requireDesktop();return importSchema.array().max(100).parse(await invoke("list_firmware_import_history"));}
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

export async function getDesktopState(){requireDesktop();return desktopStateSchema.parse(await invoke("get_desktop_state"));}
export async function saveDesktopPreferences(input:Omit<import("../types/library").DesktopPreferences,"libraryDirectory">){requireDesktop();return desktopPreferencesSchema.parse(await invoke("save_desktop_preferences",{input}));}
export async function listFirmwareBackups(){requireDesktop();return backupSchema.array().max(100).parse(await invoke("list_firmware_backups"));}
export async function createFirmwareBackup(sha256:string){requireDesktop();return jobSchema.parse(await invoke("create_firmware_backup",{sha256}));}
export async function restoreFirmwareBackup(id:string){requireDesktop();return jobSchema.parse(await invoke("restore_firmware_backup",{id,confirmed:true}));}
export async function removeFirmwareItem(id:string,backup:boolean){requireDesktop();await invoke("remove_firmware_item",{id,backup,confirmed:true});}
export async function openManagedFolder(target:"library"|"backups"|"artifact",id:string|null=null){requireDesktop();await invoke("open_managed_folder",{target,id});}
export async function selectLibraryDirectory(){requireDesktop();return storagePlanSchema.nullable().parse(await invoke("select_library_directory"));}
export async function relocateFirmwareLibrary(ticket:string){requireDesktop();return jobSchema.parse(await invoke("relocate_firmware_library",{ticket,confirmed:true}));}
export async function recoverDefaultLibrary(){requireDesktop();await invoke("recover_default_library",{confirmed:true});}

export async function approveFirmwareRouteProbe(input:AcquisitionRequest) {requireDesktop();return z.string().regex(/^[a-f0-9]{32}$/).parse(await invoke("approve_firmware_route_probe",{input}));}

export async function approveStaticMenu(input:AcquisitionRequest) {requireDesktop();return z.string().regex(/^[a-f0-9]{32}$/).parse(await invoke("approve_static_menu",{input}));}
export async function readStaticMenu(input:AcquisitionRequest) {
  requireDesktop();const report=acquisitionReportSchema.parse(await invoke("read_static_menu",{input}));
  if(report.deviceId!==input.deviceId || report.fingerprint.deviceId!==input.deviceId)throw new Error("Menu report belongs to another target.");
  return report;
}
