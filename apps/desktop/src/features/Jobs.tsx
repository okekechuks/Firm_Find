import { useEffect,useState } from "react";
import { cancelLibraryJob,errorMessage,getLibraryActivity,listLibraryJobs } from "../lib/backend";
import { useLibraryFeed } from "../lib/useLibraryFeed";
import type { JobRecord } from "../types/library";

export function JobRows({jobs,onCancel}:{jobs:JobRecord[];onCancel:(id:string)=>Promise<void>}) {
  return <div className="library-jobs">{jobs.map(job=><article className="panel job-row" key={job.id} data-job-id={job.id}>
    <div className="section-heading"><h3>{job.kind.replaceAll("_"," ")}</h3><strong>{job.status}</strong></div>
    <p>{job.stage.replaceAll("_"," ")} · {job.progressBytes.toLocaleString()} / {job.totalBytes.toLocaleString()} bytes{job.totalBytes?` · ${Math.min(100,Math.floor(job.progressBytes/job.totalBytes*100))}%`:""}</p>
    {job.status==="running"||job.status==="pending"?<><progress max={Math.max(1,job.totalBytes)} value={job.progressBytes}/><button className="secondary-button" disabled={job.cancelRequested} onClick={()=>void onCancel(job.id)}>{job.cancelRequested?"Cancellation requested":"Cancel job"}</button></>:null}
    <p role={job.status==="failed"?"alert":undefined}>{job.message}</p>{job.errorCode?<p className="muted small">{job.errorCode}</p>:null}{job.duplicate?<p>Duplicate recognized. One stored blob; another import occurrence recorded.</p>:null}
    {job.relatedArtifact?<p className="hash-value">SHA-256: {job.relatedArtifact}</p>:null}{job.recoveryInfo?<p>{job.recoveryInfo}</p>:null}
    <time className="muted small">{new Date(job.createdAt).toLocaleString()} · Job {job.id}</time>
  </article>)}</div>;
}
export function Jobs({desktop}:{desktop:boolean}) {
  const revision=useLibraryFeed(desktop);const [jobs,setJobs]=useState<JobRecord[]>([]);const [activity,setActivity]=useState<Awaited<ReturnType<typeof getLibraryActivity>>>([]);const [error,setError]=useState<string|null>(null);const [loaded,setLoaded]=useState(false);
  useEffect(()=>{let active=true;if(desktop)void Promise.all([listLibraryJobs(),getLibraryActivity()]).then(([j,a])=>{if(active){setJobs(j);setActivity(a);setError(null);setLoaded(true);}}).catch(e=>{if(active)setError(errorMessage(e));});return()=>{active=false;};},[desktop,revision]);
  const cancel=async(id:string)=>{try{await cancelLibraryJob(id);}catch(e){setError(errorMessage(e));}};
  return <><section className="panel library-panel"><h2>Persistent library jobs</h2><p>Latest 100 operations. Progress and history come from the Windows backend.</p>{!desktop?<p>Open the Windows app to view saved jobs and activity.</p>:!loaded&&!error?<p role="status">Loading jobs…</p>:null}{error?<p role="alert">{error}</p>:null}{loaded&&jobs.length===0?<p>No library jobs yet. Import a file from Firmware Library.</p>:null}</section><JobRows jobs={jobs} onCancel={cancel}/>
    <section className="panel library-panel" aria-label="Persistent activity"><h2>Workspace activity</h2>{activity.length?<ul className="library-activity">{activity.map(e=><li key={e.id}><strong>{e.message}</strong><p>{e.kind.replaceAll("_"," ")} · {e.severity}{e.jobId?` · Job ${e.jobId}`:""}</p><time>{new Date(e.createdAt).toLocaleString()}</time></li>)}</ul>:<p>No saved activity to show.</p>}</section></>;
}
