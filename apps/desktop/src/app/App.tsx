import { useCallback, useEffect, useRef, useState } from "react";
import { Circle, RefreshCw, TriangleAlert, PanelLeftClose, PanelLeftOpen } from "lucide-react";
import { destinations, pageFromHash, type Page } from "./navigation";
import {
  errorMessage,
  getSnapshot,
  hasDesktopBackend,
  saveSettings,
  onLibraryChanged,
} from "../lib/backend";
import type { Settings, Snapshot } from "../types/contracts";
import { Dashboard } from "../features/Dashboard";
import { Backups } from "../features/Backups";
import { PageHelp } from "../components/PageHelp";
import { WorkspaceProvider } from "../components/WorkspaceProvider";
import { useWorkspace } from "../lib/workspace-context";
import type { DiscoverySnapshot } from "../types/contracts";
import { Preferences } from "../features/Preferences";
import { FirmwareLibrary } from "../features/FirmwareLibrary";
import { Jobs } from "../features/Jobs";
import { ViewBoundary } from "../components/ViewBoundary";
import { Devices } from "../features/Devices";
import { SimulationWorkspace } from "../features/simulation/SimulationWorkspace";
import { Compatibility } from "../features/Compatibility";
import { useDeviceInventory } from "../lib/useDeviceInventory";

