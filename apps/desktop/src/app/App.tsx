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
} from "../lib/backend";
import type { Settings, Snapshot } from "../types/contracts";
import { Dashboard } from "../features/Dashboard";
import { Planned } from "../features/Planned";
import { Preferences } from "../features/Preferences";
import { Activity } from "../components/Activity";
import { Devices } from "../features/Devices";
import { useDeviceInventory } from "../lib/useDeviceInventory";

export function App() {
  const [page, setPage] = useState<Page>(pageFromHash);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [previewTheme, setPreviewTheme] = useState<Settings["theme"]>("system");
  const lastDiscoveryRevision = useRef(0);
  const desktop = hasDesktopBackend();
  const inventory = useDeviceInventory(desktop);
  const destination =
    destinations.find((item) => item.id === page) ?? destinations[0];

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setSnapshot(await getSnapshot());
    } catch (failure) {
      setError(errorMessage(failure));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);
  useEffect(() => {
    const changed = () => setPage(pageFromHash());
    window.addEventListener("hashchange", changed);
    return () => window.removeEventListener("hashchange", changed);
  }, []);
  useEffect(() => {
    if (!desktop || !inventory.inventory?.revision) return;
    // Initial workspace load already fetched the audit snapshot in parallel.
    if (lastDiscoveryRevision.current === 0) {
      lastDiscoveryRevision.current = inventory.inventory.revision;
      return;
    }
    if (inventory.inventory.revision <= lastDiscoveryRevision.current) return;
    lastDiscoveryRevision.current = inventory.inventory.revision;
    let active = true;
    void getSnapshot().then((next) => { if (active) setSnapshot(next); }).catch(() => {});
    return () => { active = false; };
  }, [desktop, inventory.inventory?.revision]);
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
    setError(null);
    try {
      const result = await saveSettings(settings);
      setSnapshot((current) =>
        current ? { ...current, settings: result } : current,
      );
      const updated = await getSnapshot();
      setSnapshot(updated);
      return true;
    } catch (failure) {
      setError(errorMessage(failure));
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
              {snapshot
                ? "Desktop connected"
                : desktop
                  ? "Backend unavailable"
                  : "Browser preview"}
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
          {error ? (
            <div
              className="connection-notice"
              role={desktop ? "alert" : "status"}
            >
              <TriangleAlert size={18} />
              <div>
                <strong>
                  {desktop
                    ? "Workspace connection failed"
                    : "You’re viewing the interface preview"}
                </strong>
                <p>
                  {desktop
                    ? error
                    : "The native backend is not connected. Device counts, audit records, and saved preferences are available in the Windows app."}
                </p>
              </div>
              {desktop ? (
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
          {page === "dashboard" ? (
            <Dashboard snapshot={snapshot} navigate={navigate} />
          ) : page === "devices" ? (
            <Devices inventory={inventory.inventory} desktop={desktop} error={inventory.error}
              refreshing={inventory.refreshing} onRefresh={inventory.refresh} />
          ) : page === "settings" ? (
            <Preferences
              snapshot={snapshot}
              saving={saving}
              onSave={persist}
              previewTheme={previewTheme}
              onPreviewTheme={setPreviewTheme}
            />
          ) : page === "jobs" ? (
            <>
              <section className="panel jobs-summary">
                <ListStatus count={snapshot?.counts.runningJobs} />
                <p>
                  Background device and firmware jobs will appear here as those
                  workflows are implemented.
                </p>
              </section>
              <Activity snapshot={snapshot} />
            </>
          ) : (
            <Planned destination={destination} navigate={navigate} />
          )}
          <footer className="workspace-footer">
            <span>LOCAL FIRST. ORIGINALS INTACT.</span>
            <span>OBTAINER / WINDOWS</span>
          </footer>
        </main>
      </div>
    </div>
  );
}

function ListStatus({ count }: { count: number | undefined }) {
  return (
    <h2>
      {count === undefined
        ? "Jobs require the desktop backend"
        : count === 0
          ? "No running jobs"
          : `${count} running jobs`}
    </h2>
  );
}
