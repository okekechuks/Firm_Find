import {
  ArrowRight,
  Router,
  Files,
  ListChecks,
  ShieldCheck,
  Fingerprint,
  Layers3,
  Circle,
} from "lucide-react";
import type { Page } from "../app/navigation";
import type { Snapshot } from "../types/contracts";
import { Activity } from "../components/Activity";

export function Dashboard({
  snapshot,
  navigate,
}: {
  snapshot: Snapshot | null;
  navigate: (page: Page) => void;
}) {
  const stats = [
    {
      label: "Connected devices",
      value: snapshot?.counts.devices,
      icon: Router,
      detail: "Windows-reported interfaces",
    },
    {
      label: "Firmware artifacts",
      value: snapshot?.counts.artifacts,
      icon: Files,
      detail: "Copies stored in the local vault",
    },
    {
      label: "Running jobs",
      value: snapshot?.counts.runningJobs,
      icon: ListChecks,
      detail: "Persistent library operations",
    },
  ];
  return (
    <>
      <section className="welcome-card">
        <div className="welcome-copy">
          <span className="tag">BUILT FOR CAREFUL EXPLORATION</span>
          <h2>
            Your firmware.
            <br />A clearer picture.
          </h2>
          <p>
            One workspace to discover devices, preserve firmware, and understand
            what’s inside.
          </p>
          <button
            className="primary-button"
            onClick={() => navigate("devices")}
          >
            Explore workspace <ArrowRight size={17} />
          </button>
        </div>
        <div className="hardware-art" aria-hidden="true">
          <div className="art-orbit orbit-one" />
          <div className="art-orbit orbit-two" />
          <div className="art-chip">
            <Layers3 size={52} strokeWidth={1.25} />
            <span>OBTAINER</span>
            <div className="chip-lights">
              <i />
              <i />
              <i />
            </div>
          </div>
          <span className="art-label">
            <Circle size={7} fill="currentColor" /> ORIGINALS PRESERVED
          </span>
        </div>
      </section>
      <div className="stats-grid">
        {stats.map(({ label, value, icon: Icon, detail }) => (
          <section className="panel stat" key={label}>
            <div className="stat-top">
              <span>{label}</span>
              <Icon size={19} />
            </div>
            <strong>{value ?? "—"}</strong>
            <span className="muted small">{detail}</span>
          </section>
        ))}
      </div>
      <div className="section-heading">
        <h2>A workspace with a plan</h2>
        <span className="muted small">
          FOUNDATION · v{snapshot?.version ?? "0.1.0"}
        </span>
      </div>
      <div className="workflow-grid">
        <button
          className="panel workflow-card"
          onClick={() => navigate("devices")}
        >
          <span className="step-number">01</span>
          <Router size={25} />
          <h3>Know your device</h3>
          <p>Identify interfaces and review what your hardware supports.</p>
          <span className="card-link">
            Device explorer <ArrowRight size={15} />
          </span>
        </button>
        <button
          className="panel workflow-card"
          onClick={() => navigate("firmware")}
        >
          <span className="step-number">02</span>
          <Fingerprint size={25} />
          <h3>Preserve the original</h3>
          <p>Keep firmware intact, with a verifiable fingerprint and source.</p>
          <span className="card-link">
            Firmware library <ArrowRight size={15} />
          </span>
        </button>
        <button
          className="panel workflow-card"
          onClick={() => navigate("analysis")}
        >
          <span className="step-number">03</span>
          <Layers3 size={25} />
          <h3>Look a little deeper</h3>
          <p>Inspect image structures in a separate analysis workspace.</p>
          <span className="card-link">
            Analysis workspace <ArrowRight size={15} />
          </span>
        </button>
      </div>
      <Activity snapshot={snapshot} />
      <div className="safety-note">
        <ShieldCheck size={17} />
        <span>
          Capability-driven workflows. Original artifacts preserved. Device
          operations require authorization.
        </span>
      </div>
    </>
  );
}
