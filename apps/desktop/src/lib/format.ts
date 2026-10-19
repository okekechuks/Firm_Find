export const human = (value: string) => value.replaceAll("_", " ");
export function formatBytes(bytes: number) {
  if (!bytes) return "0 B";
  const unit = Math.min(3, Math.floor(Math.log(bytes) / Math.log(1024)));
  return `${(bytes / 1024 ** unit).toLocaleString(undefined, { maximumFractionDigits: unit ? 1 : 0 })} ${["B", "KB", "MB", "GB"][unit]}`;
}
export function firmwareKind(artifact: { kind?: string; sourceCategory: string }) {
  return artifact.kind ?? ({ user_import: "imported_original", official_vendor_download: "imported_original", authorized_device_acquisition: "extracted", generated_build: "build_output" }[artifact.sourceCategory] ?? "unknown");
}
export const kindLabels: Record<string, string> = { imported_original: "Imported Original", extracted: "Extracted", customized: "Customized", restored_default: "Restored Default", build_output: "Build Output", unknown: "Unknown origin" };

export function defaultName(name: string) {
  const dot = name.lastIndexOf(".");
  const stem=dot>0?name.slice(0,dot):name;
  return `${stem.endsWith("_DEFAULT")?stem:`${stem}_DEFAULT`}${dot>0?name.slice(dot):""}`;
}
