import { z } from "zod";
import { authenticationMethodSchema, connectionStateSchema } from "./adapters";
const text=z.string().min(1).max(2048);
const uint=z.number().int().nonnegative().safe();
export const scenarioSchema=z.enum(["healthy","authentication_failure","connection_timeout","slow_authentication","slow_read","wrong_identity","endpoint_substitution","router_disconnected","dns_apply_failure","dns_verification_mismatch","dns_service_failure","resolution_failure","rollback_failure","external_revision","wifi_reload_interruption","wifi_capacity","sim_missing","sim_pin_required","unknown_operator","missing_apn","modem_absent","registration_failure","data_connection_failure","cellular_restart","ipv4_only","unsupported_modem","ussd_timeout","ussd_network_failure"]);
export const sourceSchema=z.enum(["automatic","manual","unknown"]);
export const ipFamilySchema=z.enum(["ipv4","ipv6","dual_stack","unknown"]);
export const apnAuthenticationSchema=z.enum(["none","pap","chap","pap_or_chap","unknown"]);
export const dnsSchema=z.object({upstreamIpv4:z.array(text).max(4),upstreamIpv6:z.array(text).max(4),lanIpv4:z.array(text).max(4),lanIpv6:z.array(text).max(4),
 upstreamSourceIpv4:sourceSchema,upstreamSourceIpv6:sourceSchema,lanSourceIpv4:sourceSchema,lanSourceIpv6:sourceSchema,observedBy:text}).strict();
export const simulationErrorSchema=z.object({code:z.enum(["DISABLED","NOT_FOUND","UNAUTHORIZED","AUTHENTICATION_FAILED","ENDPOINT_UNTRUSTED","IDENTITY_CONFLICT","EXPIRED","STALE","BUSY","CANCELLED","TIMEOUT","OFFLINE","UNSUPPORTED","INVALID_INPUT","APPLY_FAILED","VERIFICATION_FAILED","RESTORE_FAILED","RECOVERY_REQUIRED","SIM_MISSING","SIM_PIN_REQUIRED","MODEM_ABSENT","UNKNOWN_OPERATOR","MISSING_APN","REGISTRATION_FAILED","DATA_CONNECTION_FAILED","INTERNAL"]),message:text,retryable:z.boolean()}).strict();
const endpoint=z.string().regex(/^simulator:\/\/[a-zA-Z0-9-]+\/sim-router-001$/);
const device=z.object({id:z.literal("sim-router-001"),manufacturer:z.literal("OBTAINER"),model:z.enum(["Virtual Router Lab","Wrong Virtual Device"]),hardwareRevision:z.literal("SIM-1"),firmwareFamily:z.literal("Simulated OpenWrt-like"),firmwareVersion:z.literal("SIM-0.1"),operatingSystem:z.literal("Simulated Embedded Linux"),endpoint,interfaceId:z.literal("sim-interface-001"),peerPin:z.string().startsWith("SIMULATOR-ONLY:"),simulated:z.literal(true)}).strict();
const wifi=z.object({radio:text,ssid:text.nullable(),bssId:text.nullable(),stationLimit:uint.nullable(),verifiedMinimum:uint.nullable(),verifiedMaximum:uint.nullable(),connectedClients:uint.nullable(),observedBy:text}).strict();
const apn=z.object({source:sourceSchema,apn:text.nullable(),operator:text.nullable(),authenticationRequired:z.boolean(),ipFamily:ipFamilySchema,authenticationMethod:apnAuthenticationSchema,observedBy:text}).strict();
const ussdCapability=z.object({modemInterface:text.nullable(),assessment:z.enum(["SUPPORTED","UNSUPPORTED","UNKNOWN"]),authenticationRequired:z.boolean(),interactiveSession:z.boolean(),restrictions:z.array(text)}).strict();
const modem=z.object({present:z.boolean(),simPresent:z.boolean(),pinRequired:z.boolean(),registered:z.boolean(),operator:text.nullable(),dataConnected:z.boolean(),ipv6Supported:z.boolean(),service:text}).strict();
const read=<T extends z.ZodType>(schema:T)=>z.object({value:schema.nullable(),error:simulationErrorSchema.nullable()}).strict().refine(r=>(r.value===null)!==(r.error===null),"Exactly one value or error required.");
const preview=z.object({id:text,target:z.literal("sim-router-001"),epoch:uint,baseRevision:uint,previous:dnsSchema,proposed:dnsSchema,changes:z.array(z.object({field:text,before:z.array(text),after:z.array(text)}).strict()),affectedServices:z.array(text),snapshotRequired:z.literal(true)}).strict();
const outcome=z.object({operation:z.enum(["apply","verify","restore"]),accepted:z.boolean(),applied:z.boolean(),configurationMatches:z.boolean(),serviceHealthy:z.boolean(),resolutionSuccessful:z.boolean(),verified:z.boolean(),rollback:z.enum(["not_requested","succeeded","failed","cancelled","unavailable"]),requiresReset:z.boolean(),message:text,snapshotId:text.nullable(),revision:uint}).strict().refine(o=>!o.verified||(o.applied&&o.configurationMatches&&o.serviceHealthy&&o.resolutionSuccessful),"Verification needs actual configuration, service and resolver success.");
export const simulationStatusSchema=z.object({activeConnections:uint.max(2),available:z.literal(true),active:z.boolean(),device:device.nullable(),epoch:uint,revision:uint,virtualTime:uint,scenario:scenarioSchema,scenarios:z.array(scenarioSchema),
 faults:z.array(z.object({stage:z.enum(["connect","authenticate","read","apply","verify","resolve","restore","wifi","ussd"]),remaining:uint.max(255),repeat:z.boolean()}).strict()),connection:connectionStateSchema,authentication:z.enum(["not_attempted","required","authenticating","authenticated","failed","expired"]),approvalId:text.nullable(),readAuthorized:z.boolean(),
 features:z.object({dnsService:text,wifiService:text,dns:read(dnsSchema),wifi:read(z.array(wifi)),apn:read(apn),ussd:read(ussdCapability),modem,observedRevision:uint,observedAt:uint}).strict().nullable(),preview:preview.nullable(),outcome:outcome.nullable(),snapshots:z.array(z.object({scenario:scenarioSchema,id:text,target:z.literal("sim-router-001"),firmwareVersion:z.literal("SIM-0.1"),revision:uint,virtualTime:uint}).strict()).max(16),ussd:z.object({id:text,state:z.enum(["active","completed","cancelled","timed_out","failed"]),text,expiresAt:uint}).strict().nullable(),events:z.array(z.object({time:uint,message:text}).strict()).max(100),busy:z.boolean()}).strict().refine(s=>s.active===(s.device!==null),"Active runtime must own its virtual device.")
 .refine(s=>!s.device||s.device.model===(s.scenario==="wrong_identity"?"Wrong Virtual Device":"Virtual Router Lab"),"Virtual summary must match its scenario.")
 .refine(s=>s.scenario!=="wrong_identity"||(!s.readAuthorized&&s.activeConnections===0&&s.connection!=="connected_read_only"),"Conflicting virtual identity cannot have an authenticated session.");
