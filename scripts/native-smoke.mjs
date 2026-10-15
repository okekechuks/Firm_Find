// Drives the native OBTAINER WebView through Tauri's Windows WebDriver.
// This does not control the Codex in-app browser or mock IPC.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const arguments_ = process.argv.slice(2);
const auditOnly = arguments_.includes('--audit');
const binary = path.resolve(root, arguments_.find((argument) => argument !== '--audit') ?? 'target/release/obtainer.exe');
const auditDirectory = path.join(root, '.tools/native-audit', new Date().toISOString().replaceAll(':', '-'));
const driver = path.join(root, '.tools/cargo/bin/tauri-driver.exe');
const nativeDriver = path.join(root, '.tools/webdriver/msedgedriver.exe');
for (const file of [binary, driver, nativeDriver]) {
  assert(existsSync(file), `Required native test dependency missing: ${file}`);
}

const server = spawn(driver, ['--port', '4445', '--native-port', '4446', '--native-driver', nativeDriver], {
  cwd: root, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'],
});
let driverOutput = '';
server.stdout.on('data', (data) => { driverOutput += data.toString(); });
server.stderr.on('data', (data) => { driverOutput += data.toString(); });
let session;
let originalTheme;
let savedTheme;
let originalSettings;
const endpoint = 'http://127.0.0.1:4445';
if (auditOnly) console.log(`Audit mode: saved preferences stay unchanged; evidence: ${auditDirectory}`);

