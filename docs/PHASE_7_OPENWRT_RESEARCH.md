# Phase 7 OpenWrt research

> Research amendment, 20 October 2026: the interface decisions below are source research. Executed tests and acceptance belong in [Phase 7 verification](PHASE_7_VERIFICATION.md). Physical qualification remains pending.

Research date: 20 October 2026. Software/native qualification passed as recorded in the verification report; physical qualification remains pending.

## Selected interoperability target

OpenWrt **25.12.6, x86/64**, running in a disposable QEMU guest. This is an OS interoperability target, not a qualified physical router or modem. The owner has only a proprietary MiFi available. No physical OpenWrt qualification is claimed.

The [official target index](https://downloads.openwrt.org/releases/25.12.6/targets/x86/64/) publishes the combined ext4 image and matching ImageBuilder. The test image SHA-256 is `f8538784d0288b4133a99ac2a68443e4922ffde6343e80df56a5882364b9d499`. The [release index](https://downloads.openwrt.org/releases/) and [25.12 release page](https://openwrt.org/releases/25.12/start) were consulted rather than treating the existing 23.05 reference profile as a current universal contract.

## Transport decision

Use a supervised external OpenSSH client, with separate arguments and a fixed reviewed remote query registry. The inspected Windows implementation uses Git for Windows OpenSSH at `C:\Program Files\Git\usr\bin\ssh.exe` and sibling keyscan/keygen executables; it does not assume the Windows optional-feature client is installed. Client version and genuine Dropbear interoperability must be recorded by executed checks. [OpenSSH options](https://man.openbsd.org/ssh_config) support strict known-host checking, explicit key selection, disabling password prompts/agents/forwarding/proxies, bounded connection attempts, and source-address binding. The Rust supervisor additionally bounds total execution and both output pipes and kills/reaps its child on cancellation or failure.

[russh](https://docs.rs/russh/latest/russh/) was evaluated as an in-process alternative. It would add asynchronous transport and crypto dependencies to the current synchronous adapter framework. The existing system OpenSSH route avoids a new crypto implementation/dependency graph and can be independently exercised on Windows. This decision still requires genuine Dropbear interoperability and negative tests; client availability alone is not acceptance.

Initial scope is Ed25519 host keys and an existing key file. Passwords, askpass helpers, automatic credential retries and algorithm downgrades are unavailable. Private key contents never need to enter React. An observed key is untrusted until its SHA-256 fingerprint is compared with independent trusted information. Per-connection known-host files contain public keys only. Global host files and user SSH config must not override the selected pin.

An encrypted key requiring interactive unlock is unavailable in the batch/no-agent scope. An IP address, displayed fingerprint or claimed OpenWrt label grants no configuration authority. Endpoint, port, source interface/address, identity, operation and expiry remain separately bound by backend policy. A privileged account may itself write; OBTAINER's reviewed query dispatch enforces this phase's inspection boundary.

The native key picker reserves the backend operation slot, supports cancellation and a bounded selection deadline, and keeps its path/content out of IPC. The backend re-resolves the selected saved endpoint, interface and evidence after selection and before authentication. Discovery changes cancel in-flight authority. Established reads use the earliest session/grant/evidence expiry and reject stale publication; GUI state alone cannot grant these permissions.

Local investigation selected Git for Windows OpenSSH **10.2** after Windows OpenSSH **9.5** keyscan could not negotiate the guest's key exchange. No algorithm downgrade was introduced. Windows invocation also required a safely represented private known-hosts path: forward slashes inside the SSH option's single-quoted value handled spaces, while an earlier nested-double-quote form failed. These are implementation findings to cover with transport regression and genuine-guest evidence, not a broad compatibility claim.

## Reviewed query contract

| Query | Purpose and format | Bound | Sensitivity/limitations |
| --- | --- | --- | --- |
| `ubus call system board` | JSON model, board name, firmware release/revision/target and optional root filesystem type | 16 KiB | Authenticated server report; does not independently prove hardware revision |
| Fixed ucode UCI projection of network interfaces | JSON interface names/protocols, DNS, peer-DNS, APN source/IP family/authentication method | 64 KiB | Excludes passwords and SIM PINs; configuration is not effective runtime DNS |
| Fixed ucode UCI projection of wireless interfaces | JSON radio, BSS name, SSID, configured maxassoc/mode/disabled | 64 KiB | Excludes Wi-Fi keys; configured maxassoc does not prove driver capacity |
| Fixed ucode netifd runtime projection | Interface name/up/protocol and runtime-reported DNS only | 64 KiB | Full protocol `data` remains on the router; no functional resolver-use claim |
| Fixed ucode UCI DHCP projection | Interface, DNS-specific IPv4 DHCP options and IPv6 DNS fields | 64 KiB | Stored advertisement configuration, not proof of what LAN clients received |
| Fixed ucode UCI radio projection | Radio name/type, configured radio-wide maxassoc and disabled | 64 KiB | Radio aggregate configuration is distinct from per-BSS capacity |
| Fixed ucode hostapd projection | Reviewed BSS object name, driver, PHY, status and associated-station count | 64 KiB | Counts projected before transmission; no MAC-address export or capacity claim |
| `cat /proc/mtd` | Partition identifiers, names, sizes and erase sizes | 16 KiB | Metadata only; file may be absent on x86; never reads raw flash |

Each operation uses the existing adapter deadline/cancellation control. No dynamic remote shell fragment is accepted.

The official [procd implementation](https://raw.githubusercontent.com/openwrt/procd/master/system.c) confirms that `system board` obtains model/board/release fields from platform files; optional fields depend on the build. Master source is supporting evidence, not proof that a particular release returns all fields. Actual selected-release outputs must be recorded during VM verification.

The selected release pins procd to `58eb263d5abe03f8c1280bdfa65a3b052614215d`. Its board response reports platform/model, optional rootfs type, and firmware release values from the guest; authenticated server reports do not independently certify physical hardware. Required missing values must fail qualification; optional missing values remain unknown. [Release pin](https://raw.githubusercontent.com/openwrt/openwrt/v25.12.6/package/system/procd/Makefile), [pinned board implementation](https://raw.githubusercontent.com/openwrt/procd/58eb263d5abe03f8c1280bdfa65a3b052614215d/system.c).

The following requested official wiki pages returned an Anubis challenge through the browsing tool, rather than their documentation contents: [ubus](https://openwrt.org/docs/techref/ubus), [wireless](https://openwrt.org/docs/guide-user/network/wifi/basic), [WAN protocols](https://openwrt.org/docs/guide-user/network/wan/wan_interface_protocols), [Dropbear key authentication](https://openwrt.org/docs/guide-user/security/dropbear.public-key.auth), [ImageBuilder](https://openwrt.org/docs/guide-user/additional-software/imagebuilder). Their contents are not claimed as read. Release-source/guest inspection must resolve implementation details before qualification.

## Capability interpretation

No modem or wireless hardware in the selected VM means physical Wi-Fi limits, APN and USSD cannot be qualified there. Cellular protocol names alone do not establish USSD support. Metadata reports cannot become firmware artifacts or backups. Official image availability is separate from exact hardware compatibility, authentication of downloaded artifacts, successful builds, installation qualification and recovery evidence.

## Exact target and evidence categories

The selected guest reports release revision `r33311-981c978eff` and model `QEMU Ubuntu 26.04 PC (i440FX + PIIX, 1996)`. These are virtual OS/platform observations. The published checksum identifies the compressed `generic-ext4-combined.img.gz`, not an arbitrary customized disk. The official directory distinguishes BIOS/EFI, ext4/squashfs and rootfs-only images and includes matching ImageBuilder/profile/manifests. A checksum comparison establishes equality with that published digest; release-signature validation and exact image/board compatibility are separate future acquisition requirements. [Official release artifacts](https://downloads.openwrt.org/releases/25.12.6/targets/x86/64/).

| Evidence class | Can establish | Cannot establish |
| --- | --- | --- |
| Controlled mock SSH/parser fixture | Approval, parser, error and resource behavior | Genuine OpenWrt interoperability |
| Genuine OpenWrt guest over SSH | Exact OS command and protocol behavior | Physical router, radio, modem, extraction, installation or recovery qualification |
| Proprietary MiFi reported by owner | Candidate for later device-specific research | OpenWrt identity or access |
| Authorized physical OpenWrt target | Individually recorded model/revision/firmware operations | Universal router or MiFi support |

No physical OpenWrt device is available. The owner's regular MiFi is outside this target; no physical contact or new qualification is authorized by the VM work. See [hardware qualification](PHASE_7_HARDWARE_QUALIFICATION.md).

## Additional source-reviewed projections

`apps/desktop/src-tauri/src/openwrt/query.rs`, `ReadQuery::command()`, is the executable source of truth. Board, Network, RuntimeDns, DhcpDns, Wireless, WirelessRadios, WirelessRuntime and Partitions are present in the current registry with typed parsers. Their existence establishes source implementation; genuine-guest results and native acceptance remain separate. Optional unavailable runtime sources produce unknown results and limitations; malformed successful responses fail parsing.

### DNS scopes

Keep three observations distinct:

1. UCI `network` interface `dns`/`peerdns`: configured upstream inputs and peer-DNS policy. Preserve unknown interface names and multiple interfaces.
2. UCI `dhcp`: per-interface LAN IPv4 DNS options and IPv6 DNS advertisement settings. These are not router upstream resolvers.
3. netifd runtime: currently reported interface `dns-server` values. Down/missing fields are not successful empty resolver tests, and a reported address is not proof that a DNS request used it.

The release pins netifd to `cbb83a1857407a28a63dc09412a1f209195914ef`. Its `network.interface`/`dump` response contains an `interface` array; active interfaces include `dns-server`. The full dump may also contain protocol-specific `data`. A safe remote projection emits only interface name, up, protocol and DNS before transmission; it must not return the complete dump. [Release pin](https://raw.githubusercontent.com/openwrt/openwrt/v25.12.6/package/network/config/netifd/Makefile), [pinned runtime source](https://raw.githubusercontent.com/openwrt/netifd/cbb83a1857407a28a63dc09412a1f209195914ef/ubus.c).

dnsmasq separately consumes configured upstream servers, resolver-file policy, and DHCP DNS advertisements. Therefore flattening `network.*.dns` cannot establish complete effective DNS policy or what a LAN client received. Safe DHCP projection selects only DNS-specific option entries and IPv6 DNS fields; unrelated DHCP options stay excluded. Do not execute the service script or change resolver files. [25.12.6 dnsmasq mapping](https://raw.githubusercontent.com/openwrt/openwrt/v25.12.6/package/network/services/dnsmasq/files/dnsmasq.init).

The ucode ubus module supports local method calls and an operation timeout in seconds. A constant script can connect with a short timeout, call the fixed runtime method, select approved fields and disconnect. Syntax and absent-object behavior still require actual guest checks. [ucode ubus reference](https://ucode-lang.org/ucode/module-ubus.html).

### Wireless evidence

The selected release supports both radio-wide and BSS `maxassoc`, mapped to different hostapd settings. Inspecting only BSS values misses a possible aggregate radio constraint. A stored limit does not prove driver capacity, a valid range or enforcement. [25.12.6 hostapd configuration mapping](https://raw.githubusercontent.com/openwrt/openwrt/v25.12.6/package/network/config/wifi-scripts/files/lib/netifd/hostapd.sh).

hostapd's ubus `get_status` can report state/driver/phy, and `get_clients` includes association flags. A reviewed runtime projection may return the BSS object name and count associated stations on the router. MAC addresses and detailed client records are unnecessary and should not cross the transport. Missing radio/runtime data stays unavailable. The radio-less guest cannot qualify physical station capacity. [25.12.6 hostapd ubus implementation](https://raw.githubusercontent.com/openwrt/openwrt/v25.12.6/package/network/services/hostapd/src/src/ap/ubus.c).

For bounded hostapd enumeration, ucode `connection.list()` without an argument returns path strings; passing an object-name pattern returns signatures instead. Filter a bounded path list to reviewed `hostapd.*` grammar locally, invoke only the two approved methods, and emit counts/selected status. This API distinction prevents treating signatures as dynamic command targets. [ucode connection reference](https://ucode-lang.org/ucode/module-ubus.connection.html).

The release-family ucode package is `2026.01.16~85922056-r2`. In the corresponding `85922056` source, no-argument `list()` uses the path callback, while a supplied object name selects the signatures callback; the connection timeout is stored in seconds. The selected release's netifd dependency list includes the ubus and UCI ucode modules, avoiding a package-install prerequisite for the stock target. [Pinned ucode ubus source](https://raw.githubusercontent.com/jow-/ucode/85922056/lib/ubus.c), [official 25.12.6 package index](https://downloads.openwrt.org/releases/25.12.6/packages/armeb_xscale/base/), [selected netifd dependencies](https://raw.githubusercontent.com/openwrt/openwrt/v25.12.6/package/network/config/netifd/Makefile).

### Cellular and passive USSD

The exact QMI handler declares `apn`, `auth` and `pdptype`; a universal `pdp` assumption would miss its IP-family setting. The handler also declares credentials and PIN fields that must be excluded. Automatic APN is protocol/vendor-specific. A protocol label or stored APN does not establish a present modem or live USSD capability. Do not run a protocol setup script for inspection: it performs active modem actions. [25.12.6 QMI implementation](https://raw.githubusercontent.com/openwrt/openwrt/v25.12.6/package/network/utils/uqmi/files/lib/netifd/proto/qmi.sh).

Phase 7 sends no AT requests, live USSD, SIM-changing request, registration/reconnect operation or modem setup command. Exact modem, firmware, interface and passive capability evidence remain required; absent evidence is UNKNOWN/UNSUPPORTED with a reason.

### Optional x86 DMI identity

A separately reviewed fixed read may project only `/sys/class/dmi/id/sys_vendor`, `product_name`, `product_version`, `board_vendor`, `board_name` and `board_version`. Firmware can omit any field. Product version and baseboard version are distinct and cannot silently substitute for each other. Avoid UUIDs, serials and asset tags. Virtual DMI remains virtual evidence; missing revision must stay unknown. [Kernel DMI implementation](https://kernel.googlesource.com/pub/scm/linux/kernel/git/torvalds/linux/%2B/d39a1d7486d98668dd34aaa6732aad7977c45f5a/drivers/firmware/dmi-id.c).

## Firmware interfaces and retained goals

`/proc/mtd` describes MTD metadata; its absence does not establish missing storage on a block-backed x86 target. It cannot prove complete UBI/block layout, bootloader/calibration coverage, image format, signing/encryption, extraction support or recovery.

The upgrade implementation distinguishes configuration archives, image checking and installation. A configuration archive is not an original firmware image. Even test-mode upgrade dispatch is outside the Phase 7 allowlist. Later engines need platform-specific qualification. [25.12.6 sysupgrade implementation](https://raw.githubusercontent.com/openwrt/openwrt/v25.12.6/package/base-files/files/sbin/sysupgrade).

Official acquisition, device-specific extraction, supported customization, rebuilding, installation and recovery remain permanent product goals alongside direct configuration transactions. See [firmware feasibility](PHASE_7_FIRMWARE_FEASIBILITY.md) and proposed [pipeline milestones](FIRMWARE_PIPELINE_ROADMAP.md).

Source paths that returned 404/cache errors were not treated as evidence. Requested wiki pages challenged by Anubis were not treated as read; pinned primary source and actual guest outputs provide narrower evidence.
