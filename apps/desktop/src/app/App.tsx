import { useCallback, useEffect, useRef, useState } from "react";
import {
  ArrowUpRight,
  ChevronRight,
  Circle,
  RefreshCw,
  ShieldCheck,
  TriangleAlert,
} from "lucide-react";
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
import { Planned } from "../features/Planned";
import { Preferences } from "../features/Preferences";
import { FirmwareLibrary } from "../features/FirmwareLibrary";
import { Jobs } from "../features/Jobs";
import { ViewBoundary } from "../components/ViewBoundary";
import { Devices } from "../features/Devices";
import { Compatibility } from "../features/Compatibility";
import { useDeviceInventory } from "../lib/useDeviceInventory";

export function App() {
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
    destinations.find((item) => item.id === page) ?? destinations[0];

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

  return (
    <div className="app-shell">
      <a
        href="#content"
        className="skip-link"
        onClick={(event) => {
          event.preventDefault();
          document.getElementById("content")?.focus();
        }}
      >
        Skip to content
      </a>
      <aside className="sidebar">
        <a
          className="brand"
          href="#dashboard"
          onClick={() => navigate("dashboard")}
          aria-label="OBTAINER overview"
        >
          <img src="/mark.svg" alt="" />
          <div>
            <strong>OBTAINER</strong>
            <span>FIRMWARE WORKSPACE</span>
          </div>
        </a>
        <div className="workspace-switch">
          <span className="workspace-avatar">O</span>
          <div>
            <strong>Local workspace</strong>
            <span>Windows desktop</span>
          </div>
          <ChevronRight size={15} />
        </div>
        <span className="nav-caption">WORKSPACE</span>
        <nav aria-label="Main navigation">
          {destinations
            .filter((item) => item.id !== "settings")
            .map(({ id, label, icon: Icon }) => (
              <a
                key={id}
                href={`#${id}`}
                onClick={() => navigate(id)}
                className={page === id ? "nav-item active" : "nav-item"}
                aria-current={page === id ? "page" : undefined}
              >
                <Icon size={18} />
                <span>{label}</span>
                {id === "dashboard" ? (
                  <span className="nav-active-dot" />
                ) : null}
              </a>
            ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="foundation-card">
            <ShieldCheck size={22} />
            <strong>Careful by design</strong>
            <p>
              Supported capabilities.
              <br />
              Preserved originals.
            </p>
            <span>FOUNDATION BUILD</span>
          </div>
          <a
            href="#settings"
            onClick={() => navigate("settings")}
            className={page === "settings" ? "nav-item active" : "nav-item"}
            aria-current={page === "settings" ? "page" : undefined}
          >
            <span className="settings-avatar" aria-hidden="true">
              ◈
            </span>
            <span>Settings</span>
            <ArrowUpRight size={14} />
          </a>
          <span className="version">
            OBTAINER · v{snapshot?.version ?? "0.1.0"}
          </span>
        </div>
      </aside>
      <div className="main-shell">
        <header className="topbar">
          <div className="breadcrumb">
            <span>Workspace</span>
            <ChevronRight size={13} />
            <strong>{destination.label}</strong>
          </div>
          <div className="backend-status">
            <Circle size={7} fill="currentColor" />
            <span>
              {!desktop ? "Browser preview"
                : loading ? (snapshot ? "Refreshing workspace" : "Connecting to workspace")
                  : error || !snapshot ? "Backend unavailable" : "Desktop connected"}
            </span>
          </div>
        </header>
        <main id="content" tabIndex={-1}>
          <div className="page-heading">
            <div>
              <span className="eyebrow">{destination.eyebrow}</span>
              <h1>{destination.label}</h1>
              <p>{destination.description}</p>
            </div>
            <button
              className="secondary-button refresh-button"
              onClick={() => void refresh()}
              disabled={loading}
              aria-label="Refresh workspace"
            >
              <RefreshCw size={16} className={loading ? "spin" : undefined} />
              {loading ? "Connecting…" : "Refresh"}
            </button>
          </div>
          {error || saveError ? (
            <div
              className="connection-notice"
              role={desktop ? "alert" : "status"}
            >
              <TriangleAlert size={18} />
              <div>
                <strong>
                  {desktop
                    ? (saveError && !error ? "Preferences could not be saved" : "Workspace connection failed")
                    : "You’re viewing the interface preview"}
                </strong>
                <p>
                  {desktop
                    ? error ?? saveError
                    : "The native backend is not connected. Device counts, audit records, and saved preferences are available in the Windows app."}
                </p>
              </div>
              {desktop && error ? (
                <button onClick={() => void refresh()} className="text-button">
                  Retry
                </button>
              ) : null}
            </div>
          ) : null}
          {loading && !snapshot ? (
            <div className="loading-state" role="status">
              Connecting to local workspace…
            </div>
          ) : null}
          <ViewBoundary key={page}>
          {page === "dashboard" ? (
            <Dashboard snapshot={snapshot} navigate={navigate} />
          ) : page === "devices" ? (
            <Devices inventory={inventory.inventory} desktop={desktop} error={inventory.error}
              refreshing={inventory.refreshing} onRefresh={inventory.refresh} onCancel={inventory.cancel} />
          ) : page === "compatibility" ? (
            <Compatibility devices={inventory.inventory?.devices ?? []} revision={inventory.inventory?.revision ?? 0} desktop={desktop}/>
          ) : page === "firmware" ? (
            <FirmwareLibrary desktop={desktop}/>
          ) : page === "settings" ? (
            <Preferences
              snapshot={snapshot}
              saving={saving}
              onSave={persist}
              previewTheme={previewTheme}
              onPreviewTheme={setPreviewTheme}
            />
          ) : page === "jobs" ? (
            <Jobs desktop={desktop}/>
          ) : (
            <Planned destination={destination} navigate={navigate} />
          )}
          </ViewBoundary>
          <footer className="workspace-footer">
            <span>LOCAL FIRST. ORIGINALS INTACT.</span>
            <span>OBTAINER / WINDOWS</span>
          </footer>
        </main>
      </div>
    </div>
  );
}
