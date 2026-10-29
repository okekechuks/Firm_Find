import type { WebEvidenceReport } from "../types/web-management";

// Synthetic local evidence contract. Never physical qualification evidence.
export function webEvidenceFixture(deviceId = "a".repeat(64)): WebEvidenceReport {
  return {
    schemaVersion: 1, deviceId, entries: [], authCandidates: [], resources: [], counts: {}, warnings: [], canAcquire: false, retention: "memory_only",
    strategy: {strategy:"web_management_acquisition",canAcquire:false,sources:[],missingRequirements:["Current independently qualified device identity","Independently qualified read-only firmware source descriptor"]},
    actions: { importHar: true, analyzeLocalBundle: true, collectResources: false, authenticate: false, reason: "No independently qualified read-only identity/status route is registered for this device family." },
  };
}
