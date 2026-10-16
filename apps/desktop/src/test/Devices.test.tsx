import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { Devices } from "../features/Devices";
import type { DiscoverySnapshot } from "../types/contracts";

const unverifiedFeature = {
  state: "UNKNOWN" as const, controlPlane: null, canRead: false, canWrite: false,
  canVerify: false, canRollback: false, confidence: 0, evidence: [],
};
const inventory: DiscoverySnapshot = {
  revision: 4, scannedAt: 1_792_000_000_000, status: "ready", watching: true, warnings: [],
  devices: [{
    id: "a".repeat(64), displayName: "Test MiFi", connected: true, transports: ["usb", "serial", "network"],
    vendorId: "12D1", productId: "14DC", containerId: "11111111-1111-1111-1111-111111111111",
    interfaces: [{
      instanceId: "USB\\VID_12D1&PID_14DC\\FIXTURE", parentInstanceId: null,
      className: "Ports", classGuid: "PORTS-GUID", interfaceClasses: [], hardwareIds: ["USB\\VID_12D1&PID_14DC"],
      portName: "COM7", present: true,
    }],
    networks: [{ id: "fixture-adapter", displayName: "Test network", description: "USB network",
      interfaceIndex: 4, interfaceType: 6, linkUp: true, addresses: ["192.0.2.10"], gateways: ["192.0.2.1"] }],
    category: "router_mifi_candidate", assessment: "Possible router or MiFi", confidence: 70, evidence: ["USB identity match"],
    capabilities: { inventory: "SUPPORTED", backup: "UNKNOWN", flash: "UNSUPPORTED", features: {
      apn: unverifiedFeature, wifiUserLimit: unverifiedFeature, customDns: unverifiedFeature, ussd: unverifiedFeature,
    } }, lastSeenAt: 1_792_000_000_000,
  }, {
    id: "b".repeat(64), displayName: "Disconnected reader", connected: false, transports: ["usb"],
    vendorId: null, productId: null, containerId: null, interfaces: [], networks: [],
    category: "unrelated_device", assessment: "Unknown device", confidence: 0, evidence: ["Windows inventory only"],
    capabilities: { inventory: "SUPPORTED", backup: "UNKNOWN", flash: "UNSUPPORTED", features: {
      apn: unverifiedFeature, wifiUserLimit: unverifiedFeature, customDns: unverifiedFeature, ussd: unverifiedFeature,
    } }, lastSeenAt: 1_792_000_000_000,
  }],
};
const props = { inventory, desktop: true, error: null, refreshing: false, onRefresh: vi.fn(async () => {}) };

it("shows real device evidence, unknown backup support and unverified gateway", async () => {
  const user = userEvent.setup();
  render(<Devices {...props} />);
  expect(screen.getByText("1 present · 1 disconnected")).toBeInTheDocument();
  expect(screen.getByText("Possible router or MiFi")).toBeInTheDocument();
  const deviceCard = screen.getByRole("button", { name: /Test MiFi/ });
  expect(within(deviceCard).getByText("COM7")).toBeInTheDocument();
  await user.click(deviceCard);
  const details = screen.getByRole("region", { name: "Device details" });
  expect(within(details).getAllByText("UNKNOWN")).toHaveLength(1);
  expect(screen.getByRole("region", {name:"Device compatibility"})).toBeInTheDocument();
  expect(within(details).getByText("UNSUPPORTED")).toBeInTheDocument();
  expect(within(details).getByText("192.0.2.1")).toBeInTheDocument();
});

it("filters disconnected devices and connection types", async () => {
  const user = userEvent.setup();
  render(<Devices {...props} />);
  const search = screen.getByRole("textbox", { name: "Search devices" });
  await user.click(screen.getByRole("checkbox", {name: /Show advanced Windows/}));
  await user.type(search, "reader");
  expect(screen.getByRole("heading", { name: "Disconnected reader" })).toBeInTheDocument();
  await user.click(screen.getByRole("checkbox", { name: "Show disconnected" }));
  expect(screen.queryByRole("heading", { name: "Disconnected reader" })).not.toBeInTheDocument();
  await user.clear(search);
  await user.selectOptions(screen.getByRole("combobox", { name: "Filter by connection" }), "serial");
  expect(screen.getByRole("heading", { name: "Test MiFi" })).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Disconnected reader" })).not.toBeInTheDocument();
});

it("explains inventory scan errors and leaves browser preview hardware-free", () => {
  const { rerender } = render(<Devices {...props} error="Windows inventory could not be refreshed."/>);
  expect(screen.getByRole("alert")).toHaveTextContent("Windows inventory could not be refreshed.");
  rerender(<Devices {...props} inventory={null} desktop={false}/>);
  expect(screen.getByRole("heading", { name: "Open the Windows app to discover devices" })).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Refresh devices" })).not.toBeInTheDocument();
});

it("allows manual recovery when the initial inventory read fails", async () => {
  const onRefresh = vi.fn(async () => {});
  render(<Devices {...props} inventory={null} error="Initial read failed" onRefresh={onRefresh} />);
  expect(screen.getByRole("heading", { name: "Inventory unavailable" })).toBeInTheDocument();
  const refresh = screen.getByRole("button", { name: "Refresh devices" });
  expect(refresh).toBeEnabled();
  await userEvent.click(refresh);
  expect(onRefresh).toHaveBeenCalledOnce();
});

it("distinguishes a pending first scan from a successful empty inventory", () => {
  const { rerender } = render(<Devices {...props} inventory={null} />);
  expect(screen.getByText("Reading Windows device inventory…")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Refresh devices" })).toBeDisabled();
  rerender(<Devices {...props} inventory={{ ...inventory, devices: [] }} />);
  expect(screen.getByRole("heading", { name: "No devices to show" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Refresh devices" })).toBeEnabled();
});