export const simulationActionSchema=z.discriminatedUnion("kind",[
 z.object({kind:z.literal("approve_handshake")}).strict(),z.object({kind:z.literal("connect"),approvalId:text,credential:z.string().min(1).max(4096)}).strict(),
 ...(["approve_reads","read_features","disconnect","cancel_draft","verify_dns"] as const).map(kind=>z.object({kind:z.literal(kind)}).strict()),
 z.object({kind:z.literal("preview_dns"),configuration:dnsSchema}).strict(),z.object({kind:z.literal("apply_dns"),previewId:text}).strict(),z.object({kind:z.literal("restore"),snapshotId:text}).strict(),
 z.object({kind:z.literal("set_wifi"),radio:text,limit:uint}).strict(),z.object({kind:z.literal("set_apn"),source:sourceSchema,apn:z.string().max(100),authentication:apnAuthenticationSchema,ipFamily:ipFamilySchema}).strict(),
 z.object({kind:z.literal("ussd_start"),code:z.string().min(1).max(64)}).strict(),z.object({kind:z.literal("ussd_reply"),sessionId:text,text:z.string().min(1).max(32)}).strict(),z.object({kind:z.literal("ussd_cancel"),sessionId:text}).strict(),
]);
export const simulationRequestSchema=z.object({target:z.literal("sim-router-001"),endpoint,epoch:uint,revision:uint,action:simulationActionSchema}).strict();
export const simulationControlSchema=z.discriminatedUnion("kind",[
 z.object({kind:z.literal("start")}).strict(),z.object({kind:z.literal("stop")}).strict(),z.object({kind:z.literal("reset")}).strict(),z.object({kind:z.literal("cancel")}).strict(),
 z.object({kind:z.literal("scenario"),scenario:scenarioSchema,repeat:z.boolean()}).strict(),z.object({kind:z.literal("advance"),milliseconds:uint.max(300000)}).strict(),
]);
export type SimulationStatus=z.infer<typeof simulationStatusSchema>;
export type SimulationAction=z.infer<typeof simulationActionSchema>;
export type SimulationControl=z.infer<typeof simulationControlSchema>;
export type DnsConfiguration=z.infer<typeof dnsSchema>;
export type Scenario=z.infer<typeof scenarioSchema>;
// Reuse the established authentication vocabulary; no new frontend authority flag.
export const simulationAuthentication=authenticationMethodSchema;
