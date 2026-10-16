// Native Phase 2 acceptance. All synthetic responses come from a local test server,
// through the production URL validator, Rust WinHTTP transport, SQLite and real IPC.
import assert from 'node:assert/strict';
import http from 'node:http';
import https from 'node:https';
import { execFileSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

export async function verifyDiscovery(client, { label, evidence, record, routerAddress, hotplug }) {
  const invoke = (command, args = {}) => client.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`);
  const until = async (check, message) => {
    const deadline = Date.now() + 15000;
    while (Date.now() < deadline) {
      if (await check()) return;
      await new Promise((resolve) => setTimeout(resolve, 80));
    }
    throw new Error(message);
  };
  const discovery = await invoke('get_discovery');
  const candidates = discovery.devices.filter((d) => d.category.endsWith('_candidate'));
  assert(discovery.devices.every((d) => d.identity.confidence !== 'verified' && d.identity.model === null));
  const softwareOnly = discovery.devices.filter((d) => d.networks.every((n) => n.virtualInterface) && d.networks.length);
  assert(softwareOnly.every((d) => ['generic_network_interface', 'unrelated_device'].includes(d.category)),
    JSON.stringify(softwareOnly.map((d) => ({name: d.displayName, category: d.category, classes: d.interfaces.map((i) => i.className)}))));
  assert.equal(new Set(discovery.devices.map((d) => d.id)).size, discovery.devices.length);
  const networks = discovery.devices.filter((d) => d.connected).flatMap((d) => d.networks);
  const physical = networks.filter((n) => n.linkUp && !n.virtualInterface);
  const selected = physical.find((n) => n.gateways.includes('192.168.100.1')) ?? physical.find((n) => n.addresses.some((a) => /^192\.168\./.test(a)));
  assert(selected, 'A physical private interface is required for local IPC transport fixtures');
  // Keep fixtures on Wi-Fi when available so an intended USB unplug does not
  // invalidate a timeout fixture by removing its own server's network address.
  const fixtureNetwork = physical.find((n) => n.interfaceType === 71 && n.addresses.some((a) => /^192\.168\./.test(a))) ?? selected;
  const localIP = fixtureNetwork.addresses.find((a) => /^192\.168\./.test(a));
  assert(localIP);
  const summary = {
    records: discovery.devices.length, candidates: candidates.map((d) => ({id: d.id, name: d.displayName,
      category: d.category, identity: d.identity.confidence, vid: d.vendorId, pid: d.productId,
      interfaceClasses: d.interfaces.map((i) => i.className), manufacturers: [...new Set(d.interfaces.map((i) => i.manufacturer).filter(Boolean))],
      networkIndexes: d.networks.map((n) => n.interfaceIndex)})),
    networks: networks.map(({id, displayName, interfaceIndex, ipv6Index, luid, addresses, gateways, prefixes, dnsServers, virtualInterface, defaultRoutes, linkUp}) =>
      ({id, displayName, interfaceIndex, ipv6Index, luid, addresses, gateways, prefixes, dnsServers, virtualInterface, defaultRoutes, linkUp})),
    enumerationMs: discovery.durationMs, results: [],
  };
  record(`${label}: topology, categorical identity and virtual-interface filtering`, {candidates: candidates.length, records: discovery.devices.length, enumerationMs: discovery.durationMs});
  const savedBefore = await invoke('get_manual_routers');
  const owned = new Set();
  const save = async (address, name) => {
    const saved = await invoke('add_manual_router', {input: {address, label: name, interfaceId: fixtureNetwork.id}});
    const entry = saved.find((r) => r.address === new URL(address).href && r.interfaceId === fixtureNetwork.id);
    assert(entry);
    if (!savedBefore.some((r) => r.id === entry.id)) owned.add(entry.id);
    return entry;
  };
  let responseMode = 401;
  const requests = [];
  const sockets = new Set();
  const server = http.createServer((request, response) => {
    requests.push({method: request.method, path: request.url, authorization: Boolean(request.headers.authorization), cookie: Boolean(request.headers.cookie)});
    assert.equal(request.method, 'HEAD'); assert.equal(request.url, '/');
    assert.equal(request.headers.authorization, undefined); assert.equal(request.headers.cookie, undefined);
    if (responseMode === 'hold') return;
    response.writeHead(responseMode, {'Server': 'OBTAINER local fixture', 'Location': 'http://127.0.0.1:1/', 'Content-Length': '0'}); response.end();
  });
  server.on('connection', (socket) => { sockets.add(socket); socket.on('close', () => sockets.delete(socket)); });
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, localIP, resolve); });
  try {
    const address = `http://${localIP}:${server.address().port}/`;
    const fixture = await save(address, 'Phase 2 local test endpoint');
    const duplicate = await save(address, 'Phase 2 local test endpoint'); assert.equal(duplicate.id, fixture.id);
    assert.equal((await invoke('get_manual_routers')).filter((r) => r.id === fixture.id).length, 1);
    const denied = await client.evaluate(`window.__TAURI_INTERNALS__.invoke('probe_management_endpoint',{id:${JSON.stringify(fixture.id)},authorized:false}).then(()=>null,error=>error)`);
    assert.equal(denied.code, 'authorization_required'); assert.equal(requests.length, 0);
    for (const [status, code] of [[401, 'authentication_required'], [200, 'identity_unknown'], [302, 'redirect_blocked'], [405, 'unsupported_management']]) {
      responseMode = status;
      const result = await invoke('probe_management_endpoint', {id: fixture.id, authorized: true});
      assert.equal(result.code, code, JSON.stringify(result)); assert.equal(result.status, status);
      summary.results.push({fixture: true, ...result});
    }
    responseMode = 'hold';
    const timeout = await invoke('probe_management_endpoint', {id: fixture.id, authorized: true});
    assert.equal(timeout.code, 'timeout', JSON.stringify(timeout)); assert(timeout.elapsedMs < 6000); summary.results.push({fixture: true, ...timeout});
    const cancellation = await client.evaluate(`(async()=>{
      const pending=window.__TAURI_INTERNALS__.invoke('probe_management_endpoint',{id:${JSON.stringify(fixture.id)},authorized:true}).then(value=>({value}),error=>({error}));
      await new Promise(resolve=>setTimeout(resolve,150));
      const during=await window.__TAURI_INTERNALS__.invoke('get_manual_routers');
      await window.__TAURI_INTERNALS__.invoke('cancel_endpoint_probe',{id:${JSON.stringify(fixture.id)}});
      return {during:during.find(r=>r.id===${JSON.stringify(fixture.id)}).state, outcome:await pending};
    })()`);
    assert.equal(cancellation.during, 'identifying'); assert.equal(cancellation.outcome.error.code, 'cancelled');
    assert.equal((await invoke('get_manual_routers')).find((r) => r.id === fixture.id).lastResult.code, 'timeout');
    const keyPath = path.join(evidence, `${label}-fixture-key.pem`);
    const certPath = path.join(evidence, `${label}-fixture-cert.pem`);
    execFileSync(path.join(process.env.ProgramFiles, 'Git/usr/bin/openssl.exe'), ['req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', keyPath, '-out', certPath, '-days', '1', '-subj', '/CN=OBTAINER-local-test', '-addext', `subjectAltName=IP:${localIP}`], {windowsHide: true, stdio: 'ignore'});
    const tlsServer = https.createServer({key: await readFile(keyPath), cert: await readFile(certPath)}, (_request, response) => {response.writeHead(200); response.end();});
    tlsServer.on('tlsClientError', () => {});
    await new Promise((resolve, reject) => {tlsServer.once('error', reject); tlsServer.listen(0, localIP, resolve);});
    try {
      const tls = await save(`https://${localIP}:${tlsServer.address().port}/`, 'Phase 2 untrusted TLS fixture');
      const rejected = await invoke('probe_management_endpoint', {id: tls.id, authorized: true});
      assert.equal(rejected.code, 'tls_trust_failure'); assert.equal(rejected.tls, 'untrusted');
      summary.results.push({fixture: true, ...rejected});
      record(`${label}: native WinHTTP rejected untrusted local certificate`);
    } finally { tlsServer.closeAllConnections(); await new Promise((resolve) => tlsServer.close(resolve)); }
    // Reload the native React page to exercise persisted candidates through the UI.
    await client.send('Page.reload');
    await until(() => client.evaluate("document.querySelector('.backend-status')?.innerText === 'Desktop connected'"), 'React did not reconnect after reload');
    await client.evaluate("document.querySelector('a[href=\"#devices\"]').click()");
    await until(() => client.evaluate("document.querySelector('.saved-router') !== null"), 'Saved target did not render');
    responseMode = 401;
    await client.evaluate(`(()=>{const card=[...document.querySelectorAll('.saved-router')].find(el=>el.innerText.includes('Phase 2 local test endpoint')); card.querySelector('input[type=checkbox]').click(); card.querySelector('button').click();})()`);
    await until(() => client.evaluate("[...document.querySelectorAll('.saved-router')].some(el=>el.innerText.includes('Phase 2 local test endpoint') && el.innerText.includes('HTTP 401'))"), 'Real React check did not render authentication-required response');
    record(`${label}: native manual persistence, consent, HTTP states, deadline and cancellation`, {requests: requests.length, timeoutMs: timeout.elapsedMs});

    const cancelled = await client.evaluate(`(async()=>{
      const pending=window.__TAURI_INTERNALS__.invoke('refresh_discovery');
      await new Promise(resolve=>setTimeout(resolve,50));
      await window.__TAURI_INTERNALS__.invoke('cancel_discovery'); return await pending;
    })()`);
    assert.equal(cancelled.cancelled, true);
    const refreshed = await invoke('refresh_discovery'); assert.equal(refreshed.cancelled, false);
    assert.equal(new Set(refreshed.devices.map((d) => d.id)).size, refreshed.devices.length);
    record(`${label}: queued discovery cancellation and successful retry`);

    if (routerAddress && label === 'dev-reopen') {
      const previous = savedBefore.find((r) => r.address === new URL(routerAddress).href && r.interfaceId === selected.id);
      // User supplied the MG11 target. Save through real React form, then make one HEAD check.
      await client.evaluate("[...document.querySelectorAll('button')].find(el=>el.innerText==='Add Router').click()");
      await until(() => client.evaluate("document.querySelector('[role=dialog]') !== null"), 'Add Router dialog missing');
      await client.evaluate(`(()=>{const form=document.querySelector('[role=dialog] form'); const setter=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set;
        const fields=form.querySelectorAll('input'); setter.call(fields[0],${JSON.stringify(routerAddress)}); fields[0].dispatchEvent(new Event('input',{bubbles:true}));
        setter.call(fields[1],'comFi MG11 (user supplied)'); fields[1].dispatchEvent(new Event('input',{bubbles:true}));
        form.querySelector('select').value=${JSON.stringify(selected.id)}; form.querySelector('select').dispatchEvent(new Event('change',{bubbles:true})); form.requestSubmit();})()`);
      await until(async () => (await invoke('get_manual_routers')).some((r) => r.address === new URL(routerAddress).href && r.interfaceId === selected.id), 'React manual save did not persist');
      await until(() => client.evaluate("[...document.querySelectorAll('.saved-router')].some(el=>el.innerText.includes('comFi MG11'))"), 'MG11 user label did not render');
      const checkedAfter = Date.now();
      await client.evaluate("(()=>{const card=[...document.querySelectorAll('.saved-router')].find(el=>el.innerText.includes('comFi MG11')); card.querySelector('input').click(); card.querySelector('button').click();})()");
      let mg;
      await until(async () => { mg = (await invoke('get_manual_routers')).find((r) => r.address === new URL(routerAddress).href && r.interfaceId === selected.id); return mg?.lastResult?.checkedAt >= checkedAfter && mg.state !== 'identifying'; }, 'Fresh MG11 check did not finish');
      summary.results.push({fixture: false, userReportedModel: 'comFi MG11 Portable 4G Wi-Fi 6 MIFI', ...mg.lastResult});
      record('MG11 real read-only result; identity remains unverified', {preexisting: Boolean(previous), ...mg.lastResult});
      if (hotplug) {
        const physicalId = mg.deviceId;
        console.log('HOTPLUG READY: unplug MG11 USB, wait ten seconds, reconnect. Waiting up to two minutes per transition.');
        await writeFile(path.join(evidence, 'hotplug-ready.json'), JSON.stringify({physicalId, address: mg.address}));
        const transition = async (present) => {
          const deadline = Date.now() + 120000;
          while (Date.now() < deadline) {
            const current = await invoke('get_discovery');
            const device = current.devices.find((d) => d.id === physicalId);
            if (device && device.connected === present && (!present || device.networks.some((n) => n.linkUp))) return {revision: current.revision, id: device.id, present: device.connected, lifecycle: device.lifecycle, interfaceCount: device.interfaces.length};
            await new Promise((resolve) => setTimeout(resolve, 250));
          }
          return null;
        };
        const removed = await transition(false);
        const returned = removed ? await transition(true) : null;
        summary.hotplug = {removed, returned, verified: Boolean(removed && returned)};
        await writeFile(path.join(evidence, 'hotplug-result.json'), JSON.stringify(summary.hotplug, null, 2));
        record(summary.hotplug.verified ? 'MG11 physical removal/reconnection retained logical identity' : 'MG11 physical hot-plug qualification pending', summary.hotplug);
      }
    } else if (routerAddress) {
      const saved = (await invoke('get_manual_routers')).find((r) => r.address === new URL(routerAddress).href && r.interfaceId === selected.id);
      assert(saved?.lastResult, 'MG11 observation did not persist across restart');
      summary.results.push({persistedPhysicalObservation: true, ...saved.lastResult});
      record(`${label}: MG11 user entry and endpoint observation survived restart`, {state: saved.state});
    }
    await client.evaluate("[...document.querySelectorAll('.saved-router')].find(el=>el.innerText.includes('comFi MG11'))?.scrollIntoView({block:'center'})");
    await client.evaluate("new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))");
    const shot = await client.send('Page.captureScreenshot', {format: 'png', captureBeyondViewport: false});
    await writeFile(path.join(evidence, `${label}-devices.png`), Buffer.from(shot.data, 'base64'));
  } finally {
    for (const id of owned) await invoke('remove_manual_router', {id});
    for (const socket of sockets) socket.destroy();
    await new Promise((resolve) => server.close(resolve));
    summary.requests = requests;
    await writeFile(path.join(evidence, `${label}-discovery.json`), JSON.stringify(summary, null, 2));
  }
}

