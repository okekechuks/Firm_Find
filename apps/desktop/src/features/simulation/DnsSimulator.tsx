import { useEffect,useState } from "react";
import type { DnsConfiguration,SimulationAction,SimulationStatus } from "../../types/simulator";
import {dnsSchema} from "../../types/simulator";
type Props={status:SimulationStatus;busy:boolean;perform:(action:SimulationAction)=>Promise<void>};
const lists=[
 ["upstreamIpv4","upstreamSourceIpv4","Upstream IPv4"],
 ["upstreamIpv6","upstreamSourceIpv6","Upstream IPv6"],
 ["lanIpv4","lanSourceIpv4","LAN IPv4"],
 ["lanIpv6","lanSourceIpv6","LAN IPv6"],
] as const;
export function DnsSimulator({status,busy,perform}:Props){
 const dns=status.features?.dns.value??null;
 const [draft,setDraft]=useState<DnsConfiguration|null>(null);
 const [fields,setFields]=useState<Record<string,string>>({});
 const observed=JSON.stringify(dns);
 useEffect(()=>{const current=dnsSchema.nullable().parse(JSON.parse(observed));if(current){setDraft(current);setFields(Object.fromEntries(lists.map(([key])=>[key,current[key].join(", ")])));}},[observed]);
 const reset=()=>{setDraft(dns);if(dns)setFields(Object.fromEntries(lists.map(([key])=>[key,dns[key].join(", ")])));void perform({kind:"cancel_draft"});};
 const preview=status.preview;const outcome=status.outcome;
 return <section className="panel simulation-dns" aria-label="Simulated DNS">
  <h3>DNS simulation</h3>
  <p className="muted">Virtual configuration only. Resolver checks use modeled state, never internet requests.</p>
  {dns?<dl className="simulation-values" aria-label="Current simulated DNS">{lists.map(([key,,label])=><div key={key}><dt>{label}</dt><dd>{dns[key].join(", ")||"None"}</dd></div>)}<div><dt>DNS service</dt><dd>{status.features?.dnsService}</dd></div></dl>:<p>Authorize reads and select Read all features to inspect current DNS.</p>}
  {draft?<form onSubmit={e=>{e.preventDefault();void perform({kind:"preview_dns",configuration:{...draft,...Object.fromEntries(lists.map(([key])=>[key,(fields[key]??"").split(/[,\n]/).map(v=>v.trim()).filter(Boolean)]))}});}}>
   <div className="simulation-grid">{lists.map(([key,source,label])=><fieldset key={key} disabled={busy||!status.readAuthorized}>
    <legend>{label}</legend>
    <label>{label} mode<select aria-label={`${label} mode`} value={draft[source]} onChange={e=>setDraft({...draft,[source]:e.target.value as "manual"|"automatic"})}><option value="automatic">Automatic</option><option value="manual">Manual</option></select></label>
    <label>{label} resolvers<input aria-label={`${label} resolvers`} value={fields[key]??""} disabled={draft[source]!=="manual"} onChange={e=>setFields({...fields,[key]:e.target.value})}/></label>
   </fieldset>)}</div>
   <div className="simulation-actions"><button className="secondary-button" disabled={busy||!status.readAuthorized} type="submit">Preview DNS changes</button><button type="button" className="text-button" disabled={busy} onClick={reset}>Reset draft</button><button type="button" className="text-button" disabled={busy} onClick={reset}>Cancel draft</button></div>
  </form>:null}
  {preview&&!preview.id.startsWith("consumed-")?<section className="simulation-preview" aria-label="DNS change preview"><h4>Preview · revision {preview.baseRevision}</h4>
   <table className="support-table"><thead><tr><th>Setting</th><th>Before</th><th>Proposed</th></tr></thead><tbody>{preview.changes.map((change,i)=><tr key={`${change.field}-${i}`}><td>{change.field}</td><td>{change.before.join(", ")}</td><td>{change.after.join(", ")}</td></tr>)}</tbody></table>
   {!preview.changes.length?<p>No configuration differences.</p>:null}
   <p>Creates virtual configuration snapshot, reloads virtual DNS, then checks values, service and modeled resolution.</p>
   <button className="primary-button" disabled={busy||!status.readAuthorized} onClick={()=>void perform({kind:"apply_dns",previewId:preview.id})}>Apply DNS to Simulator</button>
  </section>:null}
  {outcome?<section className={`simulation-outcome ${outcome.verified?"verified":"unverified"}`} role="status" aria-label="Simulated DNS result"><h4>{outcome.verified?"Simulated DNS verified":"Simulated DNS not verified"}</h4><p>{outcome.message}</p>
   <dl className="simulation-values"><div><dt>Accepted</dt><dd>{String(outcome.accepted)}</dd></div><div><dt>Applied</dt><dd>{String(outcome.applied)}</dd></div><div><dt>Configuration matches</dt><dd>{String(outcome.configurationMatches)}</dd></div><div><dt>Service healthy</dt><dd>{String(outcome.serviceHealthy)}</dd></div><div><dt>Virtual resolution successful</dt><dd>{String(outcome.resolutionSuccessful)}</dd></div><div><dt>Rollback</dt><dd>{outcome.rollback}</dd></div></dl>
   {outcome.requiresReset?<p role="alert">Manual simulator reset required.</p>:null}
   <div className="simulation-actions"><button className="secondary-button" disabled={busy||!status.readAuthorized||!preview?.id.startsWith("consumed-")} onClick={()=>void perform({kind:"verify_dns"})}>Verify simulated DNS</button>
   <button className="secondary-button" disabled={busy||!status.readAuthorized||!outcome.snapshotId} onClick={()=>outcome.snapshotId&&void perform({kind:"restore",snapshotId:outcome.snapshotId})}>Restore previous virtual state</button></div>
  </section>:null}
 </section>;
}