export function App() {
  const [compact,setCompact]=useState(false);
  const [page, setPage] = useState<Page>(pageFromHash);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [previewTheme, setPreviewTheme] = useState<Settings["theme"]>("system");
  const lastDiscoveryRevision = useRef(0);
  const workspaceRequest = useRef(0);
  const desktop = hasDesktopBackend();
  const inventory = useDeviceInventory(desktop);
  const destination =
    destinations.find((item) => item.id === page) ?? { ...destinations[1], label: "Device compatibility" };

  const refresh = useCallback(async () => {
    const request = ++workspaceRequest.current;
    setLoading(true);
    setError(null);
    try {
      const next = await getSnapshot();
      if (request === workspaceRequest.current) setSnapshot(next);
    } catch (failure) {
      if (request === workspaceRequest.current) setError(errorMessage(failure));
    } finally {
      if (request === workspaceRequest.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    return () => { workspaceRequest.current += 1; };
  }, [refresh]);
  useEffect(()=>{
    if(!desktop)return;let active=true;let unlisten:(()=>void)|undefined;const states=new Map<string,string>();
    void onLibraryChanged(job=>{if(active&&states.get(job.id)!==job.status){states.set(job.id,job.status);if(states.size>256)states.delete(states.keys().next().value!);void refresh();}}).then(fn=>{if(active)unlisten=fn;else fn();}).catch(()=>{});
    return()=>{active=false;unlisten?.();};
  },[desktop,refresh]);
  useEffect(() => {
    const changed = () => setPage(pageFromHash());
    window.addEventListener("hashchange", changed);
    return () => window.removeEventListener("hashchange", changed);
  }, []);
  useEffect(() => {
    if (!desktop || !inventory.inventory?.revision) return;
    if (inventory.inventory.revision <= lastDiscoveryRevision.current) return;
    lastDiscoveryRevision.current = inventory.inventory.revision;
    // Initial enumeration can finish after the first workspace read.
    void refresh();
  }, [desktop, inventory.inventory?.revision, refresh]);
  useEffect(() => {
    const theme = snapshot?.settings.theme ?? previewTheme;
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      document.documentElement.dataset.theme =
        theme === "system" ? (query.matches ? "dark" : "light") : theme;
    };
    apply();
    query.addEventListener("change", apply);
    return () => query.removeEventListener("change", apply);
  }, [snapshot?.settings.theme, previewTheme]);
  useEffect(() => { window.scrollTo(0,0); }, [page]);
  useEffect(() => {
    document.title = `${destination.label} · OBTAINER`;
  }, [destination.label]);

  const navigate = (next: Page) => {
    window.location.hash = next;
    setPage(next);
  };
  const persist = async (settings: Settings): Promise<boolean> => {
    setSaving(true);
    setSaveError(null);
    try {
      const result = await saveSettings(settings);
      setSnapshot((current) =>
        current ? { ...current, settings: result } : current,
      );
      // A confirmed write stays successful even if the subsequent metadata read fails.
      await refresh();
      return true;
    } catch (failure) {
      setSaveError(errorMessage(failure));
      return false;
    } finally {
      setSaving(false);
    }
  };

  return <WorkspaceProvider desktop={desktop}>
    <DeviceNotifications inventory={inventory.inventory}/>
    <div className={`app-shell${compact ? " sidebar-compact" : ""}`}>
      <a href="#content" className="skip-link" onClick={event => { event.preventDefault(); document.getElementById("content")?.focus(); }}>Skip to content</a>
      <aside className="sidebar">
        <a className="brand" href="#dashboard" onClick={() => navigate("dashboard")} aria-label="OBTAINER overview"><img src="/mark.svg" alt=""/><div><strong>OBTAINER<span className="brand-dot">.</span></strong><span>FIRMWARE WORKSPACE</span></div></a>
        <nav aria-label="Main navigation">{destinations.map(({ id, label, icon: Icon }) => <a key={id} href={`#${id}`} onClick={() => navigate(id)} title={label} aria-label={label} className={`nav-item${page === id || (page === "compatibility" && id === "devices") ? " active" : ""}`} aria-current={page === id || (page === "compatibility" && id === "devices") ? "page" : undefined}><Icon size={19}/><span>{label}</span>{page === id ? <i className="nav-active-dot"/> : null}</a>)}</nav>
        <div className="sidebar-bottom"><div className="sidebar-meta"><span className="version">OBTAINER · v{snapshot?.version ?? "0.1.0"}</span><button className="icon-button sidebar-toggle" aria-label={compact ? "Expand sidebar" : "Collapse sidebar"} onClick={() => setCompact(v => !v)}>{compact ? <PanelLeftOpen size={16}/> : <PanelLeftClose size={16}/>}</button></div></div>
      </aside>
      <div className="main-shell"><header className="topbar"><div className="backend-status"><Circle size={6} fill="currentColor"/><span>{!desktop ? "Browser preview" : loading ? snapshot ? "Refreshing workspace" : "Connecting to workspace" : error || !snapshot ? "Backend unavailable" : "Desktop connected"}</span></div></header>
        <main id="content" tabIndex={-1}>
          {snapshot?.testProfile ? <div className="connection-notice native-test-notice" role="status" data-test-run-id={snapshot.testProfile.runId}>{snapshot.testProfile.discoverySource==="synthetic_workflow_fixture"?"SOFTWARE WORKFLOW FIXTURE · ISOLATED NATIVE TEST":snapshot.testProfile.simulationAvailable?"SIMULATION MODE · ISOLATED NATIVE TEST":"ISOLATED NATIVE TEST"} · {snapshot.testProfile.runId}. {snapshot.testProfile.discoverySource==="synthetic_workflow_fixture"?"Synthetic identity only. Real image processing; no physical device identity or acquisition has been verified.":snapshot.testProfile.simulationAvailable?"This is a virtual router. Changes made here do not affect connected physical devices.":"Separate test data. Discovery shows Windows observations; no simulated adapters or verified device support."}</div> : null}
          <div className="page-heading"><h1>{destination.label}</h1><div className="page-actions"><button className="icon-button workspace-refresh refresh-button" onClick={() => void refresh()} disabled={loading} aria-label="Refresh workspace" title="Refresh workspace"><RefreshCw size={17} className={loading ? "spin" : undefined}/></button><PageHelp key={page} page={page} simulation={snapshot?.testProfile?.simulationAvailable===true}/></div></div>
          {error || saveError ? <div className="connection-notice" role={desktop ? "alert" : "status"}><TriangleAlert size={18}/><div><strong>{desktop ? saveError && !error ? "Preferences could not be saved" : "Workspace connection failed" : "You’re viewing the interface preview"}</strong><p>{desktop ? error ?? saveError : "Live device inventory and saved data are available in the Windows app."}</p></div>{desktop && error ? <button className="text-button" onClick={() => void refresh()}>Retry</button> : null}</div> : null}
          {loading && !snapshot ? <div className="loading-state" role="status">Connecting to local workspace…</div> : null}
          <ViewBoundary key={page}>
            {page === "dashboard" ? <Dashboard snapshot={snapshot} navigate={navigate} inventory={inventory.inventory}/>
              : page === "devices" ? <>{snapshot?.testProfile?.simulationAvailable?<SimulationWorkspace/>:null}<Devices inventory={inventory.inventory} desktop={desktop} error={inventory.error} refreshing={inventory.refreshing} onRefresh={inventory.refresh} onCancel={inventory.cancel}/></>
              : page === "compatibility" ? <Compatibility devices={inventory.inventory?.devices ?? []} revision={inventory.inventory?.revision ?? 0} desktop={desktop}/>
              : page === "firmware" ? <FirmwareLibrary desktop={desktop}/>
              : page === "backups" ? <Backups desktop={desktop}/>
              : page === "jobs" ? <Jobs desktop={desktop}/>
              : page === "settings" ? <Preferences snapshot={snapshot} saving={saving} onSave={persist} previewTheme={previewTheme} onPreviewTheme={setPreviewTheme}/>
              : null}
          </ViewBoundary>
        </main>
      </div>
    </div>
  </WorkspaceProvider>;
}
function DeviceNotifications({ inventory }: { inventory: DiscoverySnapshot | null }) {
  const workspace = useWorkspace(); const seen = useRef<Set<string> | null>(null);
  const notify = workspace?.notify; const enabled = workspace?.state?.preferences.notifyDevices;
  useEffect(() => {
    if (!inventory?.scannedAt) return;
    const ids = new Set(inventory.devices.filter(d => d.connected && ["router_mifi_candidate", "usb_modem_candidate", "network_gateway_candidate"].includes(d.category ?? "")).map(d => d.id));
    if (seen.current && enabled) {
      const added = [...ids].filter(id => !seen.current!.has(id)).length;
      const removed = [...seen.current].filter(id => !ids.has(id)).length;
      if (added) notify?.(`${added} device candidate${added === 1 ? "" : "s"} detected. Inspect Connected Devices.`, "info");
      if (removed) notify?.(`${removed} device candidate${removed === 1 ? "" : "s"} disconnected.`, "warning");
    }
    seen.current = ids;
  }, [inventory, enabled, notify]);
  return null;
}
