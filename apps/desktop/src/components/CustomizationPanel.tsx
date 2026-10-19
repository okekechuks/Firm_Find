import { LockKeyhole, Network, Radio, Users, MessageSquare, X, ShieldCheck } from "lucide-react";
import { kindLabels, firmwareKind } from "../lib/format";
import type { ArtifactDetails } from "../types/library";
const features = [
  { id: "apn", name: "APN customization", icon: Radio, detail: "Automatic or manual operator settings, where verified." },
  { id: "wifi", name: "Maximum Wi-Fi users", icon: Users, detail: "A device-specific limit, within verified hardware capacity." },
  { id: "dns", name: "Custom DNS", icon: Network, detail: "Upstream DNS and LAN-advertised DNS are evaluated separately." },
  { id: "ussd", name: "USSD feature", icon: MessageSquare, detail: "Requires a verified modem control interface and supported firmware." },
];
// No build adapter exists. A source file or a user label must never grant feature support.
export function CustomizationPanel({ details, onClose }: { details: ArtifactDetails; onClose: () => void }) {
  return <section className="panel customization-panel" aria-label="Customize firmware"><div className="section-heading"><h2>Customize firmware</h2><button className="icon-button" aria-label="Close customization" onClick={onClose}><X size={18}/></button></div>
    <div className="selected-file"><span className="file-tile"><Radio size={23}/></span><div><strong>{details.artifact.originalFilename}</strong><p>{kindLabels[firmwareKind(details.artifact)]} · Original stays preserved</p></div><span className="badge warning">Compatibility unverified</span></div>
    <div className="feature-grid">{features.map(({ id, name, icon: Icon, detail }) => <div className="feature-card" key={id}><div className="section-heading"><Icon size={21}/><input type="checkbox" role="switch" aria-label={name} checked={false} disabled title="No verified build adapter is available"/></div><h3>{name}</h3><p>{detail}</p><span className="feature-unavailable"><LockKeyhole size={12}/> Not available for this image</span></div>)}</div>
    <div className="inline-notice"><ShieldCheck size={18}/><p>No verified extraction or build adapter is available. Feature toggles and Rebuild stay disabled. A supported build must first create and verify an original backup.</p></div><div className="dialog-actions"><button className="secondary-button" onClick={onClose}>Cancel</button><button className="primary-button" disabled><LockKeyhole size={15}/> Rebuild firmware</button></div>
  </section>;
}
