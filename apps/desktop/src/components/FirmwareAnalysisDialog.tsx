import { useEffect,useRef,useState } from "react";
import { X } from "lucide-react";
import { errorMessage,getFirmwareAnalysis,getOfficialProvenance,startFirmwareAnalysis,cancelLibraryJob } from "../lib/backend";
import { formatBytes } from "../lib/format";
import type { AnalysisReport,OfficialSource } from "../types/official";
import type { FirmwareArtifact,JobRecord } from "../types/library";

export function FirmwareAnalysisDialog({artifact,jobs,onClose}:{artifact:FirmwareArtifact;jobs:JobRecord[];onClose:()=>void}) {
  const dialog=useRef<HTMLDialogElement>(null);const mounted=useRef(false);
  const [report,setReport]=useState<AnalysisReport|null>(null);const [sources,setSources]=useState<OfficialSource[]>([]);
  const [error,setError]=useState<string|null>(null);const [submitted,setSubmitted]=useState<JobRecord|null>(null);const [busy,setBusy]=useState(false);
  const observed=jobs.find(item=>item.id===submitted?.id);
  const job=observed&&observed.updatedAt>=(submitted?.updatedAt??0)?observed:submitted;
  const active=job?.status==="running"||job?.status==="pending";
  useEffect(()=>{mounted.current=true;const element=dialog.current;const previous=document.activeElement as HTMLElement|null;element?.showModal();return()=>{mounted.current=false;element?.close();previous?.focus();};},[]);
  useEffect(()=>{let current=true;void Promise.all([getFirmwareAnalysis(artifact.sha256),getOfficialProvenance(artifact.sha256)]).then(([analysis,provenance])=>{if(current){setReport(analysis);setSources(provenance);}}).catch(e=>{if(current)setError(errorMessage(e));});return()=>{current=false;};},[artifact.sha256,job?.status]);
  const run=async()=>{setBusy(true);setError(null);try{const next=await startFirmwareAnalysis(artifact.sha256);if(mounted.current)setSubmitted(next);}catch(e){if(mounted.current)setError(errorMessage(e));}finally{if(mounted.current)setBusy(false);}};
  const cancel=async()=>{if(!job)return;try{const next=await cancelLibraryJob(job.id);if(mounted.current)setSubmitted(next);}catch(e){if(mounted.current)setError(errorMessage(e));}};
  return <dialog ref={dialog} className="customization-dialog" aria-label="Firmware analysis" onCancel={e=>{e.preventDefault();onClose();}}><section className="customization-panel">
    <div className="section-heading"><h2>Firmware analysis</h2><button className="icon-button" aria-label="Close firmware analysis" onClick={onClose}><X size={18}/></button></div>
    <p>{artifact.originalFilename} · {formatBytes(artifact.sizeBytes)}</p>
    {sources[0]?<p>Official target: {sources[0].vendor} {sources[0].model} {sources[0].revision} · OpenWrt {sources[0].version}</p>:null}
    <p className="muted small">Bounded format inspection. Recursive extraction and image execution are disabled.</p>
    {error?<p role="alert">{error}</p>:null}
    {report?<><dl><dt>Format</dt><dd>{report.format}</dd><dt>Inspected</dt><dd>{formatBytes(report.scannedBytes)} of {formatBytes(report.inputBytes)}{report.scanTruncated?" (scan capped)":""}</dd><dt>Sample entropy</dt><dd>{report.sampledEntropy.toFixed(3)} bits/byte</dd><dt>Analyzed</dt><dd>{new Date(report.analyzedAt).toLocaleString()}</dd></dl><details><summary>Observed format signatures ({report.regions.length})</summary><ul>{report.regions.map(item=><li key={`${item.offset}-${item.format}`}>{item.format} · offset {item.offset.toLocaleString()}</li>)}</ul></details><p>Hardware compatibility: unverified</p></>:<p>No analysis report yet.</p>}
    {job?<div role="status"><p>{job.message||"Analyzing…"}</p><progress aria-label="Firmware analysis progress" max={Math.max(job.totalBytes,1)} value={job.progressBytes}/></div>:null}
    <div className="dialog-actions"><button className="secondary-button" onClick={onClose}>Close</button>{active?<button className="secondary-button" onClick={()=>void cancel()}>Cancel analysis</button>:<button className="primary-button" disabled={busy} onClick={()=>void run()}>{report?"Analyze again":"Analyze firmware"}</button>}</div>
  </section></dialog>;
}
