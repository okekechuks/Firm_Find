import type { AdapterDescriptor, AdapterEvaluation } from "../types/adapters";
export function descriptor():AdapterDescriptor{return {
  id:"openwrt-readonly-contract",name:"OpenWrt read-only contract",version:"0.1.0",implementationStatus:"declared",
  transports:["network"],authentication:"ssh_key",operations:["authorized_identity_probe","inspect_capabilities","read_configuration"],
  dependencies:["Authenticated OpenWrt transport"],limitations:["No physical operations in Phase 5."],profileIds:[],firmwareFamilies:["OpenWrt"],
};}
const feature=()=>({assessment:"UNKNOWN" as const,readiness:["REQUIRES_MANUAL_VERIFICATION" as const,"NOT_IMPLEMENTED" as const],
  canRead:false as const,canWrite:false as const,canVerify:false as const,canRollback:false as const,evidenceSources:[],reason:"Exact hardware and firmware are unverified."});
export function evaluation(deviceId="a".repeat(64),revision=1):AdapterEvaluation{return {
  deviceId,discoveryRevision:revision,protectedDevice:false,selection:"no_match",selectedAdapter:null,connectionState:"identity_verification_required",
  reason:"No compatible adapter is available from current evidence.",evaluatedAt:Date.now(),expiresAt:Date.now()+300_000,
  matches:[{adapter:descriptor(),status:"not_applicable",confidence:0,reason:"No verified compatible OpenWrt identity.",
    matchedEvidence:[],missingEvidence:["verified hardware revision","authenticated endpoint handshake"],
    capabilities:[{operation:"read_configuration",exists:false,canRead:false,canPlan:false,canApply:false,canVerify:false,canRestore:false,authenticationRequired:true,reason:"Operation is not implemented."}],
    features:{dns:feature(),wifiUserLimit:feature(),apn:feature(),ussd:feature()}}],
};}
