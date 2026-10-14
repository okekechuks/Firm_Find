import { CheckCircle2, History } from 'lucide-react';
import type { Snapshot } from '../types/contracts';

export function Activity({ snapshot }: { snapshot: Snapshot | null }) {
  const events = snapshot?.recentEvents ?? [];
  return <section className="panel activity-panel" aria-label="Recent activity">
    <div className="panel-heading"><h2>Recent activity</h2><span className="muted small">LOCAL AUDIT TRAIL</span></div>
    {events.length ? <ul className="activity-list">{events.map((event) => <li key={event.id}>
      <span className="event-icon"><CheckCircle2 size={17} /></span>
      <div><strong>{event.message}</strong><span className="muted small">{event.kind.replaceAll('_', ' ')}</span></div>
      <time dateTime={event.createdAt}>{new Date(event.createdAt).toLocaleString()}</time>
    </li>)}</ul> : <div className="activity-empty"><History size={22} /><div><strong>{snapshot ? 'No activity yet' : 'Activity appears in the desktop app'}</strong><p>Workspace events are recorded locally when the backend is connected.</p></div></div>}
  </section>;
}
