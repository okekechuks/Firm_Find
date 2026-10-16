import { StrictMode, type ReactNode } from "react";
import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi, type Mock } from "vitest";
import { getDiscovery, onDiscoveryChanged, refreshDiscovery } from "../lib/backend";
import { useDeviceInventory } from "../lib/useDeviceInventory";
import type { DiscoverySnapshot } from "../types/contracts";

vi.mock("../lib/backend", () => ({
  errorMessage: (error: unknown) => (error as Error).message,
  getDiscovery: vi.fn(), onDiscoveryChanged: vi.fn(), refreshDiscovery: vi.fn(),
}));
const snapshot: DiscoverySnapshot = {
  revision: 1, scannedAt: 1, status: "ready", watching: true, warnings: [], devices: [],
};
let changed: () => void;
beforeEach(() => {
  vi.mocked(getDiscovery).mockResolvedValue(snapshot);
  vi.mocked(refreshDiscovery).mockResolvedValue(snapshot);
  vi.mocked(onDiscoveryChanged).mockImplementation(async (callback) => {
    changed = callback;
    return vi.fn<() => void>();
  });
});

it("clears a transient read failure when a later notification succeeds", async () => {
  vi.mocked(getDiscovery).mockRejectedValueOnce(new Error("Temporary read failure"));
  const { result } = renderHook(() => useDeviceInventory(true));
  await waitFor(() => expect(result.current.error).toBe("Temporary read failure"));
  await act(async () => { changed(); });
  await waitFor(() => expect(result.current.inventory?.revision).toBe(1));
  expect(result.current.error).toBeNull();
});

it("preserves a failed live-subscription warning after a successful manual refresh", async () => {
  vi.mocked(onDiscoveryChanged).mockRejectedValue(new Error("Cannot subscribe"));
  const { result } = renderHook(() => useDeviceInventory(true));
  await waitFor(() => expect(result.current.error).toContain("Live updates are unavailable"));
  await act(async () => { await result.current.refresh(); });
  expect(result.current.error).toContain("Live updates are unavailable");
});

it("keeps the newer revision when an older notification read arrives late", async () => {
  const { result } = renderHook(() => useDeviceInventory(true));
  await waitFor(() => expect(result.current.inventory?.revision).toBe(1));
  let resolveOld!: (value: DiscoverySnapshot) => void;
  vi.mocked(getDiscovery).mockReturnValueOnce(new Promise((resolve) => { resolveOld = resolve; }));
  act(() => { changed(); });
  vi.mocked(refreshDiscovery).mockResolvedValue({ ...snapshot, revision: 3 });
  await act(async () => { await result.current.refresh(); });
  await act(async () => { resolveOld({ ...snapshot, revision: 2 }); });
  expect(result.current.inventory?.revision).toBe(3);
});

it("cleans up subscriptions under StrictMode and after unmount", async () => {
  const stops: Mock<() => void>[] = [];
  vi.mocked(onDiscoveryChanged).mockImplementation(async () => {
    const stop = vi.fn<() => void>(); stops.push(stop); return stop;
  });
  const wrapper = ({ children }: { children: ReactNode }) => <StrictMode>{children}</StrictMode>;
  const { unmount } = renderHook(() => useDeviceInventory(true), { wrapper });
  await waitFor(() => expect(stops.length).toBeGreaterThan(0));
  unmount();
  expect(stops.every((stop) => stop.mock.calls.length === 1)).toBe(true);
});

it("cleans up a subscription that resolves after unmount without reading inventory", async () => {
  const stop = vi.fn();
  let resolveSubscription!: (value: () => void) => void;
  vi.mocked(onDiscoveryChanged).mockReturnValue(new Promise((resolve) => { resolveSubscription = resolve; }));
  const { unmount } = renderHook(() => useDeviceInventory(true));
  unmount();
  await act(async () => { resolveSubscription(stop); });
  expect(stop).toHaveBeenCalledOnce();
  expect(getDiscovery).not.toHaveBeenCalled();
});