async function request(method, url, body) {
  const response = await fetch(`${endpoint}${url}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(60000),
  });
  const result = await response.json();
  if (!response.ok) throw new Error(`WebDriver ${method} ${url}: ${JSON.stringify(result)}`);
  return result.value;
}

async function until(check, message, timeout = 30000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await check()) return;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(message);
}

async function script(source, args = []) {
  return request('POST', `/session/${session}/execute/sync`, { script: source, args });
}

async function asyncScript(source, args = []) {
  return request('POST', `/session/${session}/execute/async`, { script: source, args });
}

async function invoke(command) {
  const result = await asyncScript(
    "const done = arguments[arguments.length - 1]; window.__TAURI_INTERNALS__.invoke(arguments[0]).then(done, error => done({__error: String(error)}));",
    [command],
  );
  assert(!result?.__error, `${command} failed: ${result?.__error}`);
  return result;
}

async function click(selector) {
  const element = await request('POST', `/session/${session}/element`, { using: 'css selector', value: selector });
  const id = element['element-6066-11e4-a52e-4f735466cecf'];
  await request('POST', `/session/${session}/element/${id}/click`, {});
}

async function launch() {
  const value = await request('POST', '/session', { capabilities: { alwaysMatch: { 'tauri:options': { application: binary } } } });
  session = value.sessionId;
  await until(() => script("return document.body.innerText.includes('Desktop connected')"), 'Native app did not connect to Rust/SQLite');
  console.log('PASS: native window rendered and connected to Rust');
}

async function close() {
  if (session) {
    await request('DELETE', `/session/${session}`);
    session = undefined;
  }
}

async function preferences() {
  await click('a[href="#settings"]');
  await until(() => script("return Boolean(document.querySelector('input[name=theme]'))"), 'Preferences did not render');
}

async function applyTheme(theme) {
  await click(`input[name="theme"][value="${theme}"]`);
  const disabled = await script("return document.querySelector('.preference-actions button').disabled");
  if (!disabled) {
    await click('.preference-actions button');
    await until(() => script("return document.body.innerText.includes('Preferences saved to SQLite.')"), 'Appearance save did not complete');
  }
}

try {
  await until(async () => {
    try { return (await request('GET', '/status')).ready; } catch { return false; }
  }, 'Tauri driver failed to start');
  await launch();

  for (const [screen, heading] of [
    ['devices', 'Connected devices'], ['firmware', 'Firmware library'],
    ['analysis', 'Analysis'], ['backups', 'Backup manager'],
    ['builder', 'Firmware builder'], ['jobs', 'Jobs & activity'],
  ]) {
    await click(`a[href="#${screen}"]`);
    await until(() => script('return document.querySelector("h1")?.textContent === arguments[0]', [heading]), `Navigation failed: ${screen}`);
  }
  console.log('PASS: native navigation through planned workspaces and activity');

  await click('a[href="#devices"]');
  await until(() => script("return [...document.querySelectorAll('h2')].some(node => node.textContent === 'What Windows can see')"), 'Device Explorer did not render');
  const discovery = await invoke('get_discovery');
  assert(['ready', 'degraded'].includes(discovery.status), `Unexpected discovery status: ${discovery.status}`);
  for (const device of discovery.devices) {
    assert.equal(device.capabilities.flash, 'UNSUPPORTED');
    for (const feature of Object.values(device.capabilities.features)) assert.equal(feature.state, 'UNKNOWN');
  }
  await click('.device-toolbar button');
  await until(async () => (await invoke('get_discovery')).revision > discovery.revision, 'Device inventory refresh did not complete');
  console.log(`PASS: native Windows inventory and refresh (${discovery.devices.length} records; feature support remains unverified)`);
  if (auditOnly) {
    console.log(`AUDIT discovery: ${JSON.stringify({
      status: discovery.status,
      watching: discovery.watching,
      warnings: discovery.warnings,
      records: discovery.devices.length,
      transports: Object.fromEntries(['usb', 'serial', 'network'].map((transport) =>
        [transport, discovery.devices.filter((device) => device.transports.includes(transport)).length])),
    })}`);
  }

  await preferences();
  originalTheme = await script('return document.querySelector("input[name=theme]:checked").value');
  if (auditOnly) {
    const snapshot = await invoke('get_snapshot');
    originalSettings = snapshot.settings;
    assert.equal(originalSettings.theme, originalTheme);
    savedTheme = originalTheme;
    console.log(`AUDIT workspace: ${JSON.stringify({
      version: snapshot.version,
      databaseVersion: snapshot.databaseVersion,
      dataDirectory: snapshot.dataDirectory,
      counts: snapshot.counts,
    })}`);
    console.log('PASS: existing preferences read through real IPC without changing settings');
  } else {
    savedTheme = originalTheme === 'dark' ? 'light' : 'dark';
    await applyTheme(savedTheme);
    assert.equal(await script('return document.documentElement.dataset.theme'), savedTheme);
    console.log('PASS: appearance saved through real IPC');

    const rejected = await request('POST', `/session/${session}/execute/async`, {
      script: "const done = arguments[arguments.length - 1]; window.__TAURI_INTERNALS__.invoke('save_settings', {settings: {theme: 'invalid'}}).then(() => done(false), () => done(true));",
      args: [],
    });
    assert.equal(rejected, true, 'Rust accepted an invalid preference');
    console.log('PASS: Rust rejects malformed settings');
  }

  await click('a[href="#dashboard"]');
  await until(() => script("return document.querySelector('h1')?.textContent === 'Overview'"), 'Overview did not render');
  const image = await request('GET', `/session/${session}/screenshot`);
  const screenshotDirectory = auditOnly ? auditDirectory : path.join(root, 'docs/images');
  await mkdir(screenshotDirectory, { recursive: true });
  await writeFile(path.join(screenshotDirectory, 'desktop-smoke.png'), Buffer.from(image, 'base64'));

  await close();
  await launch();
  await preferences();
  assert.equal(await script('return document.querySelector("input[name=theme]:checked").value'), savedTheme);
  if (auditOnly) {
    assert.deepEqual((await invoke('get_snapshot')).settings, originalSettings);
  } else {
    assert.equal(await script('return document.documentElement.dataset.theme'), savedTheme);
  }
  console.log('PASS: appearance persists after closing and relaunching native app');

  if (!auditOnly) {
    await applyTheme(originalTheme);
    console.log('PASS: original appearance restored');
  }
  await close();
} finally {
  if (session) {
    try {
      if (originalTheme && !auditOnly) { await preferences(); await applyTheme(originalTheme); }
      await close();
    } catch (error) { console.error('Native test cleanup:', error.message); }
  }
  if (server.pid) spawnSync('taskkill', ['/PID', String(server.pid), '/T', '/F'], { windowsHide: true, stdio: 'ignore' });
  const logDirectory = auditOnly ? auditDirectory : path.join(root, '.tools');
  await mkdir(logDirectory, { recursive: true });
  await writeFile(path.join(logDirectory, 'native-driver.log'), driverOutput);
}
