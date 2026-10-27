import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "../app/App";
import { getSnapshot, hasDesktopBackend, onDiscoveryChanged, getDiscovery, refreshDiscovery, saveSettings, onLibraryChanged, listFirmwareArtifacts, listLibraryJobs, getLibraryActivity } from "../lib/backend";
import { desktopState } from "./libraryFixtures";
import { getDesktopState, listFirmwareBackups } from "../lib/backend";
import type { DiscoverySnapshot, Snapshot } from "../types/contracts";

vi.mock("../lib/backend", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/backend")>();
  return {
    ...original,
    getDesktopState:vi.fn(),listFirmwareBackups:vi.fn(),
    getSnapshot: vi.fn(),
    hasDesktopBackend: vi.fn(),
    saveSettings: vi.fn(),
    getDiscovery: vi.fn(),
    refreshDiscovery: vi.fn(),
    onDiscoveryChanged: vi.fn(),
    onLibraryChanged: vi.fn(),listFirmwareArtifacts:vi.fn(),listLibraryJobs:vi.fn(),getLibraryActivity:vi.fn(),
  };
});

const snapshot: Snapshot = {
  version: "0.1.0",
  databaseVersion: 1,
  dataDirectory: "C:\\test\\OBTAINER",
  settings: { theme: "system" },
  counts: { devices: 0, artifacts: 0, runningJobs: 0 },
  recentEvents: [
    {
      id: 1,
      kind: "application_started",
      message: "Desktop workspace initialized",
      createdAt: "2026-10-14T10:00:00Z",
    },
  ],
};
it("labels synthetic workflow identity explicitly without implying physical qualification",async()=>{
  vi.mocked(getSnapshot).mockResolvedValue({...snapshot,testProfile:{runId:"c".repeat(32),discoverySource:"synthetic_workflow_fixture",simulatedAdapters:false}});
  render(<App/>);
  const banner=await screen.findByText(/SOFTWARE WORKFLOW FIXTURE/);
  expect(banner).toHaveTextContent("no physical device identity or acquisition has been verified");
});
const inventory: DiscoverySnapshot = {
  revision: 1, scannedAt: 1_792_000_000_000, status: "ready", watching: true, warnings: [], devices: [],
};

beforeEach(() => {
  vi.mocked(hasDesktopBackend).mockReturnValue(true);
  vi.mocked(getDesktopState).mockResolvedValue(desktopState);
  vi.mocked(listFirmwareBackups).mockResolvedValue([]);
  vi.mocked(getSnapshot).mockResolvedValue(snapshot);
  vi.mocked(saveSettings).mockResolvedValue({ theme: "dark" });
  vi.mocked(getDiscovery).mockResolvedValue(inventory);
  vi.mocked(refreshDiscovery).mockResolvedValue(inventory);
  vi.mocked(onDiscoveryChanged).mockResolvedValue(vi.fn());
  vi.mocked(onLibraryChanged).mockResolvedValue(vi.fn());
  vi.mocked(listFirmwareArtifacts).mockResolvedValue({artifacts:[],total:0,uniqueBytes:0,page:0,pageSize:25,maxFileBytes:2147483648});
  vi.mocked(listLibraryJobs).mockResolvedValue([]);vi.mocked(getLibraryActivity).mockResolvedValue([]);
});

