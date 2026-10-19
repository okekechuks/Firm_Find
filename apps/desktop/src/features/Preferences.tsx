import { useEffect, useState } from "react";
import { Monitor, Sun, Moon, Database, FolderOpen, Bell, RefreshCw, ArrowUpRight, ShieldCheck } from "lucide-react";
import { errorMessage, openManagedFolder, saveDesktopPreferences, selectLibraryDirectory, relocateFirmwareLibrary } from "../lib/backend";
import { useWorkspace } from "../lib/workspace-context";
import { formatBytes } from "../lib/format";
import { ConfirmDialog } from "../components/ConfirmDialog";
import type { StoragePlan } from "../types/library";
import type { Settings, Snapshot } from "../types/contracts";

export function Preferences({
  snapshot,
  saving,
  onSave,
  previewTheme,
  onPreviewTheme,
}: {
  snapshot: Snapshot | null;
  saving: boolean;
  onSave: (settings: Settings) => Promise<boolean>;
  previewTheme: Settings["theme"];
  onPreviewTheme: (theme: Settings["theme"]) => void;
}) {
  const [theme, setTheme] = useState<Settings["theme"]>(
    snapshot?.settings.theme ?? previewTheme,
  );
  const [saved, setSaved] = useState(false);
  useEffect(() => {
    if (snapshot?.settings.theme) setTheme(snapshot.settings.theme);
  }, [snapshot?.settings.theme]);
  const choices = [
    { value: "light", label: "Light", icon: Sun },
    { value: "dark", label: "Dark", icon: Moon },
    { value: "system", label: "System", icon: Monitor },
  ] as const;
  return (
    <>
      <section className="panel preference-panel">
        <div className="panel-heading">
          <div>
            <h2>Appearance</h2>
          </div>
          <Sun size={21} />
        </div>
        <fieldset className="theme-options">
          <legend className="sr-only">Theme</legend>
          {choices.map(({ value, label, icon: Icon }) => (
            <label
              key={value}
              className={
                theme === value ? "theme-choice selected" : "theme-choice"
              }
            >
              <input
                type="radio"
                name="theme"
                value={value}
                checked={theme === value}
                onChange={() => {
                  setTheme(value);
                  setSaved(false);
                  if (!snapshot) onPreviewTheme(value);
                }}
              />
              <Icon size={25} />
              <strong>{label}</strong>
              {value === "system" ? <span className="muted small">Follow Windows</span> : null}
            </label>
          ))}
        </fieldset>
        <div className="preference-actions">
          <span className="muted small" role="status">
            {saved
              ? "Preferences saved to SQLite."
              : snapshot
                ? "Settings are stored on this computer."
                : "Browser preview only. Saving requires the desktop app."}
          </span>
          <button
            className="primary-button"
            disabled={!snapshot || saving || theme === snapshot.settings.theme}
            onClick={async () => {
              setSaved(await onSave({ theme }));
            }}
          >
            {saving ? "Saving…" : "Save preferences"}
          </button>
        </div>
      </section>
      <DesktopPreferencesPanel snapshot={snapshot}/>
    </>
  );
}

