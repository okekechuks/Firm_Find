import { ArrowUpRight, Router, SlidersHorizontal, Activity, Download, Archive, Files, CheckCircle2, Circle } from "lucide-react";
import type { Page } from "../app/navigation";
import type { Snapshot, DiscoverySnapshot } from "../types/contracts";
import { useWorkspace, isActiveJob } from "../lib/workspace-context";
import { formatBytes } from "../lib/format";
import { JobRows } from "./Jobs";
export function Dashboard({ snapshot, navigate, inventory }: { snapshot: Snapshot | null; navigate: (page: Page) => void; inventory?: DiscoverySnapshot | null }) {
  const workspace = useWorkspace();
  const active = workspace?.jobs.filter(isActiveJob) ?? [];
  const state = workspace?.state;
  const candidates = inventory?.devices.filter(d => d.connected && ["router_mifi_candidate", "usb_modem_candidate", "network_gateway_candidate"].includes(d.category ?? "")).length;
  const stats = [
    { label: "Detected devices", value: candidates, icon: Router, detail: "Router & MiFi candidates", destination: "devices" as const },
    { label: "Customized firmware", value: state?.customized, icon: SlidersHorizontal, detail: "Completed custom images", destination: "firmware" as const, filter: "customized" },
    { label: "Active jobs", value: workspace?.loading ? undefined : active.length, icon: Activity, detail: "Queued or running now", destination: "dashboard" as const },
    { label: "Extracted firmware", value: state?.extracted, icon: Download, detail: "Acquired from a device", destination: "firmware" as const, filter: "extracted" },
  ];
  return <>
    <div className="overview-intro"><div><span className="status-dot"/> LOCAL WORKSPACE <span className="intro-divider">/</span> <span>Ready for careful exploration</span></div><span className="overview-version">DESKTOP · v{snapshot?.version ?? "0.1.0"}</span></div>
    <div className="stats-grid">{stats.map(({ label, value, icon: Icon, detail, destination, filter }, index) => <button className={`panel stat stat-${index}`} key={label} onClick={() => { if (filter) sessionStorage.setItem("obtainer-library-type", filter); if(destination==="dashboard")document.querySelector('[aria-label="Active jobs"]')?.scrollIntoView({behavior:window.matchMedia("(prefers-reduced-motion: reduce)").matches?"auto":"smooth",block:"start"});else navigate(destination); }}><div className="stat-top"><span className="stat-icon"><Icon size={20}/></span><ArrowUpRight size={16}/></div><div className="stat-value"><strong>{value ?? "—"}</strong>{index === 2 ? <span className={`badge ${active.length ? "info" : "neutral"}`}>{active.length ? "In progress" : "All clear"}</span> : null}</div><h2>{label}</h2><p>{detail}</p></button>)}</div>
    <section className="panel active-jobs-panel" aria-label="Active jobs"><div className="section-heading"><div className="section-title"><span className="section-icon"><Activity size={19}/></span><div><h2>Active Jobs <span className="count-badge">{active.length}</span></h2><p>What's happening in your workspace right now.</p></div></div><a className="text-button" href="#jobs">View Logs <ArrowUpRight size={14}/></a></div>
      {workspace?.error ? <p className="inline-error" role="alert">{workspace.error} <button className="text-button" onClick={() => void workspace.refresh()}>Try again</button></p> : null}
      {workspace?.loading ? <div className="empty-state" role="status">Reading active operations…</div> : active.length ? <JobRows jobs={active} onCancel={workspace?.cancel ?? (async () => {})}/> : <div className="empty-state jobs-empty"><div className="empty-orbit"><span><CheckCircle2 size={32} strokeWidth={1.4}/></span><i/><i/><i/></div><h3>No active jobs</h3><p>Import and integrity jobs appear here while running.<br/>Completed work stays in Logs.</p><a href="#firmware" className="secondary-button">Open Library <ArrowUpRight size={15}/></a></div>}
      <div className="panel-footnote"><Circle size={7} fill="currentColor"/> Progress comes from the operation. Your original files stay intact.</div>
    </section>
    <div className="storage-grid"><a href="#firmware" className="panel storage-card"><span className="storage-icon"><Files size={22}/></span><div><h3>Firmware Library</h3><p>{snapshot ? `${snapshot.counts.artifacts} stored ${snapshot.counts.artifacts === 1 ? "artifact" : "artifacts"}` : "Connect the desktop workspace"} <span>·</span> {state ? formatBytes(state.libraryBytes) : "—"}</p></div><ArrowUpRight size={18}/></a><a href="#backups" className="panel storage-card"><span className="storage-icon"><Archive size={22}/></span><div><h3>Originals, kept safe</h3><p>{state ? formatBytes(state.backupBytes) : "—"} in preserved backups</p></div><ArrowUpRight size={18}/></a></div>
  </>;
}
