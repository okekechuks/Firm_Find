import { createContext, useContext } from "react";
import type { DesktopState, JobRecord } from "../types/library";
import type { getLibraryActivity } from "./backend";
export type ToastTone = "info" | "success" | "warning" | "error";
export interface WorkspaceValue {
  jobs: JobRecord[]; activity: Awaited<ReturnType<typeof getLibraryActivity>>; state: DesktopState | null;
  loading: boolean; error: string | null; revision: number; refresh: () => Promise<void>;
  cancel: (id: string) => Promise<void>; notify: (message: string, tone?: ToastTone) => void;
}
export const WorkspaceContext = createContext<WorkspaceValue | null>(null);
export const LibraryRevisionContext = createContext<number | null>(null);
export function useWorkspace() { return useContext(WorkspaceContext); }
export const isActiveJob = (job: JobRecord) => job.status === "pending" || job.status === "running";
