# Foundation verification

Frontend integration tests exercise native state rendering, every planned screen, confirmed settings saves, backend errors/retry and browser preview boundaries. IPC tests reject malformed responses, prevent invalid outgoing settings and ensure browser preview does not invoke native commands.

Rust tests exercise atomic migration, settings and audit persistence across reopening, rejection of newer database versions without changing their version, settings validation and forbidden risk classes. Future device and firmware tests require real provider and ingestion code, not tests of placeholders.

Windows CI performs frontend checks, Rust format/clippy/tests and both unsigned installer builds. Manual desktop acceptance must launch the app as a standard Windows user, save appearance, restart and verify persistence. Installer creation and manual launch are separate checks. Hardware validation belongs to later phases.
