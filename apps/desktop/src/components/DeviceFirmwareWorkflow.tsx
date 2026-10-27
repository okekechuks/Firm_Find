import { useCallback, useEffect, useRef, useState } from "react";
import { cancelLibraryJob, errorMessage, getDeviceWorkflow, startDeviceFirmwareAcquisition, startFirmwareAnalysis } from "../lib/backend";
import { useLibraryFeed } from "../lib/useLibraryFeed";
import { useWorkspace } from "../lib/workspace-context";
import type { DeviceWorkflow } from "../types/workflow";
import { FirmwareExtraction } from "./FirmwareExtraction";
const human=(s:string)=>s.replaceAll("_"," ");
export function DeviceFirmwareWorkflow({deviceId,revision,connected}:{deviceId:string;revision:number;connected:boolean}) {
  const [workflow,setWorkflow]=useState<DeviceWorkflow|null>(null);
  const [error,setError]=useState("");const [busy,setBusy]=useState(false);const [confirmed,setConfirmed]=useState(false);
  const [selected,setSelected]=useState("");const [refresh,setRefresh]=useState(0);
  const mounted=useRef(false);const generation=useRef(0);const autoAnalysis=useRef(new Set<string>());
  const workspace=useWorkspace();
  const refreshWorkspace=workspace?.refresh;
  const feed=useLibraryFeed(true,workflow?.jobs.some(j=>["running","pending"].includes(j.status))??false);
  const load=useCallback(async()=>{
    const run=++generation.current;
    try {const result=await getDeviceWorkflow(deviceId);if(mounted.current&&run===generation.current){setWorkflow(result);setError("");}}
    catch(e){if(mounted.current&&run===generation.current){setError(errorMessage(e));setWorkflow(null);}}
  },[deviceId]);
  useEffect(()=>{const requests=generation;mounted.current=true;return()=>{mounted.current=false;requests.current++;};},[]);
  useEffect(()=>{setConfirmed(false);setWorkflow(null);void load();},[load,revision,connected,feed,refresh]);
  const run=useCallback(async(operation:()=>Promise<unknown>)=>{
    setBusy(true);setError("");
    try {await operation();if(mounted.current){setRefresh(r=>r+1);await refreshWorkspace?.();}}
    catch(e){if(mounted.current)setError(errorMessage(e));}
    finally{if(mounted.current)setBusy(false);}
  },[refreshWorkspace]);
  const artifact=workflow?.artifacts.find(a=>a.artifact.sha256===selected)??workflow?.artifacts[0];
  // Resume one bounded local analysis after acquisition/restart. Failed or
  // cancelled jobs need an explicit retry. Extraction is always user initiated.
  useEffect(()=>{
    if(!workflow)return;
    const next=workflow.artifacts.find(a=>a.canAnalyze&&!a.analysis&&!workflow.jobs.some(j=>j.kind==="firmware_analysis"&&j.relatedArtifact===a.artifact.sha256)&&!autoAnalysis.current.has(a.artifact.sha256));
    if(next){autoAnalysis.current.add(next.artifact.sha256);void run(()=>startFirmwareAnalysis(next.artifact.sha256));}
  },[workflow,run]);
  const live=Boolean(workflow&&connected&&workflow.binding.revision===revision&&workflow.binding.expiresAt>Date.now());
  useEffect(()=>{
    if(!workflow||workflow.binding.expiresAt<=Date.now())return;
    const timer=setTimeout(()=>{setConfirmed(false);setRefresh(r=>r+1);},workflow.binding.expiresAt-Date.now());return()=>clearTimeout(timer);
  },[workflow]);
  const source=workflow?.officialSources[0];const acquisition=workflow?.stages.find(s=>s.id==="acquisition");
  const active=workflow?.jobs.filter(j=>["running","pending"].includes(j.status))??[];
  return <section className="device-firmware-workflow" aria-label="Device firmware workflow">
    <h3>Firmware workflow</h3><p className="workflow-path">Device → Identity → Obtain → Analyze → Extract → Customize → Rebuild</p>
    {error?<p role="alert">{error}</p>:null}
    {!workflow&&!error?<p role="status">Loading device workflow…</p>:null}
    {!workflow?<button className="primary-button" disabled>Obtain Firmware</button>:null}
    {workflow?<>
      <p>Identity: {workflow.identity.confidence}. {workflow.identity.verifiedManufacturer??"Manufacturer unknown"} · {workflow.identity.verifiedModel??"Model unknown"} · Revision {workflow.identity.verifiedHardwareRevision??"unknown"}</p>
      {workflow.identity.reportedModel?<p>User-reported model: {workflow.identity.reportedModel} (unverified).</p>:null}
      <details><summary>Identity evidence</summary><p>Profile: {workflow.selectedProfile??"No exact profile"}. Adapter: {workflow.adapter??"No qualified device acquisition adapter"}.</p><ul>{workflow.identity.evidence.map(e=><li key={e.id}>{human(e.attribute)}: {e.value} — {human(e.verification)}; {human(e.source)}</li>)}</ul></details>
      {!source?<p className="inline-notice">{acquisition?.reason} Next: qualify exact manufacturer, model, hardware revision and an authorized firmware source/export method.</p>:<div className="workflow-source"><h4>Official firmware image</h4><p>{source.vendor} {source.model} {source.revision} · OpenWrt {source.version}</p><p>{source.filename}</p><p>This is an official target image, not a copy of firmware installed on your device. Installation remains unqualified.</p><label><input type="checkbox" checked={confirmed} disabled={!live||busy||active.length>0} onChange={e=>setConfirmed(e.target.checked)}/>Approve this exact target and public HTTPS source.</label></div>}
      <button className="primary-button" disabled={!live||!source||!acquisition?.executable||!confirmed||busy||active.length>0} onClick={()=>void run(()=>startDeviceFirmwareAcquisition(workflow.binding,source!.id,confirmed))}>Obtain Firmware</button>
      {active.map(j=><div key={j.id} role="status"><p>{human(j.stage)}</p><progress aria-label={`${human(j.kind)} progress`} max={Math.max(j.totalBytes,1)} value={j.progressBytes}/><button className="secondary-button" disabled={busy} onClick={()=>void run(()=>cancelLibraryJob(j.id))}>Cancel {human(j.kind)}</button></div>)}
      {workflow.jobs.filter(j=>j.status==="failed"||j.status==="cancelled").slice(0,3).map(j=><p key={j.id} role={j.status==="failed"?"alert":"status"}>{human(j.kind)}: {j.message}</p>)}
      {workflow.artifacts.length?<section aria-label="Acquired firmware"><h4>Acquired firmware in Library</h4><select aria-label="Workflow artifact" value={artifact?.artifact.sha256??""} onChange={e=>setSelected(e.target.value)}>{workflow.artifacts.map(a=><option key={a.artifact.sha256} value={a.artifact.sha256}>{a.artifact.originalFilename}</option>)}</select><p>Official image download · verified release hash · historical device association</p><a href={`#firmware/${artifact?.artifact.sha256}`}>Open in Library</a>{artifact?<>
        {artifact.analysis?<p>Analysis: {artifact.analysis.format} · {artifact.analysis.regions.length} observed signatures.</p>:<p>Analysis pending; original image preserved.</p>}
        <button className="secondary-button" disabled={!artifact.canAnalyze||busy||active.length>0} onClick={()=>void run(()=>startFirmwareAnalysis(artifact.artifact.sha256))}>{artifact.analysis?"Analyze again":"Analyze firmware"}</button>
        {artifact.canExtract?<FirmwareExtraction key={artifact.artifact.sha256} sha256={artifact.artifact.sha256} jobs={workspace?.jobs??workflow.jobs}/>:<p>Extraction unavailable until analysis identifies a supported candidate. Unknown formats stay preserved.</p>}
      </>:null}</section>:null}
      <details><summary>Acquisition strategies and remaining stages</summary>{workflow.strategies.map(s=><p key={s.strategy}>{s.name}: {human(s.capability.state)}. {s.capability.reason}</p>)}<dl>{workflow.stages.map(s=><div key={s.id}><dt>{human(s.id)}: {human(s.state)}</dt><dd>{s.job?`${human(s.job.status)}: ${s.job.message} `:""}{s.reason} {s.nextAction}</dd></div>)}</dl></details>
      <details><summary>Customization and rebuild readiness</summary><ul>{workflow.configuration.map(s=><li key={s.id}>{human(s.id)}: {human(s.state)} — {s.reason}</li>)}</ul><p>Live configuration and firmware-image changes use separate authorization. Required builder inputs, size limits, partition preservation and signatures are not qualified by extraction.</p></details>
    </>:null}
    <button className="secondary-button" disabled={busy} onClick={()=>setRefresh(r=>r+1)}>Refresh workflow</button>
  </section>;
}
