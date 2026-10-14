# Foundation verification

Frontend integration tests exercise native state rendering, every planned screen, confirmed settings saves, backend errors/retry and browser preview boundaries. IPC tests reject malformed responses, prevent invalid outgoing settings and ensure browser preview does not invoke native commands.

Rust tests exercise atomic migration, settings and audit persistence across reopening, rejection of newer database versions without changing their version, settings validation and forbidden risk classes. Future device and firmware tests require real provider and ingestion code, not tests of placeholders.

Windows CI performs frontend checks, Rust format/clippy/tests and both unsigned installer builds. Manual desktop acceptance must launch the app as a standard Windows user, save appearance, restart and verify persistence. Installer creation and manual launch are separate checks. Hardware validation belongs to later phases.

`scripts/native-smoke.mjs` also verifies the actual Windows app through Tauri's native WebDriver: real workspace state, navigation, Rust input rejection, saved appearance, restart persistence, and restoration of the prior setting. It captures an actual WebView screenshot for visual review. The native test requires matching Edge and Tauri drivers; component tests alone do not establish this runtime evidence.