function DesktopPreferencesPanel({ snapshot }: { snapshot: Snapshot | null }) {
  const workspace = useWorkspace(); const state = workspace?.state; const saved = state?.preferences;
  const [draft, setDraft] = useState({ autoRefresh: true, notifyCompleted: true, notifyFailed: true, notifyDevices: false });
  const [busy, setBusy] = useState(false); const [error, setError] = useState<string | null>(null); const [notice, setNotice] = useState<string | null>(null); const [plan, setPlan] = useState<StoragePlan | null>(null);
  const autoRefresh=saved?.autoRefresh, notifyCompleted=saved?.notifyCompleted, notifyFailed=saved?.notifyFailed, notifyDevices=saved?.notifyDevices;
  useEffect(() => { if (autoRefresh!==undefined && notifyCompleted!==undefined && notifyFailed!==undefined && notifyDevices!==undefined) setDraft({autoRefresh,notifyCompleted,notifyFailed,notifyDevices}); }, [autoRefresh,notifyCompleted,notifyFailed,notifyDevices]);
  const persist = async () => { setBusy(true); setError(null); try { await saveDesktopPreferences(draft); setNotice("Device and notification preferences saved."); await workspace?.refresh(); } catch (e) { setError(errorMessage(e)); } finally { setBusy(false); } };
  const choose = async () => { setBusy(true); setError(null); try { setPlan(await selectLibraryDirectory()); } catch (e) { setError(errorMessage(e)); } finally { setBusy(false); } };
  const move = async () => { if (!plan) return; setBusy(true); setError(null); try { await relocateFirmwareLibrary(plan.ticket); setPlan(null); setNotice("Verified library copy queued. Track progress in Overview, then restart OBTAINER."); await workspace?.refresh(); } catch (e) { setError(errorMessage(e)); } finally { setBusy(false); } };
  const dirty = saved && (Object.keys(draft) as (keyof typeof draft)[]).some(key => draft[key] !== saved[key]);
  return <>
    <section className="panel preference-panel"><div className="panel-heading"><h2>Devices & notifications</h2><Bell size={21}/></div>
      {[{ key: "autoRefresh" as const, label: "Automatic device refresh", detail: "Update inventory when Windows reports hardware changes.", icon: RefreshCw }, { key: "notifyCompleted" as const, label: "Job completion notifications", detail: "Let me know when an operation finishes.", icon: Bell }, { key: "notifyFailed" as const, label: "Job failure notifications", detail: "Keep failures visible until I dismiss them.", icon: Bell }, { key: "notifyDevices" as const, label: "Device arrival and removal", detail: "Notify me when detected device candidates change.", icon: Bell }].map(({ key, label, detail, icon: Icon }) => <label className="setting-row" key={key}><Icon size={18}/><span><strong>{label}</strong><small>{detail}</small></span><input role="switch" type="checkbox" checked={draft[key]} disabled={!state || busy} onChange={e => { setDraft(v => ({ ...v, [key]: e.target.checked })); setNotice(null); }}/></label>)}
      <div className="preference-actions"><span className="muted small">Manual Refresh stays available.</span><button className="primary-button" disabled={!dirty || busy} onClick={() => void persist()}>Save workspace preferences</button></div>
      {notice ? <p role="status" className="inline-success">{notice}</p> : null}{error || workspace?.error ? <p role="alert" className="inline-error">{error ?? workspace?.error}</p> : null}
    </section>
    <section className="panel preference-panel"><div className="panel-heading"><h2>Storage & originals</h2><Database size={21}/></div>
      <div className="storage-setting"><div className="section-heading"><strong>Library directory</strong><button className="text-button" disabled={!state || busy || state.restartRequired} onClick={() => void choose()}>Change folder <ArrowUpRight size={13}/></button></div><p className="mono path-display">{state?.libraryDirectory ?? "Connect the Windows desktop workspace"}</p><div className="section-heading"><span className="muted small">{state ? formatBytes(state.libraryBytes) : "—"} stored</span><button className="secondary-button" disabled={!state} onClick={() => void openManagedFolder("library").catch(e => setError(errorMessage(e)))}><FolderOpen size={15}/>Open Library folder</button></div></div>
      <div className="storage-setting"><strong>Backup directory</strong><p className="mono path-display">{state?.backupDirectory ?? "Created automatically in your per-user application data"}</p><div className="section-heading"><span className="muted small">Separate, preserved copies</span><button className="secondary-button" disabled={!state} onClick={() => void openManagedFolder("backups").catch(e => setError(errorMessage(e)))}><FolderOpen size={15}/>Open Backup folder</button></div></div>
      {state?.restartRequired ? <div className="inline-notice" role="status">Library copy is queued or ready. Check Overview for completion, then close and reopen OBTAINER to activate the new location. Previous storage is preserved.</div> : null}
      <dl className="details"><div><dt>Concurrent jobs</dt><dd>{state?.maxConcurrentJobs ?? 1} · Enforced by the backend</dd></div><div><dt>Workspace database</dt><dd className="mono">{snapshot?.dataDirectory ?? "Not connected"}</dd></div><div><dt>Database schema</dt><dd>{snapshot ? `Version ${snapshot.databaseVersion}` : "Not connected"}</dd></div></dl>
    </section>
    <div className="safety-note"><ShieldCheck size={16}/><span>Local storage. No telemetry. Firmware extraction and device writes require separately verified support.</span></div>
    {plan ? <ConfirmDialog title="Copy Library to a new location?" confirmLabel="Copy and verify Library" busy={busy} onClose={() => setPlan(null)} onConfirm={() => void move()}><p>{plan.fileCount} stored files · {formatBytes(plan.bytes)}</p><dl className="details"><div><dt>From</dt><dd className="mono">{plan.source}</dd></div><div><dt>To</dt><dd className="mono">{plan.destination}</dd></div></dl><p>OBTAINER copies and verifies each registered blob, preserving metadata and the previous directory. After success, imports pause until you restart to use the new location. No existing destination is overwritten.</p>{error ? <p role="alert">{error}</p> : null}</ConfirmDialog> : null}
  </>;
}
