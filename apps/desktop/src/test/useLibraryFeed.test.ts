import { act, renderHook } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { useLibraryFeed } from "../lib/useLibraryFeed";
import { onLibraryChanged } from "../lib/backend";
vi.mock("../lib/backend", () => ({ onLibraryChanged: vi.fn(async () => vi.fn()) }));
afterEach(() => vi.useRealTimers());

it("backs off idle polling, keeps active jobs responsive and cleans up", async () => {
  vi.useFakeTimers();
  const view = renderHook(({ busy }) => useLibraryFeed(true, busy), { initialProps: { busy: false } });
  await act(async () => { await vi.advanceTimersByTimeAsync(15000); });
  expect(view.result.current).toBe(0);
  await act(async () => { await vi.advanceTimersByTimeAsync(15200); });
  expect(view.result.current).toBe(1);
  view.rerender({ busy: true });
  await act(async () => { await vi.advanceTimersByTimeAsync(1700); });
  expect(view.result.current).toBe(2);
  const changed = vi.mocked(onLibraryChanged).mock.calls.at(-1)![0];
  act(() => changed({} as Parameters<typeof changed>[0]));
  await act(async () => { await vi.advanceTimersByTimeAsync(200); });
  expect(view.result.current).toBe(3);
  view.unmount();
  expect(vi.getTimerCount()).toBe(0);
});
