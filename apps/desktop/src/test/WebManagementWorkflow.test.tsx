import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { DeviceFirmwareWorkflow } from "../components/DeviceFirmwareWorkflow";
import { workflowFixture } from "./workflow-fixture";
import type { Device } from "../types/contracts";
import { WebManagementWorkflow } from "../components/WebManagementWorkflow";
import { webEvidenceFixture } from "./web-management-fixture";

// Synthetic frontend contract data. This is not a router or hardware test.
const api = vi.hoisted(() => ({ workflow: vi.fn(), evidence: vi.fn(), import: vi.fn(), analyze: vi.fn(), route: vi.fn() }));
vi.mock("../lib/backend", async original => ({
  ...await original<typeof import("../lib/backend")>(),
  getDeviceWorkflow: api.workflow, getWebUiEvidence: api.evidence,
  importWebUiCapture: api.import, analyzeWebUiBundle: api.analyze,
  assessWebManagementBinding: api.route,
}));
const feature = { state: "UNKNOWN" as const, controlPlane: null, canRead: false, canWrite: false, canVerify: false, canRollback: false, confidence: 0, evidence: [] };
const device: Device = { id: "a".repeat(64), displayName: "Synthetic MiFi", connected: true, transports: ["usb"], vendorId: null, productId: null, containerId: null, interfaces: [], networks: [], category: "router_mifi_candidate", assessment: "Candidate", confidence: 0, evidence: [], lastSeenAt: 1, capabilities: { inventory: "SUPPORTED", backup: "UNKNOWN", flash: "UNSUPPORTED", features: { apn: feature, wifiUserLimit: feature, customDns: feature, ussd: feature } } };

beforeEach(() => {
  vi.clearAllMocks();
  api.workflow.mockResolvedValue(workflowFixture(device));
  api.evidence.mockResolvedValue(webEvidenceFixture());
  api.import.mockResolvedValue(null);
  api.analyze.mockResolvedValue(null);
  api.route.mockResolvedValue({state:"observed",sourceIp:"192.0.2.2",interfaceId:"synthetic-interface",reason:"Local route observed; no device request sent.",cryptographicallyVerified:false,canAuthenticate:false});
});