describe("desktop foundation", () => {
  it("shows connecting state while the first workspace request is pending", () => {
    vi.mocked(getSnapshot).mockReturnValue(new Promise(() => {}));
    render(<App />);
    expect(screen.getByText("Connecting to workspace")).toBeInTheDocument();
    expect(screen.queryByText("Backend unavailable")).not.toBeInTheDocument();
  });

  it("marks a failed refresh disconnected while retaining cached workspace data", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Desktop connected");
    vi.mocked(getSnapshot).mockRejectedValue(new Error("Database unavailable"));
    await user.click(screen.getByRole("button", { name: "Refresh workspace" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Database unavailable");
    expect(screen.queryByText("Desktop connected")).not.toBeInTheDocument();
    expect(screen.getByText("Backend unavailable")).toBeInTheDocument();
    expect(screen.getByText(/0 stored artifacts/)).toBeInTheDocument();
    vi.mocked(getSnapshot).mockResolvedValue(snapshot);
    await user.click(screen.getByRole("button", { name: "Retry" }));
    expect(await screen.findByText("Desktop connected")).toBeInTheDocument();
  });

  it("refreshes the first completed inventory and ignores an older workspace reply", async () => {
    let resolveOld!: (value: Snapshot) => void;
    vi.mocked(getSnapshot).mockReturnValueOnce(new Promise((resolve) => { resolveOld = resolve; }));
    const fresh = { ...snapshot, version: "0.2.0" };
    vi.mocked(getSnapshot).mockResolvedValue(fresh);
    render(<App />);
    expect(await screen.findByText("OBTAINER · v0.2.0")).toBeInTheDocument();
    await act(async () => { resolveOld(snapshot); });
    expect(screen.getByText("OBTAINER · v0.2.0")).toBeInTheDocument();
  });

  it("retains an unsaved appearance choice when workspace metadata refreshes", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Desktop connected");
    await user.click(screen.getByRole("link", { name: "Settings" }));
    await user.click(screen.getByRole("radio", { name: /Dark/ }));
    vi.mocked(getSnapshot).mockResolvedValue({ ...snapshot });
    await user.click(screen.getByRole("button", { name: "Refresh workspace" }));
    await screen.findByText("Desktop connected");
    expect(screen.getByRole("radio", { name: /Dark/ })).toBeChecked();
    expect(screen.getByRole("button", { name: "Save preferences" })).toBeEnabled();
  });

  it("reports a confirmed settings save even if the following workspace read fails", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Desktop connected");
    await user.click(screen.getByRole("link", { name: "Settings" }));
    await user.click(screen.getByRole("radio", { name: /Dark/ }));
    vi.mocked(getSnapshot).mockRejectedValue(new Error("Read unavailable after commit"));
    await user.click(screen.getByRole("button", { name: "Save preferences" }));
    expect(screen.queryByText("Preferences saved to SQLite.")).not.toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent("Read unavailable after commit");
    expect(screen.getByRole("radio", { name: /Dark/ })).toBeChecked();
  });

  it("retains the draft after a failed save and lets the user retry saving", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Desktop connected");
    await user.click(screen.getByRole("link", { name: "Settings" }));
    await user.click(screen.getByRole("radio", { name: /Dark/ }));
    vi.mocked(saveSettings).mockRejectedValueOnce(new Error("Write unavailable"));
    await user.click(screen.getByRole("button", { name: "Save preferences" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Preferences could not be saved");
    expect(screen.getByText("Desktop connected")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Retry" })).not.toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /Dark/ })).toBeChecked();
    vi.mocked(getSnapshot).mockResolvedValue({ ...snapshot, settings: { theme: "dark" } });
    await user.click(screen.getByRole("button", { name: "Save preferences" }));
    expect(screen.queryByText("Preferences saved to SQLite.")).not.toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("reads native state and navigates the redesigned primary workspaces", async () => {
    const user = userEvent.setup();
    render(<App />);
    expect(await screen.findByText("Desktop connected")).toBeInTheDocument();
    expect(screen.queryByText("Desktop workspace initialized")).not.toBeInTheDocument();
    const pages = [
      ["Connected Devices", "No devices to show"],
      ["Library", "Firmware file library"],
      ["Backup Manager", "Preserved originals"],
      ["Settings", "Appearance"],
    ];
    for (const [label, heading] of pages) {
      await user.click(screen.getByRole("link", { name: label }));
      expect(
        screen.getByRole("heading", { name: heading }),
      ).toBeInTheDocument();
      expect(screen.getByRole("link", { name: label })).toHaveAttribute(
        "aria-current",
        "page",
      );
    }
    await user.click(screen.getByRole("link", { name: "Logs" }));
    expect(
      screen.getByRole("heading", { name: "Operation history" }),
    ).toBeInTheDocument();
  });

  it("saves preferences through validated IPC and applies the confirmed theme", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Desktop connected");
    await user.click(screen.getByRole("link", { name: "Settings" }));
    await user.click(screen.getByRole("radio", { name: /Dark/ }));
    vi.mocked(getSnapshot).mockResolvedValue({
      ...snapshot,
      settings: { theme: "dark" },
    });
    await user.click(screen.getByRole("button", { name: "Save preferences" }));
    expect(
      screen.queryByText("Preferences saved to SQLite."),
    ).not.toBeInTheDocument();
    expect(saveSettings).toHaveBeenCalledWith({ theme: "dark" });
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("shows backend errors and retries without inventing connected state", async () => {
    vi.mocked(getSnapshot).mockRejectedValue({
      message: "Database unavailable",
    });
    const user = userEvent.setup();
    render(<App />);
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Database unavailable",
    );
    expect(screen.queryByText("Desktop connected")).not.toBeInTheDocument();
    vi.mocked(getSnapshot).mockResolvedValue(snapshot);
    await user.click(screen.getByRole("button", { name: "Retry" }));
    expect(await screen.findByText("Desktop connected")).toBeInTheDocument();
  });

  it("labels browser preview and prevents native settings saves", async () => {
    vi.mocked(hasDesktopBackend).mockReturnValue(false);
    vi.mocked(getSnapshot).mockRejectedValue(
      new Error("Desktop backend unavailable"),
    );
    const user = userEvent.setup();
    render(<App />);
    expect(
      await screen.findByText("You’re viewing the interface preview"),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("link", { name: "Settings" }));
    await user.click(screen.getByRole("radio", { name: /Dark/ }));
    expect(
      screen.getByRole("button", { name: "Save preferences" }),
    ).toBeDisabled();
    await waitFor(() =>
      expect(document.documentElement.dataset.theme).toBe("dark"),
    );
    expect(saveSettings).not.toHaveBeenCalled();
  });
});

it("distinguishes isolated test data from real Windows observations",async()=>{
 vi.mocked(getSnapshot).mockResolvedValue({...snapshot,testProfile:{runId:"0123456789abcdef0123456789abcdef",discoverySource:"windows_observed",simulatedAdapters:false}});render(<App/>);expect(await screen.findByText(/ISOLATED NATIVE TEST/)).toHaveTextContent("no simulated adapters or verified device support");
});
