import { invoke, isTauri } from "@tauri-apps/api/core";
import { adapterErrorSchema } from "../types/adapters";
import { openWrtConnectSchema, openWrtObserveSchema, openWrtStatusSchema, type OpenWrtStatus } from "../types/openwrt";

async function call(command: string, request?: unknown): Promise<OpenWrtStatus> {
  if (!isTauri()) throw new Error("OpenWrt inspection requires the Windows desktop app.");
  let response: unknown;
  try { response = await invoke(command, request === undefined ? undefined : {request}); }
  catch (error) {
    const parsed = adapterErrorSchema.safeParse(error);
    if (parsed.success) throw parsed.data;
    throw new Error("OpenWrt inspection failed. Review the selected target and try again.");
  }
  return openWrtStatusSchema.parse(response);
}
export function openWrtStatus() { return call("openwrt_status"); }
export function openWrtObserve(targetId: string, port: number) {
  return call("openwrt_observe", openWrtObserveSchema.parse({targetId, port, approved: true})).then(status => {
    if (status.targetId !== targetId) throw new Error("SSH observation belongs to another target. Disconnect and try again.");
    return status;
  });
}
export function openWrtConnect(fingerprint: string, username: string) {
  return call("openwrt_connect", openWrtConnectSchema.parse({fingerprint, username, approved: true}));
}
export function openWrtApproveReads() { return call("openwrt_approve_reads", {approved: true}); }
export function openWrtInspect() { return call("openwrt_inspect"); }
export function openWrtDisconnect() { return call("openwrt_disconnect"); }
export function openWrtCancel() { return call("openwrt_cancel"); }
