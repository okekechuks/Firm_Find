import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "../app/App";
import { getSnapshot, hasDesktopBackend, onDiscoveryChanged, getDiscovery, refreshDiscovery, saveSettings } from "../lib/backend";
import type { DiscoverySnapshot, Snapshot } from "../types/contracts";

vi.mock("../lib/backend", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/backend")>();
  return {
    ...original,
    getSnapshot: vi.fn(),
    hasDesktopBackend: vi.fn(),
    saveSettings: vi.fn(),
    getDiscovery: vi.fn(),
    refreshDiscovery: vi.fn(),
    onDiscoveryChanged: vi.fn(),
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
const inventory: DiscoverySnapshot = {
  revision: 1, scannedAt: 1_792_000_000_000, status: "ready", watching: true, warnings: [], devices: [],
};

beforeEach(() => {
  vi.mocked(hasDesktopBackend).mockReturnValue(true);
  vi.mocked(getSnapshot).mockResolvedValue(snapshot);
  vi.mocked(saveSettings).mockResolvedValue({ theme: "dark" });
  vi.mocked(getDiscovery).mockResolvedValue(inventory);
  vi.mocked(refreshDiscovery).mockResolvedValue(inventory);
  vi.mocked(onDiscoveryChanged).mockResolvedValue(vi.fn());
});

describe("desktop foundation", () => {
  it("reads native state and navigates every planned workspace", async () => {
    const user = userEvent.setup();
    render(<App />);
    expect(await screen.findByText("Desktop connected")).toBeInTheDocument();
    expect(
      screen.getByText("Desktop workspace initialized"),
    ).toBeInTheDocument();
    const pages = [
      ["Connected devices", "No devices to show"],
      ["Firmware library", "An intact original. Every time."],
      ["Analysis", "Understand what’s inside"],
      ["Backup manager", "Only supported, authorized backups"],
      ["Firmware builder", "Build for a supported target"],
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
    await user.click(screen.getByRole("link", { name: "Jobs & activity" }));
    expect(
      screen.getByRole("heading", { name: "No running jobs" }),
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
      await screen.findByText("Preferences saved to SQLite."),
    ).toBeInTheDocument();
    expect(saveSettings).toHaveBeenCalledWith({ theme: "dark" });
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("shows backend errors and retries without inventing connected state", async () => {
    vi.mocked(getSnapshot).mockRejectedValueOnce({
      message: "Database unavailable",
    });
    const user = userEvent.setup();
    render(<App />);
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Database unavailable",
    );
    expect(screen.queryByText("Desktop connected")).not.toBeInTheDocument();
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
