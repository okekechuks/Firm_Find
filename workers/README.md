# Future Linux workers

No worker executable exists in the foundation. Future static analysis and OpenWrt building should use separate versioned JSON requests such as `{ "protocolVersion": 1, "jobId": "...", "operation": "inspect", "artifactSha256": "..." }`. Define strict operation enums and backend-generated workspaces before implementation.

Workers require cancellation, timeouts, bounded output, tool version reporting and checks of staged artifact hashes. The React frontend must never provide arbitrary shell scripts. WSL2 is optional and cannot by itself provide adequate isolation for dynamic execution of malicious firmware.
