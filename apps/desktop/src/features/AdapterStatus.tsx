import { useEffect, useState } from "react";
import { getDeviceAdapterMatches } from "../lib/backend";
import { adapterErrorSchema, type AdapterEvaluation, type AdapterTargetKind } from "../types/adapters";

const human=(value:string)=>value.replaceAll("_"," ");
type Load={key:string;value?:AdapterEvaluation;error?:string;expired?:boolean};
export function AdapterStatus({deviceId,revision,desktop,targetKind="discovered"}:{
  deviceId:string;revision:number|null;desktop:boolean;targetKind?:AdapterTargetKind;
}) {
  const [attempt,setAttempt]=useState(0);
  const [load,setLoad]=useState<Load|null>(null);
  const key=JSON.stringify([deviceId,revision,targetKind,attempt]);
  useEffect(()=>{
    if(!desktop)return;
    let active=true;
    let timer:ReturnType<typeof setTimeout>|undefined;
    void getDeviceAdapterMatches(deviceId,targetKind).then(value=>{
      if(!active)return;
      if(revision!==null&&value.discoveryRevision!==revision){setLoad({key,error:"Discovery changed. Re-evaluate current adapter status."});return;}
      if(value.expiresAt<=Date.now()){setLoad({key,expired:true});return;}
      setLoad({key,value});
      timer=setTimeout(()=>{if(active)setLoad({key,expired:true});},Math.max(0,value.expiresAt-Date.now()));
    }).catch(error=>{
      if(!active)return;
      const safe=adapterErrorSchema.safeParse(error);
      setLoad({key,error:safe.success?safe.data.message+" "+safe.data.suggestedAction:"Adapter status could not be loaded. Refresh devices and try again."});
    });
    return()=>{active=false;if(timer)clearTimeout(timer);};
  },[deviceId,revision,targetKind,desktop,key]);
  const current=load?.key===key?load:null;
  const value=current?.value;
  return <section className="adapter-summary" aria-label="Device adapter status">
    <div className="section-heading"><h3>Device adapter</h3>{desktop?<button className="secondary-button" onClick={()=>setAttempt(n=>n+1)} disabled={!current}>Re-evaluate adapter</button>:null}</div>
    {!desktop?<p>Open the Windows app to inspect adapter availability.</p>:!current?<p role="status">Checking adapter requirements…</p>:current.error?<p role="alert">{current.error}</p>:current.expired?<p role="status">Adapter evaluation expired. Re-evaluate current evidence.</p>:null}
    {value?<><p><strong>{value.protectedDevice?"Protected hardware":value.selection==="no_match"?"No compatible adapter":value.selection==="ambiguous"?"Ambiguous adapter match":value.selectedAdapter?"Exact adapter candidate":"Verification required"}</strong></p>
      <p className="muted small">{value.reason}</p>
      <p className="muted small">Management connection: {human(value.connectionState)}. Physical device changes and firmware extraction remain unavailable.</p>
      {value.matches.map(match=><details className="feature-explanation" key={match.adapter.id}><summary>{match.adapter.name} · {human(match.status)}</summary>
        <p>{match.reason}</p>
        <p className="muted small">{match.adapter.implementationStatus==="declared"?"Adapter contract only; physical read-only transport is not implemented.":human(match.adapter.implementationStatus)}</p>
        <p className="muted small">Transport: {match.adapter.transports.join(", ")}. Authentication required: {human(match.adapter.authentication)}.</p>
        {match.missingEvidence.length?<p className="muted small">Required: {match.missingEvidence.join(", ")}.</p>:null}
        <p className="muted small">Dependencies: {match.adapter.dependencies.join(", ")||"None declared"}.</p>
        <ul>{match.capabilities.map(cap=><li key={cap.operation}>{human(cap.operation)}: {cap.canRead?"Available in an authenticated session":"Unavailable"} — {cap.reason}</li>)}</ul>
        <p className="muted small">{match.adapter.limitations.join(" ")}</p>
      </details>)}</>:null}
  </section>;
}