export async function watchUsb(client, {evidence, record}) {
  const invoke = (command) => client.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)})`);
  const entries = await invoke('get_manual_routers');
  const target = entries.find((r) => r.address === 'http://192.168.100.1/' && r.deviceId);
  assert(target, 'Saved MG11 association missing');
  let baseline;
  const start = Date.now();
  while (Date.now() - start < 20000) {
    baseline = await invoke('get_discovery');
    if (baseline.devices.find((d) => d.id === target.deviceId)?.connected) break;
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  assert(baseline.devices.find((d) => d.id === target.deviceId)?.connected, 'MG11 not present before monitored unplug');
  const trace = [];
  let removed = false, returned = false, previous;
  console.log('HOTPLUG READY: native app will remain open for five minutes. Unplug MG11 USB, wait ten seconds, reconnect.');
  await writeFile(path.join(evidence, 'hotplug-ready.json'), JSON.stringify({id: target.deviceId, baselineRevision: baseline.revision}));
  const deadline = Date.now() + 300000;
  while (Date.now() < deadline) {
    const current = await invoke('get_discovery');
    const device = current.devices.find((d) => d.id === target.deviceId);
    const value = device ? {id: device.id, connected: device.connected, lifecycle: device.lifecycle, interfaces: device.interfaces.filter((i) => i.present).length, networks: device.networks.map((n) => ({id: n.id, index: n.interfaceIndex, up: n.linkUp}))} : null;
    if (JSON.stringify(value) !== previous) {
      previous = JSON.stringify(value); trace.push({at: new Date().toISOString(), revision: current.revision, device: value});
      await writeFile(path.join(evidence, 'hotplug-trace.json'), JSON.stringify(trace, null, 2));
      console.log(`USB transition: ${JSON.stringify(value)}`);
    }
    if (device && !device.connected) removed = true;
    if (removed && device?.connected && device.networks.some((n) => n.linkUp)) {returned = true; assert.equal(current.devices.filter((d) => d.id === target.deviceId).length, 1); break;}
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  const outcome = {verified: removed && returned, removed, returned, trace};
  await writeFile(path.join(evidence, 'hotplug-result.json'), JSON.stringify(outcome, null, 2));
  record(outcome.verified ? 'MG11 physical removal and reconnection verified with same logical ID' : 'MG11 hot-plug remains unverified', outcome);
  if (!outcome.verified) throw new Error('Physical removal/reconnection was not observed in the five-minute window');
}
