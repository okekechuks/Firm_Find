import { beforeEach, expect, it, vi } from 'vitest';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { getSnapshot, saveSettings } from '../lib/backend';
import type { Settings } from '../types/contracts';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }));
beforeEach(() => { vi.mocked(isTauri).mockReturnValue(true); });

it('rejects malformed native responses', async () => {
  vi.mocked(invoke).mockResolvedValue({ version: '0.1.0', counts: { devices: -1 } });
  await expect(getSnapshot()).rejects.toThrow();
});

it('rejects invalid preferences before invoking Rust', async () => {
  await expect(saveSettings({ theme: 'invalid' } as unknown as Settings)).rejects.toThrow();
  expect(invoke).not.toHaveBeenCalled();
});

it('never calls IPC from browser preview', async () => {
  vi.mocked(isTauri).mockReturnValue(false);
  await expect(getSnapshot()).rejects.toThrow('Desktop backend unavailable');
  expect(invoke).not.toHaveBeenCalled();
});
