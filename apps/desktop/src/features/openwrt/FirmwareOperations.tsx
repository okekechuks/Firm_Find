import type { FirmwareAssessment, FirmwareFeasibility } from "../../types/openwrt";

function Assessment({ name, assessment }: {name: string; assessment: FirmwareAssessment}) {
  return <article className="openwrt-assessment">
    <div className="section-heading"><h4>{name}</h4><span className="badge">{assessment.status.replaceAll("_", " ")}</span></div>
    <p>{assessment.reason}</p>
    {assessment.evidence.length || assessment.missingRequirements.length ? <details>
      <summary>{name} evidence and requirements</summary>
      {assessment.evidence.length ? <><h5>Evidence</h5><ul>{assessment.evidence.map((item, index) => <li key={index}>{item}</li>)}</ul></> : null}
      {assessment.missingRequirements.length ? <><h5>Still required</h5><ul>{assessment.missingRequirements.map((item, index) => <li key={index}>{item}</li>)}</ul></> : null}
    </details> : null}
  </article>;
}

export function FirmwareOperations({ report }: {report: FirmwareFeasibility}) {
  const assessments: [string, FirmwareAssessment][] = [
    ["Firmware acquisition", report.acquisition], ["Physical extraction", report.extraction],
    ["Partition layout", report.partitionLayout], ["Image formats", report.imageFormat],
    ["Firmware rebuilding", report.build], ["Firmware installation", report.installation], ["Recovery", report.recovery],
  ];
  return <section aria-label="Firmware Operations" className="openwrt-report">
    <h3>Firmware Operations</h3><p>Feasibility assessment · no firmware image or backup created.</p>
    <div className="openwrt-report-grid">{assessments.map(([name, assessment]) => <Assessment key={name} name={name} assessment={assessment}/>)}</div>
    <details><summary>Customization pathways</summary><div className="openwrt-report-grid">{([
      ["dns", "DNS"], ["wifiUserLimit", "Wi-Fi client limits"], ["apn", "APN"], ["ussd", "USSD"],
    ] as const).map(([key, name]) => <section key={key}><h4>{name}</h4><Assessment name="Direct configuration" assessment={report.customization[key].directConfiguration}/><Assessment name="Firmware rebuild" assessment={report.customization[key].firmwareRebuild}/></section>)}</div></details>
    {report.partitions.length ? <details><summary>Observed partition metadata</summary><ul>{report.partitions.map(item => <li key={item.identifier}>{item.identifier}: {item.name} · {item.bytes.toLocaleString()} bytes</li>)}</ul><p>Metadata does not prove complete firmware coverage or installability.</p></details> : null}
    <details><summary>Assessment limits</summary><p>{report.confidence}</p><ul>{report.restrictions.map((item, index) => <li key={index}>{item}</li>)}</ul><p>Observed {new Date(report.lastObserved).toLocaleString()}</p></details>
  </section>;
}
