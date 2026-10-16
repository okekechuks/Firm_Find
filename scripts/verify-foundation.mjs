// Exercises the actual Windows Tauri dev command and release executable through native WebView2.
import assert from 'node:assert/strict';
import { spawn, spawnSync, execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const attach = process.argv.includes('--attach-dev');
const settingsRoundtrip = process.argv.includes('--settings-roundtrip');
const evidence = path.join(root, '.tools/foundation', new Date().toISOString().replaceAll(':', '-'));
await mkdir(evidence, { recursive: true });
const results = [], consoleEvents = [], commands = [], launches = [];
const env = { ...process.env, OBTAINER_TEST_ROOT: root, CARGO_INCREMENTAL: '0',
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
  return JSON.parse(powershell(`$ErrorActionPreference='Stop'; $apps=@(Get-Process -Name obtainer -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path.StartsWith((Join-Path $env:OBTAINER_TEST_ROOT 'target\\'),[StringComparison]::OrdinalIgnoreCase) } | ForEach-Object { [pscustomobject]@{id=$_.Id; window=$_.MainWindowHandle.ToInt64(); path=$_.Path} }); $vite=@(Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'node.exe' -and $_.CommandLine -like ('*'+$env:OBTAINER_TEST_ROOT+'*') -and $_.CommandLine -match '[\\\\/]vite[\\\\/]bin[\\\\/]vite\\.js' } | ForEach-Object { $_.ProcessId }); [pscustomobject]@{apps=$apps;vite=$vite} | ConvertTo-Json -Depth 4`));
}
async function clean() {
  await until(async () => !(await serverUp()) && (await targets()).length === 0, 'Native session or Vite listener did not stop');
  const state = processes();
  assert.equal(state.apps.length, 0, 'OBTAINER process remains');
  assert.equal(state.vite.length, 0, 'Vite process remains');
  record('shutdown left no app, Vite process or development listener');
}
function start(kind) {
  const command = kind === 'dev' ? 'npm.cmd run tauri -- dev -- --locked --offline' : '.\\target\\release\\obtainer.exe';
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
  await client.send('Page.reload'); await connected();
  const before = await invoke('get_snapshot');
  originalSettings ??= before.settings;
  assert.deepEqual(before.settings, originalSettings);
  assert.equal(path.normalize(before.dataDirectory).toLowerCase(), path.join(env.LOCALAPPDATA, 'OBTAINER').toLowerCase());
  assert.equal(before.databaseVersion, 1);
  const state = processes();
  assert.equal(state.apps.length, 1); assert.notEqual(state.apps[0].window, 0, 'Native window missing');
  assert.equal(state.vite.length, isDev ? 1 : 0);
  record(label + ': native window, React and real SQLite IPC', { url: client.url, pid: state.apps[0].id, dataDirectory: before.dataDirectory, theme: before.settings.theme });
  for (const [page, heading] of [
    ['dashboard', 'Overview'], ['devices', 'Connected devices'], ['firmware', 'Firmware library'],
    ['analysis', 'Analysis'], ['backups', 'Backup manager'], ['builder', 'Firmware builder'],
    ['jobs', 'Jobs & activity'], ['settings', 'Settings'],
  ]) await screen(page, heading);
  record(label + ': all eight navigation destinations');

  await screen('devices', 'Connected devices');
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
    const revision = (await invoke('get_discovery')).revision;
    await click('.device-toolbar button');
    await until(async () => (await invoke('get_discovery')).revision > revision, 'Native refresh did not advance revision');
    const next = await invoke('get_discovery');
    assert.equal(new Set(next.devices.map((item) => item.id)).size, next.devices.length);
    assert.notEqual(next.status, 'failed');
  }
  record(label + ': discovery, three refreshes, unique IDs, unknown feature support', { records: discovery.devices.length, watching: discovery.watching, warnings: discovery.warnings });
  // Exercise real save IPC with the existing value, without changing user preferences.
  assert.deepEqual(await invoke('save_settings', { settings: originalSettings }), originalSettings);
  assert.deepEqual((await invoke('get_snapshot')).settings, originalSettings);
  record(label + ': same-value settings write/read preserved original preferences');

  if (extended) {
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

    await screen('settings', 'Settings');
    originalSource = await readFile(preferencePath, 'utf8');
    const anchor = 'Choose how OBTAINER looks on your desktop.';
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
  }
  await screen('dashboard', 'Overview'); await connected(); await screenshot(label);
  await writeFile(path.join(evidence, label + '-summary.json'), JSON.stringify({
    url: client.url, dataDirectory: before.dataDirectory, databaseVersion: before.databaseVersion,
    settings: before.settings, process: state, records: discovery.devices.length,
  }, null, 2));
}
async function closeWindow() {
  client?.close(); client = undefined;
  const state = processes(); assert.equal(state.apps.length, 1);
  assert.equal(powershell(`$app=Get-Process -Id ${state.apps[0].id}; $app.CloseMainWindow()`).toLowerCase(), 'true');
  await clean();
}
async function chooseAppearance(theme) {
  await screen('settings', 'Settings');
  await click(`input[name="theme"][value="${theme}"]`);
  await until(() => client.evaluate("!document.querySelector('.preference-actions button').disabled"), 'Appearance save did not enable');
  await click('.preference-actions button');
  await until(() => client.evaluate("document.body.innerText.includes('Preferences saved to SQLite.')"), 'Native appearance save failed');
  assert.equal((await invoke('get_snapshot')).settings.theme, theme);
}
try {
  if (settingsRoundtrip) {
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
    const backendRenderedBeforeInterrupt = (await targets()).some((item) => item.type === 'page');
    spawnSync('taskkill', ['/PID', String(interrupted.process.pid), '/T', '/F'], { windowsHide: true });
    await clean();
    record('interrupted owned development launch and released resources', { backendRenderedBeforeInterrupt });
    start('dev'); await verify('dev-recovery'); await closeWindow();
    start('release'); await verify('production'); await closeWindow();
  }
  const issues = consoleEvents.filter((event) => event.method === 'Runtime.exceptionThrown' ||
    ['warning', 'error'].includes(event.params?.entry?.level) ||
    ['warning', 'error', 'assert'].includes(event.params?.type));
  await writeFile(path.join(evidence, 'console-issues.json'), JSON.stringify(issues, null, 2));
  assert.equal(issues.length, 0, 'Review native console-issues.json');
  record('no captured native console warnings or unhandled exceptions');
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
  await writeFile(path.join(evidence, 'console.json'), JSON.stringify(consoleEvents, null, 2));
  await writeFile(path.join(evidence, 'results.json'), JSON.stringify({ results, commands }, null, 2));
}
