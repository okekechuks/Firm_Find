import { beforeEach,expect,it,vi } from "vitest";
import { invoke,isTauri } from "@tauri-apps/api/core";
import { getDeviceAdapterMatches,listAvailableAdapters } from "../lib/backend";
import { evaluation,descriptor } from "./adapterFixtures";
vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn(),isTauri:vi.fn()}));
const id="a".repeat(64);
beforeEach(()=>{vi.mocked(isTauri).mockReturnValue(true);});
it("validates adapter IDs before IPC and calls only typed passive query",async()=>{
  await expect(getDeviceAdapterMatches("../unsafe")).rejects.toThrow();expect(invoke).not.toHaveBeenCalled();
  vi.mocked(invoke).mockResolvedValue(evaluation(id));await getDeviceAdapterMatches(id);
  expect(invoke).toHaveBeenCalledWith("get_device_adapter_matches",{input:{deviceId:id,targetKind:"discovered"}});
});
it("rejects writable, mismatched, simulated or connected passive responses",async()=>{
  for(const mutate of ["write","connected","simulated","wrong"]){
    const r=evaluation(id);
    if(mutate==="write")Object.assign(r.matches[0].features.dns,{canWrite:true});
    if(mutate==="connected")r.connectionState="connected_read_only";
    if(mutate==="simulated")r.matches[0].adapter.implementationStatus="test_only";
    if(mutate==="wrong")r.deviceId="b".repeat(64);
    vi.mocked(invoke).mockResolvedValue(r);await expect(getDeviceAdapterMatches(id)).rejects.toThrow();
  }
});
it("validates registry descriptor responses and rejects test adapter leakage",async()=>{
  vi.mocked(invoke).mockResolvedValue([descriptor()]);expect(await listAvailableAdapters()).toHaveLength(1);
  vi.mocked(invoke).mockResolvedValue([{...descriptor(),implementationStatus:"test_only"}]);await expect(listAvailableAdapters()).rejects.toThrow();
});
it("returns typed actionable backend errors and redacts unstructured errors",async()=>{
  const error={code:"PROTECTED_DEVICE",message:"Host adapter is protected.",operation:null,retryable:false,suggestedAction:"Select a router."};
  vi.mocked(invoke).mockRejectedValue(error);await expect(getDeviceAdapterMatches(id)).rejects.toEqual(error);
  vi.mocked(invoke).mockRejectedValue("raw secret transport output");await expect(getDeviceAdapterMatches(id)).rejects.toThrow("Adapter status is unavailable");
});
it("browser preview never calls adapter IPC",async()=>{
  vi.mocked(isTauri).mockReturnValue(false);await expect(getDeviceAdapterMatches(id)).rejects.toThrow("Windows app");expect(invoke).not.toHaveBeenCalled();
});
