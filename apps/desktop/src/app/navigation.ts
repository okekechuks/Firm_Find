import {
  LayoutDashboard,
  Router,
  Files,
  ScanLine,
  Archive,
  Blocks,
  ListChecks,
  Settings2,
  type LucideIcon,
} from "lucide-react";

export type Page =
  | "dashboard"
  | "devices"
  | "compatibility"
  | "firmware"
  | "analysis"
  | "backups"
  | "builder"
  | "jobs"
  | "settings";
export interface Destination {
  id: Page;
  label: string;
  icon: LucideIcon;
  eyebrow: string;
  description: string;
  phase?: number;
}
export const destinations: Destination[] = [
  {
    id: "dashboard",
    label: "Overview",
    icon: LayoutDashboard,
    eyebrow: "YOUR WORKSPACE",
    description: "A clear view of your firmware research.",
  },
  {
    id: "devices",
    label: "Connected devices",
    icon: Router,
    eyebrow: "DEVICE EXPLORER",
    description: "Review USB, serial, and network hardware Windows can see.",
    phase: 2,
  },
  {
    id: "compatibility",
    label: "Compatibility",
    icon: ListChecks,
    eyebrow: "DEVICE SUPPORT",
    description: "Understand feature support, evidence and prerequisites for your device.",
    phase: 3,
  },
  {
    id: "firmware",
    label: "Firmware library",
    icon: Files,
    eyebrow: "ARTIFACT VAULT",
    description: "Preserve original images and their provenance.",
    phase: 4,
  },
  {
    id: "analysis",
    label: "Analysis",
    icon: ScanLine,
    eyebrow: "ANALYSIS WORKSPACE",
    description: "Inspect firmware structure without changing the original.",
    phase: 16,
  },
  {
    id: "backups",
    label: "Backup manager",
    icon: Archive,
    eyebrow: "AUTHORIZED ACQUISITION",
    description: "Read-only backups for verified, supported device profiles.",
    phase: 8,
  },
  {
    id: "builder",
    label: "Firmware builder",
    icon: Blocks,
    eyebrow: "OPENWRT BUILDER",
    description: "Create separate images for supported OpenWrt targets.",
    phase: 15,
  },
  {
    id: "jobs",
    label: "Jobs & activity",
    icon: ListChecks,
    eyebrow: "WORKSPACE ACTIVITY",
    description: "Follow operations and review the local audit trail.",
  },
  {
    id: "settings",
    label: "Settings",
    icon: Settings2,
    eyebrow: "PREFERENCES",
    description: "Make this workspace yours.",
  },
];

export function pageFromHash(): Page {
  const id = window.location.hash.slice(1);
  return (
    destinations.find((destination) => destination.id === id)?.id ?? "dashboard"
  );
}
