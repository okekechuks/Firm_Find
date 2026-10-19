import { LayoutDashboard, Router, Files, Archive, ScrollText, Settings2, type LucideIcon } from "lucide-react";
export type Page = "dashboard" | "devices" | "firmware" | "backups" | "jobs" | "settings" | "compatibility" | "analysis" | "builder";
export interface Destination { id: Page; label: string; icon: LucideIcon; eyebrow: string; description: string; phase?: number }
export const destinations: Destination[] = [
  { id: "dashboard", label: "Overview", icon: LayoutDashboard, eyebrow: "YOUR WORKSPACE", description: "A little clarity. A longer life for your hardware." },
  { id: "devices", label: "Connected Devices", icon: Router, eyebrow: "DISCOVER & IDENTIFY", description: "Know what is connected before taking the next step." },
  { id: "firmware", label: "Library", icon: Files, eyebrow: "YOUR FIRMWARE, PRESERVED", description: "Every image. Its origin. A place to keep it safe." },
  { id: "backups", label: "Backup Manager", icon: Archive, eyebrow: "KEEP A WAY BACK", description: "Preserved copies, ready when you need them." },
  { id: "jobs", label: "Logs", icon: ScrollText, eyebrow: "THE FULL PICTURE", description: "A clear record of what happened, and what comes next." },
  { id: "settings", label: "Settings", icon: Settings2, eyebrow: "MAKE IT YOURS", description: "Your workspace. Your preferences." },
];
export function pageFromHash(): Page {
  const id = window.location.hash.slice(1);
  if (id === "compatibility") return id;
  if (id === "analysis" || id === "builder") return "firmware";
  return destinations.find(d => d.id === id)?.id ?? "dashboard";
}
