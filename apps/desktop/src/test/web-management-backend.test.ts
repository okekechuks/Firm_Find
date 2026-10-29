import { beforeEach, expect, it, vi } from "vitest";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { analyzeWebUiBundle, getWebUiEvidence, importWebUiCapture } from "../lib/backend";
import { webEvidenceFixture } from "./web-management-fixture";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: vi.fn() }));
const id = "a".repeat(64);
beforeEach(() => { vi.clearAllMocks(); vi.mocked(isTauri).mockReturnValue(true); vi.mocked(invoke).mockResolvedValue(webEvidenceFixture()); });

it("uses only fixed local evidence IPC commands and binds each response to selected device", async () => {
  await getWebUiEvidence(id); expect(invoke).toHaveBeenLastCalledWith("get_web_ui_evidence", { deviceId: id });
  await importWebUiCapture(id); expect(invoke).toHaveBeenLastCalledWith("import_web_ui_capture", { deviceId: id });
  await analyzeWebUiBundle(id, "http://192.0.2.1/"); expect(invoke).toHaveBeenLastCalledWith("analyze_web_ui_bundle", { deviceId: id, origin: "http://192.0.2.1/" });
  vi.mocked(invoke).mockResolvedValue(webEvidenceFixture("b".repeat(64)));
  await expect(importWebUiCapture(id)).rejects.toThrow("another device");
});

it("rejects executable evidence, authentication authority and unexpected secret fields", async () => {
  for (const value of [
    { ...webEvidenceFixture(), canAcquire: true },
    { ...webEvidenceFixture(), actions: { ...webEvidenceFixture().actions, authenticate: true } },
    { ...webEvidenceFixture(), authorization: "synthetic-secret" },
  ]) {
    vi.mocked(invoke).mockResolvedValue(value);
    await expect(getWebUiEvidence(id)).rejects.toThrow();
  }
});

it("rejects secret-bearing headers and qualified claims from evidence records", async () => {
  const entry = { id: "c".repeat(64), origin: "http://192.0.2.1", path: "/status.cgi", method: "GET", queryNames: [], classification: "status_read", evidence: "har_observed", readOnlyQualified: false, status: 200, mimeType: "text/xml", requestContentType: null, requestBodyNames: [], selectedHeaders: {}, context: null };
  for (const change of [{ selectedHeaders: { authorization: "synthetic-secret" } }, { selectedHeaders: { "set-cookie": "synthetic-session" } }, { readOnlyQualified: true }, { responseBody: "synthetic-private-data" }]) {
    vi.mocked(invoke).mockResolvedValue({ ...webEvidenceFixture(), entries: [{ ...entry, ...change }] });
    await expect(getWebUiEvidence(id)).rejects.toThrow();
  }
});

it("refuses browser preview and invalid device ID before local evidence IPC", async () => {
  await expect(importWebUiCapture("invalid")).rejects.toThrow("current Windows device");
  vi.mocked(isTauri).mockReturnValue(false);
  await expect(getWebUiEvidence(id)).rejects.toThrow("Windows app");
  expect(invoke).not.toHaveBeenCalled();
});
