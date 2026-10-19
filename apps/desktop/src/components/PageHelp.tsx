import { useEffect, useId, useRef, useState } from "react";
import { Info, X } from "lucide-react";
import type { Page } from "../app/navigation";
const help: Record<string, string[]> = {
  dashboard: ["Statistics show detected candidates, customized images, active jobs and extracted images. Imported files are counted separately in Library.", "Active Jobs tracks the same saved operations across every page. Completed and failed work moves to Logs.", "Progress measures the current processing stage. No percentage or ETA is shown when it cannot be measured reliably.", "Cancel a supported job here. Cancellation takes effect at a safe point; the saved job outcome is authoritative."],
  devices: ["Select a device to inspect its identity and connections. Refresh updates the Windows inventory.", "Green means verified router identity; yellow means uncertain; red means protected or unsupported hardware. Color never grants extraction permission.", "Extract Firmware stays unavailable until an exact device and authorized acquisition method are independently verified.", "Add a management address only for equipment you own or administer. Network checks need separate confirmation.", "Inspect feature evidence below the selected device. User labels and USB IDs do not verify model or firmware."],
  firmware: ["Import Firmware uses the native file picker and preserves your source. Search, sort or filter images by their recorded type and integrity.", "Select a file for its checksum, history, storage location and actions. Verify Integrity checks stored bytes, not device compatibility.", "Create Backup preserves and verifies a separate local copy. Delete removes the image from Library while keeping bytes and history for recovery.", "Customize shows APN, Wi-Fi limits, DNS and USSD availability. Rebuild is blocked until a verified device-specific build pathway exists.", "A supported rebuild must preserve and verify an original backup before proceeding. No general proprietary rebuild is available."],
  backups: ["Create a preserved copy from a selected Library image. Backups are separate from Library storage and retain the source checksum.", "Select a backup to inspect its name, source, size and last successful verification.", "Restore copies verified bytes into Library, adds _DEFAULT before the extension and resolves name conflicts. It never flashes a device.", "The backup remains after restoration. Delete needs confirmation and removes only the selected backup from this view; bytes remain recoverable.", "Refresh reads saved backup records. Integrity is rechecked during every backup and restore operation."],
  jobs: ["Logs retains real operations, completed jobs, cancellations, warnings and application events across restarts.", "Search descriptions or IDs. Filter by category, severity and status, or change the date order.", "Select an event to inspect its timestamp, associated artifact, job outcome and available recovery information.", "The latest 100 jobs and 100 events are shown. Full saved history is retained locally; this view does not delete it."],
  settings: ["Choose Light, Dark or System and save preferences. System follows your Windows color preference.", "Automatic device refresh responds to hardware changes. Turning it off still allows a manual Refresh.", "Library storage is managed by OBTAINER. A directory change must copy and verify existing files before switching; the current path remains visible.", "Backups use their own managed per-user folder. Open Folder opens the actual managed directory.", "Completion and failure notifications apply to real jobs. The single backend worker enforces safe concurrency."],
};
export function PageHelp({ page }: { page: Page }) {
  const [open, setOpen] = useState(false);
  const host = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const close = useRef<HTMLButtonElement>(null);
  const id = useId();
  useEffect(() => {
    if (!open) return;
    close.current?.focus();
    const pointer = (event: PointerEvent) => { if (!host.current?.contains(event.target as Node)) setOpen(false); };
    const keyboard = (event: KeyboardEvent) => {
      if (event.key === "Escape") { setOpen(false); trigger.current?.focus(); }
      if (event.key === "Tab") {
        const nodes = host.current?.querySelectorAll<HTMLButtonElement>("button");
        if (!nodes?.length) return;
        if (event.shiftKey && document.activeElement === nodes[0]) { event.preventDefault(); nodes[nodes.length - 1].focus(); }
        else if (!event.shiftKey && document.activeElement === nodes[nodes.length - 1]) { event.preventDefault(); nodes[0].focus(); }
      }
    };
    document.addEventListener("pointerdown", pointer);
    document.addEventListener("keydown", keyboard);
    return () => { document.removeEventListener("pointerdown", pointer); document.removeEventListener("keydown", keyboard); };
  }, [open]);
  return <div className="page-help" ref={host}>
    <button ref={trigger} className={`icon-button help-trigger${open ? " selected" : ""}`} aria-label="How to use this page" aria-expanded={open} aria-controls={id} onClick={() => setOpen(v => !v)}><Info size={19}/></button>
    {open ? <section id={id} role="dialog" aria-labelledby={`${id}-title`} className="help-popover"><div className="section-heading"><h2 id={`${id}-title`}>How to use this page</h2><button className="icon-button" ref={close} aria-label="Close page help" onClick={() => { setOpen(false); trigger.current?.focus(); }}><X size={17}/></button></div><ol>{(help[page === "compatibility" ? "devices" : page] ?? help.firmware).map(step => <li key={step}>{step}</li>)}</ol></section> : null}
  </div>;
}
