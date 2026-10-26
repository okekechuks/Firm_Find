import { useEffect,useRef,useState } from "react";
import { X } from "lucide-react";
import { cancelLibraryJob,errorMessage,listOfficialSources,startOfficialDownload } from "../lib/backend";
import { formatBytes } from "../lib/format";
import type { OfficialSource } from "../types/official";
import type { JobRecord } from "../types/library";

export function OfficialImageDialog({jobs,onClose,onOpen}:{jobs:JobRecord[];onClose:()=>void;onOpen:(hash:string)=>void}) {
  const dialog=useRef<HTMLDialogElement>(null);
  const mounted=useRef(false);
  const [sources,setSources]=useState<OfficialSource[]>([]);
  const [selected,setSelected]=useState("");const [confirmed,setConfirmed]=useState(false);
  const [loading,setLoading]=useState(true);const [submitting,setSubmitting]=useState(false);
  const [error,setError]=useState<string|null>(null);const [submitted,setSubmitted]=useState<JobRecord|null>(null);
  useEffect(()=>{mounted.current=true;const element=dialog.current;const previous=document.activeElement as HTMLElement|null;element?.showModal();let current=true;
    void listOfficialSources().then(items=>{if(current){setSources(items);setSelected(items[0]?.id??"");}}).catch(e=>{if(current)setError(errorMessage(e));}).finally(()=>{if(current)setLoading(false);});
    return()=>{mounted.current=false;current=false;element?.close();previous?.focus();};},[]);
  const source=sources.find(item=>item.id===selected);
  const observed=jobs.find(item=>item.id===submitted?.id);
  const job=observed&&observed.updatedAt>=(submitted?.updatedAt??0)?observed:submitted;
  const active=job?.status==="pending"||job?.status==="running";
  const acquire=async()=>{setSubmitting(true);setError(null);try{const result=await startOfficialDownload(selected,confirmed);if(mounted.current)setSubmitted(result);}catch(e){if(mounted.current)setError(errorMessage(e));}finally{if(mounted.current)setSubmitting(false);}};
  const cancel=async()=>{if(!job)return;try{const result=await cancelLibraryJob(job.id);if(mounted.current)setSubmitted(result);}catch(e){if(mounted.current)setError(errorMessage(e));}};
  return <dialog ref={dialog} className="customization-dialog" aria-label="Official firmware" onCancel={e=>{e.preventDefault();onClose();}}><section className="customization-panel">
    <div className="section-heading"><h2>Official firmware</h2><button className="icon-button" aria-label="Close official firmware" onClick={onClose}><X size={18}/></button></div>
    <p>Download an exact OpenWrt release image to Library. This does not extract firmware from a connected device.</p>
    {error?<p role="alert">{error}</p>:null}
    {loading?<p role="status">Loading approved sources…</p>:<label>Model and hardware revision<select aria-label="Official target" value={selected} disabled={submitting||active} onChange={e=>{setSelected(e.target.value);setConfirmed(false);setSubmitted(null);}}>{sources.map(item=><option key={item.id} value={item.id}>{item.vendor} {item.model} {item.revision} · OpenWrt {item.version}</option>)}</select></label>}
    {source?<><p>{source.filename} · {formatBytes(source.sizeBytes)}</p><p className="muted small">Pinned from a signature-verified release catalogue. Downloaded bytes must match its exact SHA-256. Installation compatibility remains unverified.</p><label className="checkbox-label"><input type="checkbox" checked={confirmed} disabled={submitting||active} onChange={e=>setConfirmed(e.target.checked)}/>I selected the exact model and hardware revision shown.</label></>:null}
    {job?<div role="status"><p>{job.message||"Acquiring official firmware…"}</p><progress aria-label="Official image acquisition" max={Math.max(job.totalBytes,1)} value={job.progressBytes}/>{job.status==="completed"&&job.relatedArtifact?<button className="primary-button" onClick={()=>onOpen(job.relatedArtifact!)}>Open in Library</button>:null}</div>:null}
    <div className="dialog-actions"><button className="secondary-button" onClick={onClose}>Close</button>{active?<button className="secondary-button" onClick={()=>void cancel()}>Cancel download</button>:<button className="primary-button" disabled={loading||!source||!confirmed||submitting} onClick={()=>void acquire()}>{submitting?"Queueing…":"Download to Library"}</button>}</div>
  </section></dialog>;
}
