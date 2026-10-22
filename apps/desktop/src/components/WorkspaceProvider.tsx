import { useCallback, useEffect, useRef, useState } from "react";
import { CheckCircle2, AlertCircle, Info, X } from "lucide-react";
import { cancelLibraryJob, errorMessage, getDesktopState, getLibraryActivity, listLibraryJobs } from "../lib/backend";
import { useLibraryFeed } from "../lib/useLibraryFeed";
import { WorkspaceContext, LibraryRevisionContext, type ToastTone } from "../lib/workspace-context";
import type { DesktopState, JobRecord } from "../types/library";
interface Toast { id: number; message: string; tone: ToastTone }
export function WorkspaceProvider({ desktop, children }: { desktop: boolean; children: React.ReactNode }) {
  const [jobs, setJobs] = useState<JobRecord[]>([]);
  const revision = useLibraryFeed(desktop, jobs.some(job => ["pending", "running"].includes(job.status)));
  const [activity, setActivity] = useState<Awaited<ReturnType<typeof getLibraryActivity>>>([]);
  const [state, setState] = useState<DesktopState | null>(null);
  const [loading, setLoading] = useState(desktop);
  const [error, setError] = useState<string | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const generation = useRef(0);
  const started = useRef(Date.now());
  const seen = useRef(new Map<string, string>());
  const firstRead = useRef(true);
  const preferences = useRef<DesktopState["preferences"] | null>(null);
  const toastId = useRef(0);
  const notify = useCallback((message: string, tone: ToastTone = "info") => {
    setToasts(current => [...current, { id: ++toastId.current, message, tone }].slice(-5));
  }, []);
  const refresh = useCallback(async () => {
    if (!desktop) return;
    const run = ++generation.current;
    const replies = await Promise.allSettled([listLibraryJobs(), getLibraryActivity(), getDesktopState()]);
    if (run !== generation.current) return;
    const [jobReply, events, config] = replies;
    if (config.status === "fulfilled") { setState(config.value); preferences.current = config.value.preferences; }
    if (events.status === "fulfilled") setActivity(events.value);
    if (jobReply.status === "fulfilled") {
      const p = preferences.current;
      for (const job of jobReply.value) {
        const previous = seen.current.get(job.id);
        if (!["pending", "running"].includes(job.status) && previous !== job.status &&
          (previous !== undefined || (!firstRead.current && job.createdAt >= started.current))) {
          if (job.status === "completed" && p?.notifyCompleted !== false) notify(job.kind === "backup_restore" ? "Backup restore verified." : `${job.kind === "firmware_backup" ? "Backup created" : job.kind === "integrity_verify" ? "Integrity verification complete" : job.kind === "library_relocation" ? "Library copied. Restart to use the new location" : "Firmware import complete"}${job.originalFilename ? ` — ${job.originalFilename}` : ""}.`, "success");
          else if (job.status === "failed" && p?.notifyFailed !== false) notify(job.message || "Operation failed. Inspect Logs for recovery details.", "error");
          else if (job.status === "cancelled") notify("Operation cancelled. Details retained in Logs.", "info");
        }
        seen.current.set(job.id, job.status);
      }
      if (seen.current.size > 512) { const currentIds = new Set(jobReply.value.map(j => j.id)); for (const id of seen.current.keys()) if (!currentIds.has(id)) seen.current.delete(id); }
      firstRead.current = false;
      setJobs(jobReply.value);
    }
    setError(replies.filter(r => r.status === "rejected").map(r => errorMessage((r as PromiseRejectedResult).reason)).join(" ") || null);
    setLoading(false);
  }, [desktop, notify]);
  useEffect(() => { const requests=generation; void refresh(); return () => { requests.current++; }; }, [refresh, revision]);
  const cancel = useCallback(async (id: string) => { try { await cancelLibraryJob(id); await refresh(); } catch (e) { notify(errorMessage(e), "error"); } }, [refresh, notify]);
  return <WorkspaceContext.Provider value={{ jobs, activity, state, loading, error, revision, refresh, cancel, notify }}><LibraryRevisionContext.Provider value={revision}>{children}</LibraryRevisionContext.Provider>
    <aside className="toast-stack" aria-label="Notifications">
      {state?.libraryAvailable === false ? <div className="notification-toast warning" role="alert"><AlertCircle size={19}/><div><p>{state.restartRequired ? "Library location selected. Restart OBTAINER to activate it. Existing records and settings are retained." : "Library storage is unavailable. Reconnect its drive and restart, or open Settings to use the default location. Existing records and settings are retained."}</p></div></div> : null}
      {toasts.map(toast => <ToastItem key={toast.id} toast={toast} onClose={() => setToasts(items => items.filter(t => t.id !== toast.id))}/>)}</aside>
  </WorkspaceContext.Provider>;
}
function ToastItem({ toast, onClose }: { toast: Toast; onClose: () => void }) {
  const close = useRef(onClose); close.current = onClose;
  useEffect(() => { if (toast.tone === "error" || toast.tone === "warning") return; const timer = setTimeout(() => close.current(), 8000); return () => clearTimeout(timer); }, [toast.tone]);
  const Icon = toast.tone === "success" ? CheckCircle2 : toast.tone === "error" ? AlertCircle : Info;
  return <div className={`notification-toast ${toast.tone}`} role={toast.tone === "error" ? "alert" : "status"}><Icon size={19}/><div><p>{toast.message}</p><a href="#jobs">View in Logs →</a></div><button className="icon-button" aria-label="Dismiss notification" onClick={onClose}><X size={15}/></button></div>;
}
