import type { Device } from "../types/contracts";
export function deviceStatus(device: Device): { tone: "success" | "warning" | "error"; label: string } {
  const protectedClasses = /^(diskdrive|hdc|scsiadapter|hidclass|keyboard|mouse|monitor|display|processor|system|computer|battery)$/i;
  if (device.category === "unrelated_device" || device.interfaces.some(i => protectedClasses.test(i.className))) return { tone: "error", label: "Protected hardware" };
  const candidate = ["router_mifi_candidate", "usb_modem_candidate", "network_gateway_candidate"].includes(device.category ?? "");
  if (candidate && device.identity?.confidence === "verified") return { tone: "success", label: "Recognized router / MiFi" };
  if (candidate || !device.category || device.category === "unknown") return { tone: "warning", label: "Possible candidate" };
  return { tone: "error", label: "Protected / unsupported" };
}
export function extractionEligibility(device: Device) {
  if (!device.connected) return { allowed: false, reason: "This device is disconnected. Reconnect it and refresh inventory." };
  if (deviceStatus(device).tone === "error") return { allowed: false, reason: "Protected or unsupported hardware. OBTAINER cannot extract or modify this device." };
  return { allowed: false, reason: "No verified device-specific acquisition adapter is available. Detection or a recognized identity does not enable firmware extraction." };
}
