import { ArrowRight, LockKeyhole } from 'lucide-react';
import { destinations, type Destination, type Page } from '../app/navigation';

const content: Record<string, { title: string; body: string; checklist: string[] }> = {
  devices: { title: 'Start with a known device', body: 'Native Windows discovery will list USB, serial, and network interfaces. Connecting a device does not automatically provide firmware access.', checklist: ['Windows device inventory', 'Identification evidence and confidence', 'Per-device capability status'] },
  firmware: { title: 'An intact original. Every time.', body: 'Local firmware import will stream images into a SHA-256 vault, detect duplicates, and record each source. Import is not available in this foundation build.', checklist: ['Streaming import and hashing', 'Content-addressed storage', 'Integrity checks and provenance'] },
  analysis: { title: 'Understand what’s inside', body: 'Read-only analysis will inspect supported signatures and headers in bounded workers. Advanced Linux tools will remain optional.', checklist: ['Format and header inspection', 'Bounded parsing and hex preview', 'Structured findings and reports'] },
  backups: { title: 'Only supported, authorized backups', body: 'Acquisition will require a verified device profile, an approved read method, and explicit authorization. Unknown capabilities never grant access.', checklist: ['Verified model and hardware revision', 'Allowlisted backup targets', 'Streamed acquisition with integrity checks'] },
  builder: { title: 'Build for a supported target', body: 'The OpenWrt builder will create new artifacts from verified inputs. A successful build does not establish installation compatibility.', checklist: ['Validated OpenWrt target and release', 'Reproducible build manifests', 'Separate generated artifacts'] },
};

export function Planned({ destination, navigate }: { destination: Destination; navigate: (page: Page) => void }) {
  const details = content[destination.id];
  const Icon = destination.icon;
  return <>
    <section className="panel planned-panel"><span className="planned-icon"><Icon size={38} strokeWidth={1.4} /></span><span className="tag subtle-tag">PLANNED · PHASE {destination.phase}</span><h2>{details.title}</h2><p>{details.body}</p><button className="secondary-button" onClick={() => navigate('dashboard')}>Back to overview <ArrowRight size={16} /></button></section>
    <section className="panel upcoming"><div className="panel-heading"><h2>Coming to this workspace</h2><LockKeyhole size={18} /></div><ul>{details.checklist.map((item) => <li key={item}><span className="future-dot" />{item}<span className="muted small">Planned</span></li>)}</ul></section>
    <div className="roadmap-strip"><span className="muted small">DEVELOPMENT PATH</span>{destinations.filter((item) => item.phase).map((item) => <button key={item.id} aria-current={item.id === destination.id ? 'page' : undefined} onClick={() => navigate(item.id)}>P{item.phase} · {item.label}</button>)}</div>
  </>;
}
