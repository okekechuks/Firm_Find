import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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
