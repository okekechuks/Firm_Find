// Synthetic frontend transport fixtures. Never included in the application registry.
import type { CompatibilityReport } from "../types/profiles";
export function report(deviceId = "fixture-device", discoveryRevision = 1): CompatibilityReport {
  const feature = {assessment:"UNKNOWN" as const, readiness:["REQUIRES_MANUAL_VERIFICATION" as const,"NOT_IMPLEMENTED" as const], quality:"insufficient",reason:"Exact model and revision unverified.",nextAction:"Obtain trusted identity evidence.",evidenceSources:[],subCapabilities:{manual:{assessment:"UNKNOWN" as const,reason:"No modem evidence.",evidenceSources:[]}},verifiedRange:null,adapterImplemented:false as const,canApply:false as const};
  return {deviceId,discoveryRevision,evaluatedAt:Date.now(),expiresAt:Date.now()+300000,engineVersion:"1.0.0",registryDigest:"fixture-registry",inputDigest:"fixture-input",confidence:"CANDIDATE",selectedProfile:null,profileVersion:null,profileContentHash:null,candidates:[],evidence:[],conflicts:[],warnings:[],change:null,features:{dns:{...feature},wifiUserLimit:{...feature},apn:{...feature},ussd:{...feature}}};
}