it("offers local evidence import inside selected-device workflow without replay or credentials", async () => {
  const user = userEvent.setup();
  render(<DeviceFirmwareWorkflow deviceId={device.id} revision={4} connected managementOrigin="http://192.0.2.1/"/>);
  await user.click(await screen.findByText("Web UI Evidence"));
  expect(await screen.findByRole("button", { name: "Import Web UI Capture (HAR)" })).toBeEnabled();
  expect(screen.getByRole("button", { name: "Analyze Management UI" })).toBeEnabled();
  expect(screen.queryByRole("textbox", { name: /password/i })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Authenticate Read-Only" })).not.toBeInTheDocument();
  expect(screen.getByText(/No independently qualified read-only identity\/status route/)).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Obtain Firmware" })).toBeDisabled();
  expect(api.import).not.toHaveBeenCalled();
  expect(api.analyze).not.toHaveBeenCalled();
});

it("uses native local file selection only after an explicit action, and reports cancelled selection", async () => {
  const user = userEvent.setup();
  render(<DeviceFirmwareWorkflow deviceId={device.id} revision={4} connected/>);
  await user.click(await screen.findByText("Web UI Evidence"));
  await user.click(await screen.findByRole("button", { name: "Import Web UI Capture (HAR)" }));
  await waitFor(() => expect(api.import).toHaveBeenCalledWith(device.id));
  expect(await screen.findByText("No capture selected. Evidence unchanged.")).toBeInTheDocument();
  expect(api.analyze).not.toHaveBeenCalled();
});

it("keeps local parse errors separate from firmware extraction and allows retry", async () => {
  api.import.mockRejectedValue(new Error("Malformed HAR. No evidence saved."));
  const user = userEvent.setup();
  render(<DeviceFirmwareWorkflow deviceId={device.id} revision={4} connected/>);
  await user.click(await screen.findByText("Web UI Evidence"));
  await user.click(await screen.findByRole("button", { name: "Import Web UI Capture (HAR)" }));
  const evidence = screen.getByRole("region", { name: "Web management evidence" });
  expect(await within(evidence).findByRole("alert")).toHaveTextContent("Malformed HAR");
  expect(screen.queryByText("Firmware extraction failed.")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Import Web UI Capture (HAR)" })).toBeEnabled();
});

it("renders dangerous GET, correlated and static evidence without authorizing a request", async () => {
  const report = webEvidenceFixture();
  report.entries = [{ id: "c".repeat(64), origin: "http://192.0.2.1", path: "/reset.cgi", method: "GET", queryNames: ["action"], classification: "factory_reset", evidence: "both", readOnlyQualified: false, status: 200, mimeType: "text/html", requestContentType: null, requestBodyNames: [], selectedHeaders: {}, context: null }];
  report.authCandidates = ["digest"];
  api.import.mockResolvedValue(report);
  const user = userEvent.setup();
  render(<WebManagementWorkflow deviceId={device.id} revision={4} connected managementOrigin="http://192.0.2.1/"/>);
  await user.click(screen.getByText("Web UI Evidence"));
  await user.click(await screen.findByRole("button", { name: "Import Web UI Capture (HAR)" }));
  const dangerous = await screen.findByText("Potentially dangerous requests (1)");
  await user.click(dangerous);
  expect(within(dangerous.closest("details")!).getByText("GET /reset.cgi")).toBeInTheDocument();
  expect(within(dangerous.closest("details")!).getByText(/factory reset.*HAR observed \+ static candidate.*unverified/)).toBeInTheDocument();
  expect(screen.queryByRole("link", { name: /reset/ })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Authenticate Read-Only" })).not.toBeInTheDocument();
  expect(screen.getByText("Web-management acquisition: BLOCKED")).toBeInTheDocument();
  expect(api.analyze).not.toHaveBeenCalled();
});

it("analyzes local files with selected origin, never starts live collection", async () => {
  const user = userEvent.setup();
  render(<WebManagementWorkflow deviceId={device.id} revision={4} connected managementOrigin="http://192.0.2.1/"/>);
  await user.click(screen.getByText("Web UI Evidence"));
  await user.click(await screen.findByRole("button", { name: "Analyze Management UI" }));
  await waitFor(() => expect(api.analyze).toHaveBeenCalledWith(device.id, "http://192.0.2.1/"));
  expect(screen.getByText("No management files selected. Evidence unchanged.")).toBeInTheDocument();
  expect(api.import).not.toHaveBeenCalled();
});

it("does not expose import or authentication when backend permissions fail", async () => {
  api.evidence.mockRejectedValue(new Error("Selected device is stale."));
  const user = userEvent.setup();
  render(<WebManagementWorkflow deviceId={device.id} revision={4} connected managementOrigin="http://192.0.2.1/"/>);
  expect(api.evidence).not.toHaveBeenCalled();
  await user.click(screen.getByText("Web UI Evidence"));
  expect(await screen.findByRole("alert")).toHaveTextContent("Selected device is stale.");
  expect(screen.queryByRole("button", { name: "Import Web UI Capture (HAR)" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Authenticate Read-Only" })).not.toBeInTheDocument();
});

it("hides local actions when backend explicitly denies those permissions", async () => {
  const denied = webEvidenceFixture();
  denied.actions.importHar = false;
  denied.actions.analyzeLocalBundle = false;
  api.evidence.mockResolvedValue(denied);
  const user = userEvent.setup();
  render(<WebManagementWorkflow deviceId={device.id} revision={4} connected managementOrigin="http://192.0.2.1/"/>);
  await user.click(screen.getByText("Web UI Evidence"));
  await screen.findByText(/No independently qualified read-only identity\/status route/);
  expect(screen.queryByRole("button", { name: "Import Web UI Capture (HAR)" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Analyze Management UI" })).not.toBeInTheDocument();
});

it("does not publish a pending import into a changed selected-device generation", async () => {
  const user = userEvent.setup();
  const view = render(<WebManagementWorkflow deviceId={device.id} revision={4} connected managementOrigin="http://192.0.2.1/"/>);
  await user.click(screen.getByText("Web UI Evidence"));
  let finish!: (value: ReturnType<typeof webEvidenceFixture>) => void;
  api.import.mockReturnValueOnce(new Promise<ReturnType<typeof webEvidenceFixture>>(resolve => { finish = resolve; }));
  await user.click(await screen.findByRole("button", { name: "Import Web UI Capture (HAR)" }));
  view.rerender(<WebManagementWorkflow deviceId={device.id} revision={5} connected managementOrigin="http://192.0.2.1/"/>);
  const old = webEvidenceFixture(); old.warnings = ["Stale result must remain hidden."];
  finish(old);
  await waitFor(() => expect(api.evidence).toHaveBeenCalledTimes(2));
  expect(screen.queryByText("Stale result must remain hidden.")).not.toBeInTheDocument();
});

it("shows backend source assessments and exact missing requirements without granting acquisition", async () => {
  const report = webEvidenceFixture();
  report.entries = [
    { id: "c".repeat(64), origin: "http://192.0.2.1", path: "/config/export", method: "GET", queryNames: [], classification: "config_backup", evidence: "har_observed", readOnlyQualified: false, status: 200, mimeType: "application/octet-stream", requestContentType: null, requestBodyNames: [], selectedHeaders: {}, context: null },
    { id: "d".repeat(64), origin: "http://192.0.2.1", path: "/firmware/download", method: "GET", queryNames: [], classification: "firmware_download", evidence: "both", readOnlyQualified: false, status: 200, mimeType: "application/octet-stream", requestContentType: null, requestBodyNames: [], selectedHeaders: {}, context: null },
  ];
  api.import.mockResolvedValue(Object.assign(report, { strategy: { strategy: "web_management_acquisition", canAcquire: false, sources: [
    { evidenceId: "c".repeat(64), sourceType: "config_export", state: "not_firmware", readOnlyQualified: false, firmwareCompatible: false, reason: "Backend: this configuration export is not firmware." },
    { evidenceId: "d".repeat(64), sourceType: "observed_firmware_download", state: "requires_qualification", readOnlyQualified: false, firmwareCompatible: false, reason: "Backend: observed download requires independently qualified model and revision." },
  ], missingRequirements: ["Backend: exact interface and firmware-source descriptor are missing."] } }));
  const user = userEvent.setup();
  render(<WebManagementWorkflow deviceId={device.id} revision={4} connected managementOrigin="http://192.0.2.1/"/>);
  await user.click(screen.getByText("Web UI Evidence"));
  await user.click(await screen.findByRole("button", { name: "Import Web UI Capture (HAR)" }));
  const assessment = await screen.findByRole("region", { name: "Web-management source assessment" });
  expect(within(assessment).getByText("config export: not firmware")).toBeInTheDocument();
  expect(within(assessment).getByText("observed firmware download: requires qualification")).toBeInTheDocument();
  expect(within(assessment).getByText("Backend: exact interface and firmware-source descriptor are missing.")).toBeInTheDocument();
  expect(within(assessment).getByText("Backend: this configuration export is not firmware.")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: /download firmware|authenticate read-only|acquire/i })).not.toBeInTheDocument();
});

it("shows local route evidence without authorizing authentication", async () => {
  const user = userEvent.setup();
  render(<WebManagementWorkflow {...{deviceId:device.id,revision:4,connected:true,managementOrigin:"http://192.0.2.1/",managementInterfaceId:"synthetic-interface"}}/>);
  await user.click(screen.getByText("Web UI Evidence"));
  await user.click(await screen.findByRole("button", {name:"Check selected route"}));
  await waitFor(()=>expect(api.route).toHaveBeenCalledWith(device.id,"http://192.0.2.1/","synthetic-interface"));
  expect(await screen.findByText("Local route observed; no device request sent.")).toBeInTheDocument();
  expect(screen.getByText("Selected local source: 192.0.2.2")).toBeInTheDocument();
  expect(screen.queryByRole("button",{name:"Authenticate Read-Only"})).not.toBeInTheDocument();
});
