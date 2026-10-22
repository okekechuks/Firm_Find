import assert from 'node:assert/strict';
export async function verifyDeviceGUI(client, {label, record, screenshot}) {
  const evaluate = source => client.evaluate(source);
  async function until(check, message) {
    const end=Date.now()+20000;
    while(Date.now()<end){if(await check())return;await new Promise(resolve=>setTimeout(resolve,150));}
    throw new Error(message);
  }
  assert.equal(await evaluate('Array.from(document.querySelectorAll(".device-toolbar button")).some(b=>/Add Router|Manage saved connections|Inspect OpenWrt/.test(b.textContent))'),false);
  assert.equal(await evaluate('document.querySelector(".disconnected-toggle input").checked'),false);
  if(!await evaluate('!!document.querySelector(".device-card")')) {
    await evaluate('document.querySelector(".other-interfaces").open=true; document.querySelector(".other-interfaces input").click()');
    await until(()=>evaluate('!!document.querySelector(".device-card")'),'No Windows device available for native popup check');
  }
  const size=await evaluate('({width:document.querySelector(".device-card").getBoundingClientRect().width,text:document.querySelector(".device-card").innerText})');
  assert(size.width<=341,JSON.stringify(size));
  assert(!/Check compatibility|Possible candidate|Currently present/.test(size.text));
  await screenshot(label+'-compact-devices');
  await evaluate('document.querySelector(".device-card button").click()');
  await until(()=>evaluate('!!document.querySelector(".device-summary-dialog[open]")'),'Device summary did not open');
  assert.equal(await evaluate('!!document.querySelector(".device-summary-dialog .device-network-summary")'),true);
  assert.equal(await evaluate('/Identification|Windows indexes|Setup class|Hardware IDs|Container:/.test(document.querySelector(".device-summary-dialog").innerText)'),false);
  await evaluate('Array.from(document.querySelectorAll(".device-summary-dialog button")).find(b=>b.textContent==="Verify Compatibility").click()');
  await until(()=>evaluate('document.querySelector(".device-summary-dialog [role=alert] strong")?.textContent==="Cannot locate firmware"'),'Actual extraction compatibility check did not report unavailable firmware');
  assert.equal(await evaluate('Array.from(document.querySelectorAll(".device-summary-dialog button")).find(b=>b.textContent.includes("Extract Firmware")).disabled'),true);
  await screenshot(label+'-device-network-summary');
  await evaluate('document.querySelector(".device-summary-dialog .icon-button").click()');
  record(label+': compact native device cards, network-only popup, actual compatibility IPC and unavailable extraction');
}

