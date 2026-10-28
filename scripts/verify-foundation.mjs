import { verifyExtraction } from "./verify-extraction.mjs";
import { verifyUniversalWorkflow } from "./verify-universal-workflow.mjs";
import { verifyExtractionReview } from "./verify-extraction-review.mjs";
import { verifyAuthChallenge } from "./verify-auth-challenge.mjs";
import { verifyManufacturerPackage } from "./verify-manufacturer-package.mjs";
import { verifyGUI, verifyGUIRestart, configureGUIEvidence } from "./verify-gui.mjs";
import { verifyAdapters } from "./verify-adapters.mjs";
import { verifySimulator } from "./verify-simulator.mjs";
import { verifyOpenWrt } from "./verify-openwrt.mjs";
import { verifyDeviceGUI } from "./verify-device-gui.mjs";
import { verifyOfficialAcquisition } from "./verify-official-acquisition.mjs";
// Exercises the actual Windows Tauri dev command and release executable through native WebView2.
import assert from 'node:assert/strict';
import { verifyDiscovery, watchUsb } from './verify-discovery.mjs';
import { verifyProfiles } from './verify-profiles.mjs';
import { configureFirmwareTestRun, verifyFirmware, interruptFirmwareImport, verifyFirmwareHardening, startShutdownVerification } from './verify-firmware.mjs';
import { spawn, spawnSync, execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile, rename, realpath, lstat } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createTestProfile, probeNormal, cleanupProcess, verifyCleanupGuards, verifyActivationGuards, assertOwned } from './native-test-profile.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const attach = process.argv.includes('--attach-dev');
assert(!attach,'Attaching to an existing app is disabled: native verification requires an owned isolated profile.');
const profile=createTestProfile(root);
configureFirmwareTestRun(profile);
const settingsRoundtrip = process.argv.includes('--settings-roundtrip');
const evidence = path.join(root, '.tools/foundation', new Date().toISOString().replaceAll(':', '-'));
await mkdir(evidence, { recursive: true });
configureGUIEvidence(path.join(evidence,'gui'));
const results = [], consoleEvents = [], commands = [], launches = [];
const env = { ...process.env, OBTAINER_TEST_ROOT: root, OBTAINER_NATIVE_TEST_RUN_ID:profile.id, CARGO_INCREMENTAL: '0',
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9223' };
const preferencePath = path.join(root, 'apps/desktop/src/features/Preferences.tsx');
const backendPath = path.join(root, 'apps/desktop/src/lib/backend.ts');
let client, originalSource, changedSource, originalSettings;
let originalBackend, changedBackend;
console.log(`Evidence: ${evidence}`);

function record(name, details = {}) {
  results.push({ name, ...details });
  console.log(`PASS: ${name} ${JSON.stringify(details)}`);
}
async function until(check, message, timeout = 30000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await check()) return;
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  throw new Error(message);
}
async function targets() {
  try { return await (await fetch('http://127.0.0.1:9223/json/list', { signal: AbortSignal.timeout(1000) })).json(); }
  catch { return []; }
}
async function serverUp() {
  try { return (await fetch('http://127.0.0.1:1420/', { signal: AbortSignal.timeout(1000) })).ok; }
  catch { return false; }
}
function powershell(source) {
  return execFileSync('powershell.exe', ['-NoProfile', '-Command', source], { cwd: root, env, windowsHide: true, encoding: 'utf8' }).trim();
}
function processes() {
  return JSON.parse(powershell(`$ErrorActionPreference='Stop'; $apps=@(Get-Process -Name obtainer,obtainer-native-test -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path.StartsWith((Join-Path $env:OBTAINER_TEST_ROOT 'target\\'),[StringComparison]::OrdinalIgnoreCase) } | ForEach-Object { [pscustomobject]@{id=$_.Id; window=$_.MainWindowHandle.ToInt64(); path=$_.Path} }); $vite=@(Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'node.exe' -and $_.CommandLine -like ('*'+$env:OBTAINER_TEST_ROOT+'*') -and $_.CommandLine -match '[\\\\/]vite[\\\\/]bin[\\\\/]vite\\.js' } | ForEach-Object { $_.ProcessId }); $webviews=@(Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'msedgewebview2.exe' -and $_.CommandLine -like ('*'+$env:OBTAINER_NATIVE_TEST_RUN_ID+'*') } | ForEach-Object { $_.ProcessId }); [pscustomobject]@{apps=$apps;vite=$vite;webviews=$webviews} | ConvertTo-Json -Depth 4`));
}
async function clean() {
  await until(async () => !(await serverUp()) && (await targets()).length === 0, 'Native session or Vite listener did not stop');
  let state;await until(()=>{state=processes();return state.apps.length===0&&state.vite.length===0&&state.webviews.length===0;},'Owned native/WebView/Vite processes did not stop');
  assert.equal(state.apps.length, 0, 'OBTAINER process remains');
  assert.equal(state.vite.length, 0, 'Vite process remains');
  record('shutdown left no app, owned WebView process, Vite process or development listener');
}
function start(kind) {
  let command = kind === 'dev' ? 'npm.cmd run tauri -- dev --config src-tauri/tauri.native-test.conf.json --features native-test-profile -- --bin obtainer-native-test --locked --offline' : '.\\target\\release\\obtainer-native-test.exe';
  const appFlags=[];
  if(process.argv.includes('--phase6'))appFlags.push('--simulator');
  if(process.argv.includes('--phase7'))appFlags.push('--openwrt-lab');
  if(process.argv.includes('--universal-workflow'))appFlags.push('--workflow-lab');
  if(appFlags.length)command+=(kind==='dev'?' -- ':' ')+appFlags.join(' ');
  commands.push({ command, kind });
  const process_ = spawn('powershell.exe', ['-NoProfile', '-Command', command], {
    cwd: root, env, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'],
  });
  const launch = { process: process_, kind, output: '', exitCode: undefined };
  process_.stdout.on('data', (value) => { launch.output += value; });
  process_.stderr.on('data', (value) => { launch.output += value; });
  process_.on('exit', (code) => { launch.exitCode = code; });
  launches.push(launch);
  return launch;
}
async function connect() {
  let target;
  await until(async () => {
    target = (await targets()).find((item) => item.type === 'page' &&
      (item.url.startsWith('http://127.0.0.1:1420/') || item.url.startsWith('http://tauri.localhost/')));
    return Boolean(target);
  }, 'Native WebView target did not appear', 180000);
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Native CDP connection timed out')), 10000);
    socket.addEventListener('open', () => { clearTimeout(timer); resolve(); }, { once: true });
    socket.addEventListener('error', () => { clearTimeout(timer); reject(new Error('Native CDP connection failed')); }, { once: true });
  });
  const pending = new Map();
  let nextId = 0;
  socket.addEventListener('message', (event) => {
    const value = JSON.parse(event.data);
    if (value.id) {
      const request = pending.get(value.id);
      if (request) {
        clearTimeout(request.timer); pending.delete(value.id);
        if (value.error) request.reject(new Error(JSON.stringify(value.error)));
        else request.resolve(value.result);
      }
    } else if (['Runtime.consoleAPICalled', 'Runtime.exceptionThrown', 'Log.entryAdded'].includes(value.method)) {
      consoleEvents.push({ url: target.url, ...value });
    }
  });
  const send = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++nextId;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 20000);
    pending.set(id, { resolve, reject, timer }); socket.send(JSON.stringify({ id, method, params }));
  });
  client = {
    send, url: target.url,
    async evaluate(expression) {
      const result = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
      return result.result.value;
    },
    close() { socket.close(); },
  };
  await send('Runtime.enable'); await send('Log.enable'); await send('Page.enable');
}
async function connected() {
  try {
    await until(async () => {
      try { return await client.evaluate("document.querySelector('.backend-status')?.innerText === 'Desktop connected'"); }
      catch { return false; }
    }, 'React did not connect to Rust');
  } catch (error) {
    await screenshot('failed-connection');
    await writeFile(path.join(evidence, 'failed-connection.json'), JSON.stringify(await client.evaluate(
      "({url:location.href,body:document.body?.innerText,ready:document.readyState,tauri:typeof window.__TAURI_INTERNALS__})"), null, 2));
    throw error;
  }
}
const invoke = (command, args = {}) => client.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`);
const click = (selector) => client.evaluate(`document.querySelector(${JSON.stringify(selector)}).click()`);
async function screen(page, heading) {
  await click(`a[href="#${page}"]`);
  await until(() => client.evaluate(`document.querySelector('h1')?.textContent === ${JSON.stringify(heading)}`), `Navigation failed: ${page}`);
}
async function screenshot(name) {
  const result = await client.send('Page.captureScreenshot', { format: 'png' });
  await writeFile(path.join(evidence, name + '.png'), Buffer.from(result.data, 'base64'));
}
async function verify(label, extended = false) {
  await connect();
  const isDev = client.url.startsWith('http://127.0.0.1:1420/');
  // A CDP target exists before the first navigation/IPC bootstrap is finished.
  await connected();
  // Verify the actual startup page. Reloading here destroys active subscription/
  // invoke callbacks and manufactures Tauri orphan-callback/fallback warnings.
  // Restart/recovery is exercised by closing and launching the native process.
  const before = await invoke('get_snapshot');
  const guiState=await invoke('get_desktop_state');assert.equal(guiState.maxConcurrentJobs,1);assert.equal(typeof guiState.customized,'number');
  await writeFile(path.join(evidence,label+'-desktop-state.json'),JSON.stringify(guiState,null,2));
  originalSettings ??= before.settings;
  assert.deepEqual(before.settings, originalSettings);
  assert.equal(path.normalize(before.dataDirectory).toLowerCase(), path.join(profile.root,'data').toLowerCase());
  assert.equal(before.databaseVersion, 6);
  assert.equal(before.testProfile.runId,profile.id);assert.equal(before.testProfile.discoverySource,'windows_observed');assert.equal(before.testProfile.simulatedAdapters,false);
  assert.equal(await client.evaluate("document.querySelector('.native-test-notice')?.dataset.testRunId"),profile.id);
  if(label==='dev-reopen'){assert.notEqual(cleanupProcess(profile,env).status,0);record('active test ownership prevents cleanup');}
  record(label+': isolated database/vault/logs and WebView profile; explicit test banner',{runId:profile.id,dataDirectory:before.dataDirectory});
  const state = processes();
  assert.equal(state.apps.length, 1); assert.notEqual(state.apps[0].window, 0, 'Native window missing');
  assert.equal(state.vite.length, isDev ? 1 : 0);
  record(label + ': native window, React and real SQLite IPC', { url: client.url, pid: state.apps[0].id, dataDirectory: before.dataDirectory, theme: before.settings.theme });
  const navigation=[['dashboard','Overview'],['devices','Connected Devices'],['firmware','Library'],['backups','Backup Manager'],['jobs','Logs'],['settings','Settings']];
  assert.equal(await client.evaluate("document.querySelectorAll('nav[aria-label=\"Main navigation\"] a').length"),6);
  for(const [page,heading] of navigation) {await screen(page,heading);await screenshot(label+'-'+page);}
  record(label+': exactly six primary navigation destinations');

  if(process.argv.includes('--stage3-upgrade')) {
    await screen('firmware','Library');
    await until(()=>client.evaluate("document.querySelector('.artifact-card h3')?.textContent === 'harmless-migration-fixture.bin'"),'Migrated fixture did not render in Library');
    assert.equal(await client.evaluate("document.querySelectorAll('.artifact-card').length"),1);
    await screenshot(label+'-loaded-library');
    await click('.artifact-card');
    await until(()=>client.evaluate("document.querySelector('.firmware-actions button')?.disabled === false"),'Migrated fixture details IPC did not finish');
    await click('.firmware-actions button');
    await until(()=>client.evaluate("document.querySelector('dialog[aria-label=\"Customize firmware\"]')?.open === true"),'Customization modal did not open');
    assert.equal(await client.evaluate("document.querySelectorAll('.customization-dialog input[role=\"switch\"]:disabled').length"),4);
    assert.equal(await client.evaluate("document.querySelector('.customization-dialog .dialog-actions .primary-button')?.disabled"),true);
    await screenshot(label+'-disabled-customization');
    await click('[aria-label="Close customization"]');
    record(label+': migrated harmless file rendered; details IPC and customization modal work; all four features and rebuild remain disabled');
  }

  await screen('devices', 'Connected Devices');
  let discovery;
  await until(async () => {
    discovery = await invoke('get_discovery');
    assert.notEqual(discovery.status, 'failed', JSON.stringify(discovery.warnings));
    return ['ready', 'degraded'].includes(discovery.status);
  }, 'Initial discovery did not finish');
  assert.equal(new Set(discovery.devices.map((item) => item.id)).size, discovery.devices.length);
  for (const item of discovery.devices) {
    assert.equal(item.capabilities.flash, 'UNSUPPORTED');
    for (const feature of Object.values(item.capabilities.features)) assert.equal(feature.state, 'UNKNOWN');
  }
  for (let count = 0; count < 3; count += 1) {
    await until(() => client.evaluate("!document.querySelector('.device-toolbar button').disabled"), 'Refresh button did not enable');
    const scannedAt = (await invoke('get_discovery')).scannedAt;
    await click('.device-toolbar button');
    await until(async () => (await invoke('get_discovery')).scannedAt > scannedAt, 'Native refresh did not complete a new scan');
    const next = await invoke('get_discovery');
    assert.equal(new Set(next.devices.map((item) => item.id)).size, next.devices.length);
    assert.notEqual(next.status, 'failed');
  }
  record(label + ': discovery, three refreshes, unique IDs, unknown feature support', { records: discovery.devices.length, watching: discovery.watching, warnings: discovery.warnings });
  if(process.argv.includes('--device-gui'))await verifyDeviceGUI(client,{label,record,screenshot,pid:state.apps[0].id,profile,evidence});
  if(process.argv.includes('--phase5'))await verifyAdapters(client,{label,evidence,record});
  if(process.argv.includes('--phase6'))await verifySimulator(client,{label,evidence,record});
  if(process.argv.includes('--phase7'))await verifyOpenWrt(client,{label,evidence,record,pid:state.apps[0].id});
  if (process.argv.includes('--phase2')) {
    const index = process.argv.indexOf('--router');
    await verifyDiscovery(client, {label, evidence, record, routerAddress: index >= 0 ? process.argv[index + 1] : undefined, hotplug: process.argv.includes('--hotplug')});
  }
  if (process.argv.includes('--phase3')) await verifyProfiles(client, {label,evidence,record});
  if (process.argv.includes('--phase4')) await verifyFirmware(client, {label,evidence,record,pid:state.apps[0].id});
  if(process.argv.includes('--phase4-1')&&label==='dev-reopen')await verifyFirmwareHardening(client,{pid:state.apps[0].id,evidence,record});
  // Exercise real save IPC with the existing value, without changing user preferences.
  assert.deepEqual(await invoke('save_settings', { settings: originalSettings }), originalSettings);
  assert.deepEqual((await invoke('get_snapshot')).settings, originalSettings);
  record(label + ': same-value settings write/read preserved original preferences');

  if (extended) {
    // Prove component HMR before deliberately invalidating the shared backend module.
    await screen('settings', 'Settings');
    originalSource = await readFile(preferencePath, 'utf8');
    const anchor = '<h2>Appearance</h2>';
    assert(originalSource.includes(anchor), 'HMR proof anchor missing');
    changedSource = originalSource.replace(anchor, anchor + ' Hot reload verified.');
    assert.notEqual(changedSource, originalSource, 'HMR proof anchor missing');
    let restoreConflict = false;
    try {
      await writeFile(preferencePath, changedSource);
      await until(() => client.evaluate("document.body.innerText.includes('Hot reload verified.')"), 'React HMR did not reach native view');
      assert.deepEqual(processes(), state, 'HMR changed app/Vite process identities');
      assert.deepEqual((await invoke('get_snapshot')).settings, originalSettings);
      record(label + ': React hot reload with unchanged app/Vite PIDs');
      await screenshot('dev-hmr');
    } finally {
      if (await readFile(preferencePath, 'utf8') === changedSource) await writeFile(preferencePath, originalSource);
      else restoreConflict = true;
      changedSource = undefined;
    }
    assert(!restoreConflict, 'Concurrent Preferences edit; refused to overwrite it');
    await until(() => client.evaluate("!document.body.innerText.includes('Hot reload verified.')"), 'HMR restoration did not render');
    record(label + ': temporary HMR edit restored');
    // Tauri's invoke property is immutable. Temporarily instrument the dev module;
    // fault flags affect only this test window and the original source is restored.
    originalBackend = await readFile(backendPath, 'utf8');
    changedBackend = originalBackend.replace('export async function getSnapshot() {',
      'export async function getSnapshot() {\n  if (Reflect.get(globalThis, "__phase1FailSnapshot")) throw new Error("Injected Phase 1 read failure");\n  if (Reflect.get(globalThis, "__phase1DelaySnapshot")) await new Promise((resolve) => setTimeout(resolve, 1200));');
    assert.notEqual(changedBackend, originalBackend);
    await writeFile(backendPath, changedBackend);
    await until(() => client.evaluate('import("/src/lib/backend.ts?t=" + Date.now()).then(module => module.getSnapshot.toString().includes("__phase1FailSnapshot"))'), 'Test instrumentation did not load');
    await connected();
    await client.evaluate('window.__phase1FailSnapshot = true');
    await click('.refresh-button');
    await until(() => client.evaluate("document.querySelector('.backend-status')?.innerText === 'Backend unavailable' && document.body.innerText.includes('Injected Phase 1 read failure')"), 'Failed refresh remained falsely connected');
    await screenshot('dev-error');
    await client.evaluate('window.__phase1FailSnapshot = false');
    await click('.connection-notice button'); await connected();
    record(label + ': injected read error and real IPC retry recovery');

    await client.evaluate('window.__phase1DelaySnapshot = true');
    await click('.refresh-button');
    assert(await client.evaluate("document.querySelector('.refresh-button').disabled && document.querySelector('.backend-status').innerText === 'Refreshing workspace'"));
    await screenshot('dev-loading'); await connected();
    await client.evaluate('delete window.__phase1DelaySnapshot; delete window.__phase1FailSnapshot');
    assert.equal(await readFile(backendPath, 'utf8'), changedBackend, 'Concurrent backend edit; refused to overwrite it');
    await writeFile(backendPath, originalBackend); changedBackend = undefined;
    await connected();
    record(label + ': loading state retains cached data');
    // Wait for restored exports without navigating away from in-flight Tauri callbacks.
    // A forced Page.reload here races HMR-triggered feature reads and emits IPC fallback/
    // orphan callback warnings even though the native operation succeeds.
    await until(() => client.evaluate('import("/src/lib/backend.ts?t=" + Date.now()).then(module => !module.getSnapshot.toString().includes("__phase1FailSnapshot"))'), 'Restored backend module did not load');
    await connected();

  }
  if(process.argv.includes('--gui')&&['dev-reopen','production'].includes(label))await verifyGUI(client,{label,pid:state.apps[0].id,profile,evidence,record});
  await screen('dashboard', 'Overview'); await connected(); await screenshot(label);
  await writeFile(path.join(evidence, label + '-summary.json'), JSON.stringify({
    url: client.url, dataDirectory: before.dataDirectory, databaseVersion: before.databaseVersion,
    settings: before.settings, process: state, records: discovery.devices.length,
  }, null, 2));
  if(process.argv.includes('--phase4-1')&&label==='dev-reopen')await startShutdownVerification(client,{record,evidence});
}
async function closeWindow() {
  client?.close(); client = undefined;
  const state = processes(); assert.equal(state.apps.length, 1);
  assert.equal(powershell(`$app=Get-Process -Id ${state.apps[0].id}; $app.CloseMainWindow()`).toLowerCase(), 'true');
  try { await clean(); }
  catch(error) {
    await writeFile(path.join(evidence,'shutdown-timeout.json'),JSON.stringify({processes:processes(),targets:(await targets()).map(({type,url})=>({type,url}))},null,2));
    throw error;
  }
}
async function verifyUnavailableLibrary() {
  // Only the marked test profile's generated-byte Library is moved, after shutdown.
  const directory = path.join(profile.root, 'fixtures/gui-library-parent/OBTAINER-Library');
  const held = directory + '-offline-fixture';
  await assertOwned(profile, directory); await assertOwned(profile, held);
  const relative = path.relative(await realpath(profile.root), await realpath(directory));
  assert(relative && !relative.startsWith('..') && !path.isAbsolute(relative));
  assert(!(await lstat(directory)).isSymbolicLink());
  const before = JSON.parse(await readFile(path.join(evidence, 'gui-reopened-storage.json'), 'utf8'));
  await rename(directory, held);
  try {
    start('release'); await connect(); await connected();
    const state = await invoke('get_desktop_state');
    assert.equal(state.libraryAvailable, false);
    assert.equal(path.normalize(state.libraryDirectory), path.normalize(directory));
    assert.deepEqual(state.preferences, before.preferences);
    await until(() => client.evaluate("[...document.querySelectorAll('[role=alert]')].some(e=>e.textContent.includes('Library storage is unavailable'))"), 'Unavailable Library alert did not render');
    const placement = await client.evaluate("(() => { const a=document.querySelector('.toast-stack [role=alert]').getBoundingClientRect(); const sidebar=document.querySelector('.sidebar').getBoundingClientRect(); return {left:a.left,right:a.right,top:a.top,bottom:a.bottom,sidebarRight:sidebar.right,width:innerWidth,height:innerHeight}; })()");
    assert(placement.left >= placement.sidebarRight && placement.right <= placement.width && placement.top >= 0 && placement.bottom <= placement.height, 'Unavailable Library alert must stay visible outside the sidebar');
    const migration = JSON.parse(await readFile(path.join(evidence, 'gui-storage-migration.json'), 'utf8'));
    const page = await invoke('list_firmware_artifacts', {query:{}});
    assert.deepEqual(page.artifacts.map(a=>({sha256:a.sha256,sizeBytes:a.sizeBytes,kind:a.kind,sourceCount:a.sourceCount})), migration.artifacts);
    assert(page.artifacts.every(a=>a.storageStatus==='present'), 'Absent volume must not mark individual blobs missing');
    // Catch inside the WebView: the CDP helper wraps uncaught rejections in Error
    // and intentionally does not preserve the IPC error object's code property.
    const denial = await client.evaluate(`window.__TAURI_INTERNALS__.invoke('verify_firmware_integrity', {sha256:${JSON.stringify(page.artifacts[0].sha256)}}).then(()=>null,error=>error.code)`);
    assert.equal(denial, 'LIBRARY_UNAVAILABLE', 'Unavailable storage must reject new file jobs');
    await screenshot('production-library-unavailable');
    await closeWindow();
    await assert.rejects(lstat(directory), {code:'ENOENT'});
    record('production restart: unavailable relocated Library keeps native window, React/IPC, records and settings; file jobs rejected and missing root not recreated');
  } finally {
    if (processes().apps.length) await closeWindow(); else await clean();
    await assertOwned(profile, held); await rename(held, directory);
  }
  start('release'); await connect(); await connected();
  assert.equal((await invoke('get_desktop_state')).libraryAvailable, true);
  await verifyGUIRestart(client,{profile,evidence,record}); await closeWindow();
  record('production restart: restored test Library reacquires leases and verifies bytes without switching storage directories');
  // Exercise the explicit recovery path without moving or deleting normal data.
  await rename(directory, held);
  try {
    start('release'); await connect(); await connected();
    await screen('settings', 'Settings');
    const clickText = async text => client.evaluate(`(()=>{const b=[...document.querySelectorAll('button')].find(b=>b.textContent.trim()===${JSON.stringify(text)}&&!b.disabled);if(!b)throw Error('Button missing: '+${JSON.stringify(text)});b.click();})()`);
    await until(()=>client.evaluate("Boolean([...document.querySelectorAll('button')].find(b=>b.textContent==='Use default Library location'&&!b.disabled))"),'Storage recovery action unavailable');
    await clickText('Use default Library location');
    await until(()=>client.evaluate("Boolean(document.querySelector('.confirm-dialog[open]'))"),'Recovery confirmation missing');
    await clickText('Cancel');
    assert.notEqual((await invoke('get_desktop_state')).preferences.libraryDirectory,null,'Cancelled recovery changed location');
    await clickText('Use default Library location'); await clickText('Use default location');
    await until(async()=>{const s=await invoke('get_desktop_state');return s.restartRequired&&s.preferences.libraryDirectory===null;},'Default recovery was not persisted');
    await screenshot('production-library-recovery-selected'); await closeWindow();
    start('release'); await connect(); await connected();
    const recovered=await invoke('get_desktop_state');
    assert.equal(recovered.libraryAvailable,true);assert.equal(recovered.preferences.libraryDirectory,null);
    assert.equal(path.normalize(recovered.libraryDirectory),path.normalize(path.join(profile.root,'data/vault')));
    const page=await invoke('list_firmware_artifacts',{query:{}});assert(page.artifacts.length>0);
    const verify=await invoke('verify_firmware_integrity',{sha256:page.artifacts[0].sha256});
    await until(async()=>{const j=await invoke('get_library_job',{id:verify.id});return j.status==='completed';},'Recovered default bytes did not verify');
    await screenshot('production-library-default-recovered');await closeWindow();
    record('production recovery: actual Settings confirmation/cancellation, default selection, restart, retained metadata and verified default bytes; old storage preserved');
  } finally {
    if(processes().apps.length)await closeWindow();
    await assertOwned(profile,held);await rename(held,directory);
  }
}
async function chooseAppearance(theme) {
  await screen('settings', 'Settings');
  await click(`input[name="theme"][value="${theme}"]`);
  await until(() => client.evaluate("!document.querySelector('.preference-actions button').disabled"), 'Appearance save did not enable');
  await click('.preference-actions button');
  await until(() => client.evaluate("document.body.innerText.includes('Preferences saved to SQLite.')"), 'Native appearance save failed');
  assert.equal((await invoke('get_snapshot')).settings.theme, theme);
}
let succeeded=false;let baselineCaptured=false;
try {
  probeNormal(root,evidence,'before');baselineCaptured=true;record('normal DB/WAL/SHM/log/vault byte snapshot captured without SQLite');
  // Stage 3 qualification launches only the dedicated native-test executable.
  // It must never invoke the production binary against the owner's data directory.
  if(!process.argv.includes('--stage3-upgrade')&&!process.argv.includes('--official-acquisition')&&!process.argv.includes('--universal-workflow')&&!process.argv.includes('--manufacturer-package')) {
    const startupProbe=execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.join(root,'scripts/verify-startup.ps1'),'-Binary',path.join(root,'target/release/obtainer.exe')],{env,windowsHide:true,encoding:'utf8',timeout:30000});
    await writeFile(path.join(evidence,'production-startup-error.json'),startupProbe);
    assert.equal(JSON.parse(startupProbe).exitCode,1);
    record('ordinary release: actual visible startup failure dialog for duplicate workspace; closes cleanly before opening normal data');
    verifyActivationGuards(root,profile,env,record);
  }
  if(process.argv.includes('--manufacturer-package')) {
    await clean();start('release');await verify('manufacturer-before');
    await verifyManufacturerPackage(client,{root,profile,evidence,pid:processes().apps[0].id,record,screenshot});
    await closeWindow();start('release');await verify('manufacturer-reopened');
    await verifyManufacturerPackage(client,{root,profile,evidence,pid:processes().apps[0].id,record,screenshot,reopened:true});
    await closeWindow();
  } else if(process.argv.includes('--universal-workflow')) {
    await clean(); start('release'); await connect(); await connected();
    await verifyUniversalWorkflow(client,{root,profile,evidence,record,screenshot});
    await closeWindow(); start('release'); await connect(); await connected();
    await verifyUniversalWorkflow(client,{root,profile,evidence,record,screenshot,reopened:true});
    await closeWindow();
  } else if(process.argv.includes('--mg11-challenge')) {
    await clean();start('release');await verify('production');
    await verifyAuthChallenge(client,{label:'production',pid:processes().apps[0].id,evidence});
    await closeWindow();
  } else if(process.argv.includes('--official-acquisition')) {
    await clean();start('release');await verify('official-before');
    const hash=await verifyOfficialAcquisition(client,{root,profile,evidence,record,screenshot});
    if(process.argv.includes('--firmware-extraction'))await verifyExtraction(client,{root,profile,evidence,record,screenshot,pid:processes().apps[0].id,hash});
    await closeWindow();
    start('release');await verify('official-reopened');
    const report=await invoke('get_firmware_analysis',{sha256:hash});assert.equal(report.inputSha256,hash);
    const provenance=await invoke('get_official_provenance',{sha256:hash});assert.equal(provenance[0].sha256,hash);
    record('official image, exact source provenance and bounded report remain accessible after native restart');
    if(process.argv.includes('--firmware-extraction'))await verifyExtraction(client,{root,profile,evidence,record,screenshot,pid:processes().apps[0].id,hash,reopened:true});
      if(process.argv.includes('--extraction-review'))await verifyExtractionReview(client,{root,profile,evidence,record,screenshot,pid:processes().apps[0].id});
    await closeWindow();
  } else if (process.argv.includes('--stage3-upgrade')) {
    const fixture=operation=>JSON.parse(execFileSync('py',[path.join(root,'scripts/stage3-native-fixture.py'),operation,profile.id],{env,windowsHide:true,encoding:'utf8'}));
    const before=fixture('create');
    await writeFile(path.join(evidence,'stage3-before-no.json'),JSON.stringify(before,null,2));
    const answer=(launch,value)=>JSON.parse(execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.join(root,'scripts/answer-isolated-library-upgrade.ps1'),'-LauncherProcessId',String(launch.process.pid),'-RunId',profile.id,'-Answer',value],{env,windowsHide:true,encoding:'utf8',timeout:45000}));
    await clean();
    const declined=start('release');
    const no=answer(declined,'No');
    await writeFile(path.join(evidence,'stage3-native-no.json'),JSON.stringify(no,null,2));
    await clean();
    const afterNo=fixture('snapshot');
    assert.deepEqual(afterNo,before,'Native No changed fixture data or directories');
    await writeFile(path.join(evidence,'stage3-after-no.json'),JSON.stringify(afterNo,null,2));
    record('isolated native migration dialog: default No, explicit No, complete data/directory byte preservation');
    const approved=start('release');
    const yes=answer(approved,'Yes');
    await writeFile(path.join(evidence,'stage3-native-yes.json'),JSON.stringify(yes,null,2));
    await verify('stage3-upgraded'); await closeWindow();
    const inspected=fixture('inspect');
    assert.equal(inspected.version,6);assert.equal(inspected.theme,'dark');
    assert.equal(inspected.artifact[2],'legacy_unclassified');
    assert.deepEqual(inspected.artifact.slice(0,2),inspected.backupArtifact);
    assert.equal(inspected.artifact[0],inspected.blobSha256);
    assert.equal(inspected.backupVersion,5);assert.deepEqual(inspected.backupIntegrity,[['ok']]);assert.deepEqual(inspected.backupForeignKeys,[]);
    await writeFile(path.join(evidence,'stage3-upgrade-inspection.json'),JSON.stringify(inspected,null,2));
    record('isolated native Yes: schema 6, preserved harmless blob/settings/legacy class, verified schema-5 backup');
    start('release'); await verify('stage3-reopened'); await closeWindow();
  } else if (process.argv.includes('--release-only')) {
    await clean(); start('release'); await verify('production'); await closeWindow();
  } else if (process.argv.includes('--watch-usb')) {
    await clean(); start('dev'); await connect(); await connected();
    await screen('devices', 'Connected Devices');
    await watchUsb(client, {evidence, record});
    await closeWindow();
  } else if (process.argv.includes('--gui-preview')) {
    await clean();start('dev');await verify('dev-reopen',true);await closeWindow();
  } else if (settingsRoundtrip) {
    await clean();
    start('release'); await connect(); await connected();
    const baseline = await invoke('get_snapshot'); originalSettings = baseline.settings;
    const changedTheme = originalSettings.theme === 'dark' ? 'light' : 'dark';
    await chooseAppearance(changedTheme);
    record('production UI saved a changed appearance through real Rust/SQLite IPC');
    await closeWindow();
    start('dev'); await connect(); await connected();
    const dev = await invoke('get_snapshot');
    assert.equal(dev.dataDirectory, baseline.dataDirectory);
    assert.equal(dev.settings.theme, changedTheme);
    await screen('settings', 'Settings');
    assert.equal(await client.evaluate('document.querySelector("input[name=theme]:checked").value'), changedTheme);
    record('development reopened the changed production setting from the same database');
    await chooseAppearance(originalSettings.theme);
    record('development UI restored the original appearance');
    await closeWindow();
    start('release'); await connect(); await connected();
    const restored = await invoke('get_snapshot');
    assert.equal(restored.dataDirectory, baseline.dataDirectory);
    assert.deepEqual(restored.settings, originalSettings);
    record('production restart verified the original settings restored by development');
    await closeWindow();
  } else if (attach) {
    await verify('dev-initial', true); await closeWindow();
  } else {
    await clean();
    start('dev'); await verify('dev-reopen', true); await closeWindow();
    const interrupted = start('dev');
    await until(serverUp, 'Interrupted launch never started Vite', 120000);
    if(process.argv.includes('--phase4')) {await connect();await connected();await interruptFirmwareImport(client,{pid:processes().apps[0].id,evidence,record});client.close();client=undefined;}
    const backendRenderedBeforeInterrupt = (await targets()).some((item) => item.type === 'page');
    spawnSync('taskkill', ['/PID', String(interrupted.process.pid), '/T', '/F'], { windowsHide: true });
    await clean();
    record('interrupted owned development launch and released resources', { backendRenderedBeforeInterrupt });
    start('dev'); await verify('dev-recovery'); await closeWindow();
    start('release'); await verify('production'); await closeWindow();
    if(process.argv.includes('--gui')){start('release');await connect();await connected();await verifyGUIRestart(client,{profile,evidence,record});await screenshot('production-relocated');await closeWindow();await verifyUnavailableLibrary();}
  }
  const issues = consoleEvents.filter((event) => event.method === 'Runtime.exceptionThrown' ||
    ['warning', 'error'].includes(event.params?.entry?.level) ||
    ['warning', 'error', 'assert'].includes(event.params?.type));
  await writeFile(path.join(evidence, 'console-issues.json'), JSON.stringify(issues, null, 2));
  assert.equal(issues.length, 0, 'Review native console-issues.json');
  record('no captured native console warnings or unhandled exceptions');
  await verifyCleanupGuards(profile,env,evidence,record);
  succeeded=true;
} catch (error) {
  await writeFile(path.join(evidence, 'failure.json'), JSON.stringify({message:error.message,stack:error.stack},null,2));
  if(client) {
    try {
      await writeFile(path.join(evidence,'failure-ui.txt'),await client.evaluate('document.body.innerText'));
      const capture=await client.send('Page.captureScreenshot',{format:'png'});
      await writeFile(path.join(evidence,'failure.png'),Buffer.from(capture.data,'base64'));
    } catch { /* Preserve the original failure if the window has already exited. */ }
  }
  throw error;
} finally {
  // Preserve the original preference if a roundtrip assertion fails before normal restoration.
  if (settingsRoundtrip && originalSettings && client) {
    try { await invoke('save_settings', { settings: originalSettings }); }
    catch (error) { console.error('Original preference restoration failed:', error.message); }
  }
  client?.close();
  if (changedBackend && await readFile(backendPath, 'utf8') === changedBackend) await writeFile(backendPath, originalBackend);
  if (changedSource && await readFile(preferencePath, 'utf8') === changedSource) await writeFile(preferencePath, originalSource);
  for (const launch of launches) {
    if (launch.process.exitCode === null) spawnSync('taskkill', ['/PID', String(launch.process.pid), '/T', '/F'], { windowsHide: true });
  }
  for (const [index, launch] of launches.entries()) {
    await writeFile(path.join(evidence, `launch-${index}-${launch.kind}.txt`), launch.output);
    commands[index].exitCode = launch.exitCode;
  }
  if(baselineCaptured){probeNormal(root,evidence,'after');record('normal application file bytes unchanged after isolated native suite');}
  if(succeeded){const cleanup=cleanupProcess(profile,env);await writeFile(path.join(evidence,'cleanup.json'),JSON.stringify({status:cleanup.status,stdout:cleanup.stdout,stderr:cleanup.stderr},null,2));assert.equal(cleanup.status,0,'Test cleanup retained files; inspect cleanup.json');record('owned isolated run cleaned after all processes stopped',{runId:profile.id});}
  else console.log('Failed test run retained: '+profile.root);
  await writeFile(path.join(evidence, 'console.json'), JSON.stringify(consoleEvents, null, 2));
  await writeFile(path.join(evidence, 'results.json'), JSON.stringify({ results, commands }, null, 2));
}
