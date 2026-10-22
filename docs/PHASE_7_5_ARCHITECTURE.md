# Phase 7.5 acquisition architecture — Sprint A review handoff

22 October 2026. Owner-authorized workstream, between 7 and 8. This is the initial design for independent review; high-risk acquisition/analysis operations are not accepted by this document.

Existing Windows discovery remains the observation source. A new Rust `acquisition` module builds a typed fingerprint from current inventory and retained evidence, recognizes candidate management families from a bounded root-page observation, and independently evaluates five acquisition strategies. Profile matching remains feature evidence, not a firmware extractor. Existing adapter/session authorization remains intact.

`discover_firmware_routes` accepts a current device reference and optionally an explicitly approved local endpoint/interface. No approval means no network traffic. Approval permits one unauthenticated root GET only: no cookies, credentials, redirects, endpoint crawling, TLS bypass, public destinations, self-addresses or arbitrary paths. Route/interface selection reuses connection policy. Deadline, response bounds, cancellation and post-operation discovery checks reject stale results. Generic host NICs identify the transport to a selected gateway, never the router itself.

Evidence distinguishes Windows observations, candidate protocol fingerprints, owner reports and authenticated identity. A page title or USB identifier cannot verify exact model, chipset, firmware or export support. HTTP reachability does not establish a management API. Each acquisition strategy returns its own state, requirements, evidence and executable flag. An empty qualified-source catalog yields unknown, not hardware unsupported. Sprint A has no executable acquisition route and creates no firmware artifacts.

React More Info invokes this backend discovery, shows identity/protocol candidates and independent route results, keeps unqualified devices amber, and offers backend-redacted diagnostics. Obtain Firmware stays disabled until a later reviewed worker provides backend authorization and durable verified bytes in the existing Library.

Sprint B must extend existing Library jobs/vault/provenance rather than create another store. Exact publisher/model/revision/region/version/image matching, source trust, bounded downloads, integrity/authenticity and restart handling need review before enabling acquisition. Sprint C needs a separately reviewed isolated Linux analysis worker; WSL alone is not the isolation boundary. Build/install/recovery remain independent and disabled.

Review requested from Claude: endpoint/interface binding, self/host exclusions, cancellation ownership, transport bounds, evidence authority, strategy independence, diagnostics redaction, UI state and regressions. Supply findings through the owner; no review message is sent automatically.
