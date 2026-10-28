param(
    [ValidatePattern('^[0-9A-Fa-f]{4}$')][string]$VendorId = '1286',
    [ValidatePattern('^[0-9A-Fa-f]{4}$')][string]$ProductId = '4E31',
    [string]$OutputPath = '.tools\physical-sprint-inventory.json',
    [switch]$SelfTest
)

# Local Windows observations only. No socket, HTTP, router command, database or
# credential access. Unique identifiers are salted per run; salt is not retained.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))

function Resolve-ReportPath([string]$RelativePath) {
    if ([IO.Path]::IsPathRooted($RelativePath) -or $RelativePath.Contains(':')) { throw 'UNSAFE_OUTPUT' }
    $parts = @($RelativePath.Replace('/', '\').Split('\'))
    if ($parts.Count -lt 2 -or $parts[0] -cne '.tools') { throw 'UNSAFE_OUTPUT' }
    foreach ($part in $parts) {
        if ($part -in @('', '.', '..') -or $part -notmatch '^[A-Za-z0-9._-]+$' -or
            $part.EndsWith('.') -or $part -match '^(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)') { throw 'UNSAFE_OUTPUT' }
    }
    if ([IO.Path]::GetExtension($RelativePath) -cne '.json') { throw 'UNSAFE_OUTPUT' }
    $result = [IO.Path]::GetFullPath((Join-Path $repoRoot $RelativePath))
    $prefix = (Join-Path $repoRoot '.tools') + [IO.Path]::DirectorySeparatorChar
    if (-not $result.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'UNSAFE_OUTPUT' }
    return $result
}

function New-RunSalt {
    $bytes = New-Object byte[] 32
    $rng = [Security.Cryptography.RandomNumberGenerator]::Create()
    try { $rng.GetBytes($bytes) } finally { $rng.Dispose() }
    return [Convert]::ToBase64String($bytes)
}

function Get-PrivateReference([string]$Kind, [string]$Value, [string]$Salt) {
    if ([string]::IsNullOrWhiteSpace($Value)) { return $null }
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [Text.Encoding]::UTF8.GetBytes("OBTAINER/local-inventory/v1|$Salt|$Kind|$($Value.Trim().ToUpperInvariant())")
        return ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
    } finally { $sha.Dispose() }
}

function Get-SafeName([string]$Value) {
    # Never serialize arbitrary friendly names. Best-effort regex scrubbing
    # cannot establish that an unlabelled serial or other secret was removed.
    return 'Windows device (name omitted)'
}

function Get-ObservedClass([string]$Value) {
    foreach ($allowed in @('USB', 'Net', 'Ports', 'Modem', 'WPD', 'USBDevice')) {
        if ($Value -ieq $allowed) { return $allowed }
    }
    return 'unknown'
}

function Get-ObservedStatus([string]$Value) {
    foreach ($allowed in @('OK', 'Error', 'Degraded', 'Unknown', 'Pred Fail', 'Starting', 'Stopping', 'Service', 'Stressed', 'NonRecover', 'No Contact', 'Lost Comm')) {
        if ($Value -ieq $allowed) { return $allowed }
    }
    return 'Unknown'
}

function Get-ObservedPresence($Record) {
    $property = $Record.PSObject.Properties['Present']
    if ($null -ne $property -and $property.Value -is [bool]) { return $property.Value }
    return $null
}

function Get-PresentRecordMap($Records) {
    $map = @{}
    foreach ($row in $Records) {
        if ((Get-ObservedPresence $row) -eq $true -and $row.PNPDeviceID) { $map[$row.PNPDeviceID.ToUpperInvariant()] = $true }
    }
    return $map
}

function Get-CandidateConnectionStatus($Records) {
    $rows = @($Records)
    if ($rows.Count -eq 0) { return 'absent' }
    $unknown = $false
    foreach ($row in $rows) {
        $present = Get-ObservedPresence $row
        if ($present -eq $true) { return 'connected_candidate' }
        if ($null -eq $present) { $unknown = $true }
    }
    if ($unknown) { return 'unknown' }
    return 'absent'
}

function Get-UsbClassTuples($Values) {
    # Windows compatible IDs describe a device, interface or interface collection;
    # they are not a freshly fetched raw USB descriptor or an access capability.
    # https://learn.microsoft.com/en-us/windows-hardware/drivers/install/standard-usb-identifiers
    # DFU 1.1 Tables 4.1/4.4: FE/01/01 runtime, FE/01/02 DFU mode.
    # Table 4.2 defines upload capability separately in functional bmAttributes.
    # https://www.usb.org/sites/default/files/DFU_1.1.pdf
    # U-Boot DFU requires configured support and board-specific backends, not a
    # universal firmware access path: https://docs.u-boot.org/en/latest/usage/dfu.html
    # RNDIS tuple is a network hint only:
    # https://support.microsoft.com/en-gb/topic/a187837e-e438-7a59-a858-56af2ceee84e
    $seen = @{}
    foreach ($value in @($Values | Select-Object -First 64)) {
        if ($value -isnot [string] -or $value.Length -gt 64) { continue }
        $canonical = $value.ToUpperInvariant()
        $match = [regex]::Match($canonical, '\AUSB\\CLASS_(?<class>[0-9A-F]{2})(?:&SUBCLASS_(?<sub>[0-9A-F]{2})(?:&PROT_(?<protocol>[0-9A-F]{2}))?)?\z')
        if (-not $match.Success -or $seen.ContainsKey($canonical)) { continue }
        $seen[$canonical] = $true
        $classCode = $match.Groups['class'].Value
        $subclassCode = $null
        $protocolCode = $null
        if ($match.Groups['sub'].Success) { $subclassCode = $match.Groups['sub'].Value }
        if ($match.Groups['protocol'].Success) { $protocolCode = $match.Groups['protocol'].Value }
        $classification = 'usb_class_tuple_observed'
        if ($classCode -eq 'FE' -and $subclassCode -eq '01' -and $protocolCode -eq '01') {
            $classification = 'dfu_runtime_advertised_only'
        } elseif ($classCode -eq 'FE' -and $subclassCode -eq '01' -and $protocolCode -eq '02') {
            $classification = 'dfu_mode_advertised_only'
        } elseif ($classCode -eq 'FF') {
            $classification = 'vendor_specific_hint'
        } elseif ($classCode -eq 'E0' -and $subclassCode -eq '01' -and $protocolCode -eq '03') {
            $classification = 'rndis_network_hint'
        }
        [ordered]@{
            classCode=$classCode; subclassCode=$subclassCode; protocolCode=$protocolCode
            source='windows_pnp_compatible_id'; verification='observed'
            classification=$classification; firmwareAccess='unknown'; uploadSupport='unknown'; acquisitionAvailable=$false
        }
    }
}

function Get-IPv4Range([string]$Address, [int]$PrefixLength) {
    $parsed = $null
    if ($PrefixLength -lt 0 -or $PrefixLength -gt 32 -or
        -not [Net.IPAddress]::TryParse($Address, [ref]$parsed) -or
        $parsed.AddressFamily -ne [Net.Sockets.AddressFamily]::InterNetwork) { return $null }
    $bytes = $parsed.GetAddressBytes()
    [uint64]$value = ([uint64]$bytes[0] -shl 24) -bor ([uint64]$bytes[1] -shl 16) -bor ([uint64]$bytes[2] -shl 8) -bor [uint64]$bytes[3]
    [uint64]$mask = 0
    if ($PrefixLength -gt 0) { $mask = (([uint64]4294967295) -shl (32 - $PrefixLength)) -band ([uint64]4294967295) }
    $start = $value -band $mask
    return @($start, ($start -bor (([uint64]4294967295) -bxor $mask)))
}

function Test-PrefixOverlap($Left, $Right) {
    $a = Get-IPv4Range $Left.address $Left.prefixLength
    $b = Get-IPv4Range $Right.address $Right.prefixLength
    return ($null -ne $a -and $null -ne $b -and $a[0] -le $b[1] -and $b[0] -le $a[1])
}

function Assert-Check([bool]$Condition, [string]$Name) {
    $script:selfTestChecks++
    if (-not $Condition) { throw "SELF_TEST_FAILED:$Name" }
}

if ($SelfTest) {
    $script:selfTestChecks = 0
    $salt = New-RunSalt
    $raw = 'USB\VID_1286&PID_4E31\SYNTHETIC_UNIT_A_123456789012345'
    $refA = Get-PrivateReference 'pnp' $raw $salt
    Assert-Check ($refA -match '^[0-9a-f]{64}$' -and -not $refA.Contains('123456')) 'redaction'
    Assert-Check ($refA -eq (Get-PrivateReference 'pnp' $raw.ToLowerInvariant() $salt)) 'canonical_reference'
    Assert-Check ($refA -ne (Get-PrivateReference 'pnp' ($raw.Replace('UNIT_A', 'UNIT_B')) $salt)) 'different_physical_identifier'
    Assert-Check ($refA -ne (Get-PrivateReference 'pnp' $raw (New-RunSalt))) 'per_run_salt'
    Assert-Check ((Get-SafeName "Synthetic`r`nMiFi") -eq 'Windows device (name omitted)') 'arbitrary_name_omitted'
    Assert-Check ((Get-SafeName ('x ' * 100)).Length -le 96) 'name_length'
    Assert-Check ((Get-SafeName 'IMEI 123456789012345') -notmatch '123456789012345') 'labelled_secret'
    Assert-Check ((Get-SafeName 'MiFi 123456789012345 00:11:22:33:44:55') -notmatch '123456789012345|00:11') 'numeric_mac_redaction'
    Assert-Check ((Get-SafeName 'MiFi 0123456789ABCDEF') -notmatch '0123456789ABCDEF') 'unlabelled_identifier_name'
    Assert-Check ((Get-SafeName 'Synthetic 12345678-1234-1234-1234-123456789abc') -notmatch '12345678-1234') 'guid_redaction'
    Assert-Check ((Get-SafeName ("Synthetic" + [char]0x202e + 'MiFi')) -notmatch '\p{C}') 'bidi_name_controls'
    Assert-Check ((Get-SafeName $raw) -notmatch 'VID_1286|SYNTHETIC_UNIT') 'pnp_name_redaction'
    Assert-Check ((Get-ObservedClass 'USB\SYNTHETIC_SECRET') -eq 'unknown') 'class_allowlist'
    Assert-Check ((Get-ObservedStatus 'SYNTHETIC_SECRET') -eq 'Unknown') 'status_allowlist'
    $present = [pscustomobject]@{Present=$true;PNPDeviceID='synthetic-present'}
    $disconnected = [pscustomobject]@{Present=$false;PNPDeviceID='synthetic-disconnected'}
    $missing = [pscustomobject]@{PNPDeviceID='synthetic-missing-status'}
    $nullPresence = [pscustomobject]@{Present=$null;PNPDeviceID='synthetic-null-status'}
    $presentMap = Get-PresentRecordMap @($present, $disconnected, $missing, $nullPresence)
    Assert-Check ($presentMap.Count -eq 1 -and $presentMap.ContainsKey('SYNTHETIC-PRESENT')) 'only_present_records_correlate'
    Assert-Check ((Get-CandidateConnectionStatus @($present, $disconnected)) -eq 'connected_candidate') 'present_candidate'
    Assert-Check ((Get-CandidateConnectionStatus @($disconnected)) -eq 'absent') 'disconnected_is_absent'
    Assert-Check ((Get-CandidateConnectionStatus @($missing)) -eq 'unknown') 'missing_presence_is_unknown'
    Assert-Check ((Get-CandidateConnectionStatus @($nullPresence, $disconnected)) -eq 'unknown') 'null_presence_is_unknown'
    Assert-Check ((Get-CandidateConnectionStatus @()) -eq 'absent') 'no_matches_is_absent'
    $runtimeDfu = @(Get-UsbClassTuples @('USB\Class_FE&SubClass_01&Prot_01'))
    Assert-Check ($runtimeDfu.Count -eq 1) 'dfu_runtime_tuple_observed'
    Assert-Check ($runtimeDfu[0].classification -eq 'dfu_runtime_advertised_only') 'dfu_runtime_not_dfu_mode'
    $modeDfu = @(Get-UsbClassTuples @('USB\Class_FE&SubClass_01&Prot_02'))
    Assert-Check ($modeDfu.Count -eq 1 -and $modeDfu[0].classification -eq 'dfu_mode_advertised_only') 'dfu_mode_tuple_observed'
    foreach ($entry in @($runtimeDfu + $modeDfu)) {
        Assert-Check ($entry.firmwareAccess -eq 'unknown' -and $entry.uploadSupport -eq 'unknown' -and -not $entry.acquisitionAvailable) 'dfu_does_not_authorize_access'
    }
    foreach ($invalidId in @('USB', 'Net', 'Mobile DFU Interface', 'MiFi 0123456789ABCDEF', 'USB\VID_1286&PID_4E31\SYNTHETIC_SERIAL', 'USB\Class_FE&SubClass_01&Prot_01&SERIAL_123456', 'USB\Class_FE&Prot_01', 'USB\Class_FEE&SubClass_01&Prot_01', 'USB\Class_FE&SubClass_01&Prot_G1', 'USB\MS_COMP_RNDIS', "USB\Class_FE&SubClass_01&Prot_01`n", ' USB\Class_FE&SubClass_01&Prot_01')) {
        Assert-Check (@(Get-UsbClassTuples @($invalidId)).Count -eq 0) 'noncanonical_compatible_id_rejected'
    }
    $partialDfu = @(Get-UsbClassTuples @('USB\Class_FE', 'USB\Class_FE&SubClass_01', 'USB\Class_FE&SubClass_01&Prot_00', 'USB\Class_FE&SubClass_02&Prot_01'))
    Assert-Check (@($partialDfu | Where-Object { $_.classification -like 'dfu_*' }).Count -eq 0) 'incomplete_or_other_tuple_not_dfu'
    $vendorTuple = @(Get-UsbClassTuples @('USB\Class_FF&SubClass_00&Prot_00'))
    Assert-Check ($vendorTuple.Count -eq 1 -and $vendorTuple[0].classification -eq 'vendor_specific_hint' -and $vendorTuple[0].firmwareAccess -eq 'unknown') 'vendor_specific_not_firmware_access'
    $rndisTuple = @(Get-UsbClassTuples @('USB\Class_E0&SubClass_01&Prot_03'))
    Assert-Check ($rndisTuple.Count -eq 1 -and -not $rndisTuple[0].acquisitionAvailable -and $rndisTuple[0].firmwareAccess -eq 'unknown') 'network_tuple_not_firmware_access'
    Assert-Check (@(Get-UsbClassTuples @('usb\class_fe&subclass_01&prot_01', 'USB\Class_FE&SubClass_01&Prot_01')).Count -eq 1) 'canonical_case_deduplication'
    Assert-Check ((Resolve-ReportPath '.tools/synthetic-inventory.json').StartsWith((Join-Path $repoRoot '.tools'))) 'contained_output'
    foreach ($invalid in @('outside.json', '.tools/../outside.json', '.tools-x/report.json', '.tools/a.json:secret', 'C:\outside.json', '.tools/NUL.json', '.tools/a./report.json')) {
        $rejected = $false
        try { $null = Resolve-ReportPath $invalid } catch { $rejected = $true }
        Assert-Check $rejected 'unsafe_output'
    }
    Assert-Check (Test-PrefixOverlap @{address='192.0.2.7';prefixLength=24} @{address='192.0.2.8';prefixLength=25}) 'overlapping_prefix'
    Assert-Check (-not (Test-PrefixOverlap @{address='192.0.2.7';prefixLength=24} @{address='198.51.100.7';prefixLength=24})) 'distinct_prefix'
    @{ synthetic = $true; checks = $script:selfTestChecks; failed = 0; physicalQualification = $false } | ConvertTo-Json -Compress
    exit 0
}

$directoryLeases = New-Object 'System.Collections.Generic.List[System.IDisposable]'
try {
    $reportPath = Resolve-ReportPath $OutputPath
    # Hold every output ancestor without FILE_SHARE_DELETE. OPEN_REPARSE_POINT
    # and handle metadata reject a swapped junction rather than following it.
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
public static class ObtainerInventoryDirectory {
    [StructLayout(LayoutKind.Sequential)] struct AttributeTag { public uint Attributes; public uint Tag; }
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern SafeFileHandle CreateFile(string name, uint access, uint share, IntPtr security, uint creation, uint flags, IntPtr template);
    [DllImport("kernel32.dll", SetLastError=true)]
    static extern bool GetFileInformationByHandleEx(SafeFileHandle handle, int kind, out AttributeTag info, uint size);
    public static SafeFileHandle Hold(string path) {
        var handle = CreateFile(path, 0x80000000, 3, IntPtr.Zero, 3, 0x02200000, IntPtr.Zero);
        if (handle.IsInvalid) { handle.Dispose(); throw new InvalidOperationException("UNSAFE_OUTPUT_PARENT"); }
        AttributeTag info;
        if (!GetFileInformationByHandleEx(handle, 9, out info, 8) || (info.Attributes & 0x400) != 0 || (info.Attributes & 0x10) == 0) {
            handle.Dispose(); throw new InvalidOperationException("UNSAFE_OUTPUT_PARENT");
        }
        return handle;
    }
}
'@
    $directoryLeases.Add([ObtainerInventoryDirectory]::Hold($repoRoot))
    $parent = Split-Path -Parent $reportPath
    $relativeParent = $parent.Substring($repoRoot.Length).TrimStart('\')
    $cursor = $repoRoot
    foreach ($part in $relativeParent.Split('\')) {
        $cursor = Join-Path $cursor $part
        $directoryLeases.Add([ObtainerInventoryDirectory]::Hold($cursor))
    }
    if ([IO.File]::Exists($reportPath) -or [IO.Directory]::Exists($reportPath)) { throw 'OUTPUT_EXISTS' }

    $salt = New-RunSalt
    $pnp = @(Get-CimInstance -ClassName Win32_PnPEntity -Property PNPDeviceID,PNPClass,Status,Present,ConfigManagerErrorCode)
    $adapters = @(Get-CimInstance -ClassName Win32_NetworkAdapter -Property PNPDeviceID,GUID,InterfaceIndex,PhysicalAdapter,NetEnabled)
    $links = @(Get-NetAdapter -IncludeHidden)
    $ips = @(Get-NetIPAddress -AddressFamily IPv4)
    $dns = @(Get-DnsClientServerAddress)
    $routes = @(Get-NetRoute -AddressFamily IPv4)
    $pattern = '^USB\\VID_' + $VendorId + '&PID_' + $ProductId + '(?:&|\\)'
    $usbRecords = @($pnp | Where-Object { $_.PNPDeviceID -match $pattern })
    $matchedIds = Get-PresentRecordMap $usbRecords
    $allPresentIds = Get-PresentRecordMap $pnp
    $parentCache = @{}
    $script:unknownParents = 0
    $script:unknownCompatibleIds = 0
    function Get-ParentId([string]$Id) {
        if ([string]::IsNullOrWhiteSpace($Id)) { return $null }
        $key = $Id.ToUpperInvariant()
        if (-not $parentCache.ContainsKey($key)) {
            try {
                $property = Get-PnpDeviceProperty -InstanceId $Id -KeyName 'DEVPKEY_Device_Parent' -ErrorAction Stop
                $parentCache[$key] = [string]$property.Data
            } catch { $parentCache[$key] = $null; $script:unknownParents++ }
        }
        return $parentCache[$key]
    }
    function Get-MatchedAncestors([string]$Id) {
        $seen = @{}
        $current = $Id
        for ($depth = 0; $depth -lt 32 -and $current; $depth++) {
            $key = $current.ToUpperInvariant()
            if ($seen.ContainsKey($key)) { break }
            $seen[$key] = $true
            if ($matchedIds.ContainsKey($key)) { Get-PrivateReference 'pnp' $key $salt }
            $current = Get-ParentId $current
        }
    }
    $deviceReports = @($usbRecords | ForEach-Object {
        $classTuples = @()
        $compatibleIdStatus = 'property_unavailable'
        try {
            $compatibleProperty = Get-PnpDeviceProperty -InstanceId $_.PNPDeviceID -KeyName 'DEVPKEY_Device_CompatibleIds' -ErrorAction Stop
            $classTuples = @(Get-UsbClassTuples $compatibleProperty.Data)
            $compatibleIdStatus = 'no_canonical_class_tuple'
            if ($classTuples.Count -gt 0) { $compatibleIdStatus = 'observed' }
            if (@($compatibleProperty.Data).Count -gt 64) { $compatibleIdStatus = 'truncated_property_observation' }
        } catch { $script:unknownCompatibleIds++ }
        [ordered]@{
            reference = Get-PrivateReference 'pnp' $_.PNPDeviceID $salt
            parentReference = Get-PrivateReference 'pnp' (Get-ParentId $_.PNPDeviceID) $salt
            vendorId = $VendorId.ToUpperInvariant(); productId = $ProductId.ToUpperInvariant()
            name = 'USB ' + $VendorId.ToUpperInvariant() + ':' + $ProductId.ToUpperInvariant() + ' device record'; class = Get-ObservedClass $_.PNPClass
            present = Get-ObservedPresence $_; windowsStatus = Get-ObservedStatus $_.Status
            problemCode = $_.ConfigManagerErrorCode; identityVerification = 'unverified'
            usbClassTupleObservationStatus = $compatibleIdStatus; usbClassTuples = $classTuples
            usbNodeScope = $(if ($_.PNPDeviceID -match '&MI_[0-9A-F]{2}\\') { 'interface_or_interface_collection' } else { 'device_or_composite_parent' })
        }
    })
    $networkReports = @($adapters | Where-Object { $null -ne $_.InterfaceIndex } | ForEach-Object {
        $adapter = $_
        $index = [int]$adapter.InterfaceIndex
        $link = @($links | Where-Object { $_.ifIndex -eq $index } | Select-Object -First 1)
        $linked = $link.Count -gt 0 -and [string]$link[0].Status -eq 'Up'
        $pnpPresent = $adapter.PNPDeviceID -and $allPresentIds.ContainsKey($adapter.PNPDeviceID.ToUpperInvariant())
        $related = @()
        if ($pnpPresent) { $related = @(Get-MatchedAncestors $adapter.PNPDeviceID) }
        $addresses = @($ips | Where-Object { $_.InterfaceIndex -eq $index -and $_.IPAddress -notmatch '^(127\.|0\.)' } | ForEach-Object {
            [ordered]@{ address = [string]$_.IPAddress; prefixLength = [int]$_.PrefixLength; addressState = [string]$_.AddressState }
        })
        [ordered]@{
            adapterReference = Get-PrivateReference 'adapter' ([string]$adapter.GUID) $salt
            pnpReference = Get-PrivateReference 'pnp' $adapter.PNPDeviceID $salt
            interfaceIndex = $index; name = $(if ($related.Count -gt 0) { 'USB-correlated Windows network adapter' } else { 'Windows network adapter' })
            physicalAdapterObserved = $adapter.PhysicalAdapter; linkUp = $linked; pnpPresent = [bool]$pnpPresent
            correlatedUsbReferences = $related; ipv4Addresses = $addresses
            dnsServers = @($dns | Where-Object { $_.InterfaceIndex -eq $index } | ForEach-Object { $_.ServerAddresses } | Sort-Object -Unique)
            defaultRoutes = @($routes | Where-Object { $_.InterfaceIndex -eq $index -and $_.DestinationPrefix -eq '0.0.0.0/0' } | ForEach-Object {
                [ordered]@{ nextHop = [string]$_.NextHop; routeMetric = [int]$_.RouteMetric; protocol = [string]$_.Protocol }
            })
        }
    })
    $collisions = @()
    foreach ($target in @($networkReports | Where-Object { $_.pnpPresent -and $_.linkUp -and $_.correlatedUsbReferences.Count -gt 0 })) {
        foreach ($other in @($networkReports | Where-Object { $_.pnpPresent -and $_.linkUp -and $_.interfaceIndex -ne $target.interfaceIndex })) {
            $overlap = $false
            foreach ($a in $target.ipv4Addresses) { foreach ($b in $other.ipv4Addresses) { if (Test-PrefixOverlap $a $b) { $overlap = $true } } }
            if ($overlap) { $collisions += [ordered]@{ selectedInterfaceIndex=$target.interfaceIndex; overlappingInterfaceIndex=$other.interfaceIndex; reason='overlapping_observed_ipv4_prefixes_not_verified_endpoint_ownership' } }
        }
    }
    $report = [ordered]@{
        schemaVersion = 1; collectedAt = [DateTime]::UtcNow.ToString('o')
        provenance = 'windows_observed'; identityVerification = 'unverified'
        connectionStatus = Get-CandidateConnectionStatus $usbRecords
        matchingUsbRecordCount = $usbRecords.Count; presentMatchingUsbRecordCount = $matchedIds.Count
        presentUsbCorrelatedNetworkCount = @($networkReports | Where-Object { $_.pnpPresent -and $_.correlatedUsbReferences.Count -gt 0 }).Count
        activeUsbCorrelatedNetworkCount = @($networkReports | Where-Object { $_.pnpPresent -and $_.linkUp -and $_.correlatedUsbReferences.Count -gt 0 }).Count
        presentDfuRuntimeAdvertisedRecordCount = @($deviceReports | Where-Object { $_.present -eq $true -and @($_.usbClassTuples | Where-Object { $_.classification -eq 'dfu_runtime_advertised_only' }).Count -gt 0 }).Count
        presentDfuModeAdvertisedRecordCount = @($deviceReports | Where-Object { $_.present -eq $true -and @($_.usbClassTuples | Where-Object { $_.classification -eq 'dfu_mode_advertised_only' }).Count -gt 0 }).Count
        acquisitionAttempted = $false; routerRequests = 0
        query = @{ usbVendorId=$VendorId.ToUpperInvariant(); usbProductId=$ProductId.ToUpperInvariant() }
        redaction = @{ identifiers='salted_sha256'; lifetime='per_run_only'; saltRetained=$false; names='fixed_labels_only'; ssidCollected=$false; rawPropertiesRetained=$false }
        matchingUsbRecords = $deviceReports
        networks = @($networkReports | Where-Object { $_.linkUp -or $_.correlatedUsbReferences.Count -gt 0 })
        subnetCollisions = $collisions
        parentObservationFailures = $script:unknownParents
        compatibleIdObservationFailures = $script:unknownCompatibleIds
        limitations = @('Windows identifiers and branding do not verify router identity.', 'Per-run references cannot establish that different captures represent the same physical unit.', 'Configured Windows DNS and routes are observations, not device configuration.', 'No firmware bytes or authentication evidence collected.', 'Canonical USB class tuples are cached Windows compatible-ID observations; descriptor scope depends on the PnP node.', 'DFU advertisement does not establish upload capability, authorized ranges, firmware coverage, a working driver or extraction support. No DFU requests or mode changes were sent.', 'No observed DFU tuple does not prove the device has no other service modes. Vendor-specific and RNDIS hints do not establish firmware access.')
    }
    $bytes = [Text.Encoding]::UTF8.GetBytes(($report | ConvertTo-Json -Depth 12) + "`n")
    # CreateNew prevents overwrite, including a destination created during capture.
    $stream = [IO.File]::Open($reportPath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try { $stream.Write($bytes, 0, $bytes.Length); $stream.Flush($true) } finally { $stream.Dispose() }
    @{ output=$OutputPath; matchingUsbRecords=$deviceReports.Count; presentMatchingUsbRecords=$report.presentMatchingUsbRecordCount; presentUsbCorrelatedNetworks=$report.presentUsbCorrelatedNetworkCount; activeUsbCorrelatedNetworks=$report.activeUsbCorrelatedNetworkCount; connectionStatus=$report.connectionStatus; collisions=$collisions.Count; dfuRuntimeAdvertisedRecords=$report.presentDfuRuntimeAdvertisedRecordCount; dfuModeAdvertisedRecords=$report.presentDfuModeAdvertisedRecordCount; provenance='windows_observed'; identityVerification='unverified'; routerRequests=0 } | ConvertTo-Json -Compress
} catch {
    # Provider exceptions can contain raw instance IDs. Do not echo them.
    Write-Error 'Local inventory stopped. No router request was sent. Check output containment/nonexistence, ordinary output parents, and Windows enumeration availability.'
    exit 1
} finally {
    for ($i = $directoryLeases.Count - 1; $i -ge 0; $i--) { $directoryLeases[$i].Dispose() }
}
