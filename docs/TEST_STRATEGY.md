# Foundation verification

Frontend integration tests exercise native state rendering, every planned screen, confirmed settings saves, backend errors/retry and browser preview boundaries. IPC tests reject malformed responses, prevent invalid outgoing settings and ensure browser preview does not invoke native commands.

Rust tests exercise atomic migration, settings and audit persistence across reopening, rejection of newer database versions without changing their version, settings validation and forbidden risk classes. Future device and firmware tests require real provider and ingestion code, not tests of placeholders.

Windows CI performs frontend checks, Rust format/clippy/tests and both unsigned installer builds. Manual desktop acceptance must launch the app as a standard Windows user, save appearance, restart and verify persistence. Installer creation and manual launch are separate checks. Hardware validation belongs to later phases.

`scripts/native-smoke.mjs` also verifies the actual Windows app through Tauri's native WebDriver: real workspace state, navigation, Rust input rejection, saved appearance, restart persistence, and restoration of the prior setting. It captures an actual WebView screenshot for visual review. The native test requires matching Edge and Tauri drivers; component tests alone do not establish this runtime evidence.

For a data-preserving audit, run `npm run test:native -- target/debug/obtainer.exe --audit` in a normal Windows user session. This mode does not call `save_settings`; it reads the existing preference through real IPC, restarts the app, and checks that it persists. Normal startup and passive-inventory audit records still occur. Screenshots and driver logs go to unique ignored `.tools/native-audit/` directories, preserving prior evidence. Restricted execution may fail before WebView2 creation; report that context separately from normal-session results. Tests of changing preferences remain in temporary SQLite databases and frontend fixtures unless a separate full native smoke run is authorized.
