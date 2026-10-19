import { LayoutDashboard, Router, Files, Archive, ScrollText, Settings2, type LucideIcon } from "lucide-react";
export type Page = "dashboard" | "devices" | "firmware" | "backups" | "jobs" | "settings" | "compatibility" | "analysis" | "builder";
export interface Destination { id: Page; label: string; icon: LucideIcon }
export const destinations: Destination[] = [
  { id: "dashboard", label: "Overview", icon: LayoutDashboard },
  { id: "devices", label: "Connected Devices", icon: Router },
  { id: "firmware", label: "Library", icon: Files },
  { id: "backups", label: "Backup Manager", icon: Archive },
  { id: "jobs", label: "Logs", icon: ScrollText },
  { id: "settings", label: "Settings", icon: Settings2 },
];
export function pageFromHash(): Page {
  const id = window.location.hash.slice(1);
  if (id === "compatibility") return id;
  if (id === "analysis" || id === "builder") return "firmware";
  return destinations.find(d => d.id === id)?.id ?? "dashboard";
}
