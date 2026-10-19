import { useEffect, useId, useRef } from "react";
import { AlertTriangle, X } from "lucide-react";
export function ConfirmDialog({ title, children, confirmLabel, busy = false, destructive = false, onConfirm, onClose }: { title: string; children: React.ReactNode; confirmLabel: string; busy?: boolean; destructive?: boolean; onConfirm: () => void; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const cancel = useRef<HTMLButtonElement>(null);
  const id = useId();
  useEffect(() => { const element=dialog.current; const previous=document.activeElement as HTMLElement|null; element?.showModal(); cancel.current?.focus(); return () => { element?.close(); previous?.focus(); }; }, []);
  return <dialog className="confirm-dialog" ref={dialog} aria-labelledby={id} onCancel={event => { event.preventDefault(); if (!busy) onClose(); }}>
    <div className="dialog-heading"><span className={destructive ? "dialog-symbol danger" : "dialog-symbol"}><AlertTriangle size={23}/></span><button disabled={busy} className="icon-button" aria-label="Close confirmation" onClick={onClose}><X size={18}/></button></div>
    <h2 id={id}>{title}</h2><div className="dialog-copy">{children}</div><div className="dialog-actions"><button ref={cancel} disabled={busy} className="secondary-button" onClick={onClose}>Cancel</button><button disabled={busy} className={destructive ? "danger-button" : "primary-button"} onClick={onConfirm}>{busy ? "Working..." : confirmLabel}</button></div>
  </dialog>;
}
