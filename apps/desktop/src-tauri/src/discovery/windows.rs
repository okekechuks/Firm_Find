//! Read-only Windows inventory. No device handles, sockets, driver changes or elevation.
use super::{service::Signal, AddressPrefix, DefaultRoute, Inventory, NetworkAdapter, RawNode};
use std::{
    collections::BTreeMap,
    ffi::c_void,
    mem::size_of,
    net::{Ipv4Addr, Ipv6Addr},
    ptr,
    sync::mpsc::SyncSender,
};
use windows_sys::{
    core::GUID,
    Win32::{
        Devices::{
            DeviceAndDriverInstallation::*,
            Properties::{DEVPKEY_Device_ContainerId, DEVPROP_TYPE_GUID},
        },
        Foundation::{
            GetLastError, ERROR_BUFFER_OVERFLOW, ERROR_NO_DATA, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS,
            HANDLE, INVALID_HANDLE_VALUE,
        },
        NetworkManagement::{IpHelper::*, Ndis::IfOperStatusUp},
        Networking::WinSock::{
            AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6, SOCKADDR_INET, SOCKET_ADDRESS,
        },
        System::Registry::{
            RegCloseKey, RegQueryValueExW, HKEY, KEY_QUERY_VALUE, REG_MULTI_SZ, REG_SZ,
        },
    },
};

const USB_INTERFACE: GUID = GUID::from_u128(0xa5dcbf10_6530_11d2_901f_00c04fb951ed);
const SERIAL_INTERFACE: GUID = GUID::from_u128(0x86e0d1e0_8089_11d0_9ce4_08003e301f73);
const NET_INTERFACE: GUID = GUID::from_u128(0xcac88484_7515_4c03_82e6_71a87abac361);
const MAX_BUFFER: usize = 4 * 1024 * 1024;

fn guid(value: &GUID) -> String {
    format!(
        "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        value.data1,
        value.data2,
        value.data3,
        value.data4[0],
        value.data4[1],
        value.data4[2],
        value.data4[3],
        value.data4[4],
        value.data4[5],
        value.data4[6],
        value.data4[7]
    )
}

fn wide(value: &[u16]) -> String {
    String::from_utf16_lossy(
        &value[..value
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(value.len())],
    )
}

unsafe fn wide_ptr(value: *const u16) -> String {
    if value.is_null() {
        return String::new();
    }
    // These pointers come from a successful Windows API call and remain in its owned buffer.
    let mut length = 0;
    while length < 32768 && *value.add(length) != 0 {
        length += 1;
    }
    wide(std::slice::from_raw_parts(value, length))
}

fn os_error(api: &str) -> String {
    format!("{api} failed (Windows error {})", unsafe { GetLastError() })
}

struct DeviceSet(HDEVINFO);
impl Drop for DeviceSet {
    fn drop(&mut self) {
        unsafe {
            SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}

fn instance_id(set: HDEVINFO, info: &SP_DEVINFO_DATA) -> Option<String> {
    let mut buffer = [0u16; 512];
    // Device instance IDs are bounded by MAX_DEVICE_ID_LEN (200 characters).
    (unsafe {
        SetupDiGetDeviceInstanceIdW(
            set,
            info,
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            ptr::null_mut(),
        )
    } != 0)
        .then(|| wide(&buffer))
}

fn property(set: HDEVINFO, info: &SP_DEVINFO_DATA, key: u32) -> Vec<String> {
    let mut required = 0;
    let mut kind = 0;
    unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            set,
            info,
            key,
            &mut kind,
            ptr::null_mut(),
            0,
            &mut required,
        );
    }
    if required == 0 || required > 65536 {
        return vec![];
    }
    let mut buffer = vec![0u16; (required as usize).div_ceil(2)];
    if unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            set,
            info,
            key,
            &mut kind,
            buffer.as_mut_ptr().cast(),
            required,
            ptr::null_mut(),
        )
    } == 0
        || (kind != REG_SZ && kind != REG_MULTI_SZ)
    {
        return vec![];
    }
    buffer
        .split(|value| *value == 0)
        .filter(|value| !value.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

fn container_id(set: HDEVINFO, info: &SP_DEVINFO_DATA) -> Option<String> {
    let mut value = GUID::default();
    let mut kind = 0;
    if unsafe {
        SetupDiGetDevicePropertyW(
            set,
            info,
            &DEVPKEY_Device_ContainerId,
            &mut kind,
            (&mut value as *mut GUID).cast(),
            size_of::<GUID>() as u32,
            ptr::null_mut(),
            0,
        )
    } == 0
        || kind != DEVPROP_TYPE_GUID
    {
        return None;
    }
    Some(guid(&value))
}

fn parent_id(dev_inst: u32) -> Option<String> {
    let mut parent = 0;
    let mut buffer = [0u16; 512];
    if unsafe { CM_Get_Parent(&mut parent, dev_inst, 0) } != CR_SUCCESS
        || unsafe { CM_Get_Device_IDW(parent, buffer.as_mut_ptr(), buffer.len() as u32, 0) }
            != CR_SUCCESS
    {
        return None;
    }
    Some(wide(&buffer))
}

fn registry_string(
    set: HDEVINFO,
    info: &SP_DEVINFO_DATA,
    key_type: u32,
    name: &str,
) -> Option<String> {
    let key =
        unsafe { SetupDiOpenDevRegKey(set, info, DICS_FLAG_GLOBAL, 0, key_type, KEY_QUERY_VALUE) };
    if std::ptr::eq(key, INVALID_HANDLE_VALUE) {
        return None;
    }
    struct Key(HKEY);
    impl Drop for Key {
        fn drop(&mut self) {
            unsafe {
                RegCloseKey(self.0);
            }
        }
    }
    let key = Key(key);
    let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let mut buffer = [0u16; 512];
    let mut length = size_of_val(&buffer) as u32;
    let mut kind = 0;
    if unsafe {
        RegQueryValueExW(
            key.0,
            name.as_ptr(),
            ptr::null(),
            &mut kind,
            buffer.as_mut_ptr().cast(),
            &mut length,
        )
    } != ERROR_SUCCESS
        || kind != REG_SZ
    {
        return None;
    }
    Some(wide(&buffer))
}

fn nodes() -> Result<Vec<RawNode>, String> {
    let set = DeviceSet(unsafe {
        SetupDiGetClassDevsW(
            ptr::null(),
            ptr::null(),
            ptr::null_mut(),
            DIGCF_ALLCLASSES | DIGCF_PRESENT,
        )
    });
    if set.0 == INVALID_HANDLE_VALUE as HDEVINFO {
        return Err(os_error("SetupDiGetClassDevsW"));
    }
    let mut nodes = BTreeMap::new();
    for index in 0..16384 {
        let mut info = SP_DEVINFO_DATA {
            cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };
        if unsafe { SetupDiEnumDeviceInfo(set.0, index, &mut info) } == 0 {
            if unsafe { GetLastError() } == ERROR_NO_MORE_ITEMS {
                break;
            }
            return Err(os_error("SetupDiEnumDeviceInfo"));
        }
        let Some(instance_id) = instance_id(set.0, &info) else {
            continue;
        }; // May disappear during enumeration.
        let class_name = property(set.0, &info, SPDRP_CLASS)
            .into_iter()
            .next()
            .unwrap_or_default();
        let display_name = property(set.0, &info, SPDRP_FRIENDLYNAME)
            .into_iter()
            .next()
            .or_else(|| property(set.0, &info, SPDRP_DEVICEDESC).into_iter().next())
            .unwrap_or_else(|| "Unknown Windows device".into());
        let node = RawNode {
            instance_id: instance_id.clone(),
            parent_instance_id: parent_id(info.DevInst),
            container_id: container_id(set.0, &info),
            display_name,
            class_name: class_name.clone(),
            class_guid: guid(&info.ClassGuid),
            hardware_ids: property(set.0, &info, SPDRP_HARDWAREID),
            compatible_ids: property(set.0, &info, SPDRP_COMPATIBLEIDS),
            manufacturer: property(set.0, &info, SPDRP_MFG).into_iter().next(),
            interface_paths: vec![],
            port_name: if class_name.eq_ignore_ascii_case("Ports") {
                registry_string(set.0, &info, DIREG_DEV, "PortName")
            } else {
                None
            },
            adapter_id: if class_name.eq_ignore_ascii_case("Net") {
                registry_string(set.0, &info, DIREG_DRV, "NetCfgInstanceId")
            } else {
                None
            },
            interface_classes: vec![],
        };
        nodes.insert(instance_id.to_ascii_uppercase(), node);
        if index == 16383 {
            return Err("Windows device inventory exceeded its safety limit".into());
        }
    }
    for class in [&USB_INTERFACE, &SERIAL_INTERFACE, &NET_INTERFACE] {
        add_interfaces(class, &mut nodes)?;
    }
    Ok(nodes.into_values().collect())
}

fn add_interfaces(class: &GUID, nodes: &mut BTreeMap<String, RawNode>) -> Result<(), String> {
    let set = DeviceSet(unsafe {
        SetupDiGetClassDevsW(
            class,
            ptr::null(),
            ptr::null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    });
    if set.0 == INVALID_HANDLE_VALUE as HDEVINFO {
        return Err(os_error("SetupDiGetClassDevsW (interfaces)"));
    }
    for index in 0..16384 {
        let mut interface = SP_DEVICE_INTERFACE_DATA {
            cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };
        if unsafe { SetupDiEnumDeviceInterfaces(set.0, ptr::null(), class, index, &mut interface) }
            == 0
        {
            if unsafe { GetLastError() } == ERROR_NO_MORE_ITEMS {
                return Ok(());
            }
            return Err(os_error("SetupDiEnumDeviceInterfaces"));
        }
        let mut required = 0;
        unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                set.0,
                &interface,
                ptr::null_mut(),
                0,
                &mut required,
                ptr::null_mut(),
            );
        }
        if required == 0 || required as usize > MAX_BUFFER {
            continue;
        }
        // u64 backing storage supplies native structure alignment on Windows x64.
        let mut buffer = vec![0u64; (required as usize).div_ceil(8)];
        let detail = buffer
            .as_mut_ptr()
            .cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
        let mut info = SP_DEVINFO_DATA {
            cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };
        unsafe {
            (*detail).cbSize = if cfg!(target_pointer_width = "64") {
                8
            } else {
                6
            };
        }
        if unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                set.0,
                &interface,
                detail,
                required,
                ptr::null_mut(),
                &mut info,
            )
        } == 0
        {
            continue;
        }
        if let Some(id) = instance_id(set.0, &info) {
            if let Some(node) = nodes.get_mut(&id.to_ascii_uppercase()) {
                node.interface_classes.push(guid(class));
                node.interface_paths
                    .push(unsafe { wide_ptr((*detail).DevicePath.as_ptr()) });
            }
        }
    }
    Err("Windows interface inventory exceeded its safety limit".into())
}

unsafe fn socket_address(address: &SOCKET_ADDRESS) -> Option<String> {
    if address.lpSockaddr.is_null() {
        return None;
    }
    match (*address.lpSockaddr).sa_family {
        AF_INET if address.iSockaddrLength as usize >= size_of::<SOCKADDR_IN>() => {
            let socket = &*address.lpSockaddr.cast::<SOCKADDR_IN>();
            let ip = Ipv4Addr::from(socket.sin_addr.S_un.S_addr.to_ne_bytes());
            (!ip.is_unspecified()).then(|| ip.to_string())
        }
        AF_INET6 if address.iSockaddrLength as usize >= size_of::<SOCKADDR_IN6>() => {
            let socket = &*address.lpSockaddr.cast::<SOCKADDR_IN6>();
            let ip = Ipv6Addr::from(socket.sin6_addr.u.Byte);
            if ip.is_unspecified() {
                return None;
            }
            let scope = socket.Anonymous.sin6_scope_id;
            Some(if scope == 0 {
                ip.to_string()
            } else {
                format!("{ip}%{scope}")
            })
        }
        _ => None,
    }
}

fn adapters() -> Result<Vec<NetworkAdapter>, String> {
    let flags = GAA_FLAG_INCLUDE_GATEWAYS
        | GAA_FLAG_INCLUDE_ALL_INTERFACES
        | GAA_FLAG_SKIP_ANYCAST
        | GAA_FLAG_SKIP_MULTICAST;
    let mut required = 15000u32;
    for _ in 0..4 {
        if required as usize > MAX_BUFFER {
            return Err("Network inventory exceeded its safety limit".into());
        }
        let mut buffer = vec![0u64; (required as usize).div_ceil(8)];
        let head = buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        let result = unsafe {
            GetAdaptersAddresses(AF_UNSPEC as u32, flags, ptr::null(), head, &mut required)
        };
        if result == ERROR_BUFFER_OVERFLOW {
            continue;
        }
        if result == ERROR_NO_DATA {
            return Ok(vec![]);
        }
        if result != ERROR_SUCCESS {
            return Err(format!(
                "GetAdaptersAddresses failed (Windows error {result})"
            ));
        }
        let mut adapters = vec![];
        let mut current = head;
        // Windows guarantees linked pointers are valid while the owning buffer is alive.
        unsafe {
            while !current.is_null() {
                let adapter = &*current;
                if adapter.IfType != IF_TYPE_SOFTWARE_LOOPBACK {
                    let id = if adapter.AdapterName.is_null() {
                        String::new()
                    } else {
                        std::ffi::CStr::from_ptr(adapter.AdapterName.cast())
                            .to_string_lossy()
                            .into_owned()
                    };
                    let mut addresses = vec![];
                    let mut prefixes = vec![];
                    let mut unicast = adapter.FirstUnicastAddress;
                    while !unicast.is_null() {
                        if let Some(address) = socket_address(&(*unicast).Address) {
                            prefixes.push(AddressPrefix {
                                address: address.clone(),
                                length: (*unicast).OnLinkPrefixLength,
                            });
                            addresses.push(address);
                        }
                        unicast = (*unicast).Next;
                    }
                    let mut gateways = vec![];
                    let mut gateway = adapter.FirstGatewayAddress;
                    while !gateway.is_null() {
                        if let Some(address) = socket_address(&(*gateway).Address) {
                            gateways.push(address);
                        }
                        gateway = (*gateway).Next;
                    }
                    let mut dns_servers = vec![];
                    let mut dns = adapter.FirstDnsServerAddress;
                    while !dns.is_null() {
                        if let Some(address) = socket_address(&(*dns).Address) {
                            dns_servers.push(address);
                        }
                        dns = (*dns).Next;
                    }
                    let mut row = MIB_IF_ROW2 {
                        InterfaceLuid: adapter.Luid,
                        ..Default::default()
                    };
                    let virtual_interface = GetIfEntry2(&mut row) == ERROR_SUCCESS
                        && row.InterfaceAndOperStatusFlags._bitfield & 1 == 0;
                    addresses.sort();
                    addresses.dedup();
                    gateways.sort();
                    gateways.dedup();
                    adapters.push(NetworkAdapter {
                        id,
                        display_name: wide_ptr(adapter.FriendlyName),
                        description: wide_ptr(adapter.Description),
                        interface_index: adapter.Anonymous1.Anonymous.IfIndex,
                        interface_type: adapter.IfType,
                        link_up: adapter.OperStatus == IfOperStatusUp,
                        addresses,
                        gateways,
                        prefixes,
                        dns_servers,
                        virtual_interface,
                        luid: adapter.Luid.Value.to_string(),
                        ipv6_index: adapter.Ipv6IfIndex,
                        default_routes: vec![],
                    });
                }
                current = adapter.Next;
            }
        }
        add_routes(&mut adapters)?;
        return Ok(adapters);
    }
    Err("Network inventory changed repeatedly; refresh to try again".into())
}

// Route metrics are Windows route offsets, not interface+route preference scores.
fn add_routes(adapters: &mut [NetworkAdapter]) -> Result<(), String> {
    let mut table = ptr::null_mut();
    let result = unsafe { GetIpForwardTable2(AF_UNSPEC, &mut table) };
    if result != ERROR_SUCCESS {
        return Err(format!(
            "GetIpForwardTable2 failed (Windows error {result})"
        ));
    }
    struct Table(*mut MIB_IPFORWARD_TABLE2);
    impl Drop for Table {
        fn drop(&mut self) {
            unsafe {
                FreeMibTable(self.0.cast());
            }
        }
    }
    let table = Table(table);
    if table.0.is_null() {
        return Ok(());
    }
    unsafe {
        let count = (*table.0).NumEntries as usize;
        if count > 65536 {
            return Err("Routing table exceeded its safety limit".into());
        }
        for row in std::slice::from_raw_parts((*table.0).Table.as_ptr(), count) {
            if row.DestinationPrefix.PrefixLength != 0 {
                continue;
            }
            let Some(adapter) = adapters
                .iter_mut()
                .find(|a| a.luid == row.InterfaceLuid.Value.to_string())
            else {
                continue;
            };
            let family = row.NextHop.si_family;
            let socket = SOCKET_ADDRESS {
                lpSockaddr: (&row.NextHop as *const SOCKADDR_INET).cast_mut().cast(),
                iSockaddrLength: if family == AF_INET {
                    size_of::<SOCKADDR_IN>()
                } else {
                    size_of::<SOCKADDR_IN6>()
                } as i32,
            };
            if let Some(gateway) = socket_address(&socket) {
                adapter.default_routes.push(DefaultRoute {
                    gateway,
                    metric: row.Metric,
                    family: if family == AF_INET { "ipv4" } else { "ipv6" }.into(),
                });
            }
        }
    }
    Ok(())
}

pub fn capture() -> Result<Inventory, String> {
    Ok(Inventory {
        nodes: nodes()?,
        adapters: adapters()?,
    })
}

pub struct Notifications {
    pnp: HCMNOTIFICATION,
    network: HANDLE,
    context: Option<Box<SyncSender<Signal>>>,
    pub warnings: Vec<String>,
}

impl Notifications {
    pub fn register(sender: SyncSender<Signal>) -> Self {
        let context = Box::new(sender);
        let filter = CM_NOTIFY_FILTER {
            cbSize: size_of::<CM_NOTIFY_FILTER>() as u32,
            Flags: CM_NOTIFY_FILTER_FLAG_ALL_INTERFACE_CLASSES,
            FilterType: CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE,
            ..Default::default()
        };
        let mut watch = Self {
            pnp: ptr::null_mut(),
            network: ptr::null_mut(),
            context: Some(context),
            warnings: vec![],
        };
        let address = &**watch.context.as_ref().expect("callback context exists")
            as *const SyncSender<Signal> as *const c_void;
        let result = unsafe {
            CM_Register_Notification(&filter, address, Some(pnp_changed), &mut watch.pnp)
        };
        if result != CR_SUCCESS {
            watch.pnp = ptr::null_mut();
            watch.warnings.push(
                "Live USB/serial notifications unavailable; inventory refreshes every 30 seconds."
                    .into(),
            );
        }
        let result = unsafe {
            NotifyIpInterfaceChange(
                AF_UNSPEC,
                Some(network_changed),
                address,
                false,
                &mut watch.network,
            )
        };
        if result != ERROR_SUCCESS {
            watch.network = ptr::null_mut();
            watch.warnings.push(
                "Live network notifications unavailable; inventory refreshes every 30 seconds."
                    .into(),
            );
        }
        watch
    }

    pub fn watching(&self) -> bool {
        !self.pnp.is_null() && !self.network.is_null()
    }
}

impl Drop for Notifications {
    fn drop(&mut self) {
        // Both APIs wait for outstanding callbacks. Never unregister inside a callback.
        let pnp_safe =
            self.pnp.is_null() || unsafe { CM_Unregister_Notification(self.pnp) } == CR_SUCCESS;
        let network_safe = self.network.is_null()
            || unsafe { CancelMibChangeNotify2(self.network) } == ERROR_SUCCESS;
        if !pnp_safe || !network_safe {
            // A rare unregister failure must not leave a dangling callback context.
            if let Some(context) = self.context.take() {
                Box::leak(context);
            }
            tracing::error!(
                "Discovery callback unregister failed; context retained for process lifetime"
            );
        }
    }
}

unsafe extern "system" fn pnp_changed(
    _handle: HCMNOTIFICATION,
    context: *const c_void,
    _action: CM_NOTIFY_ACTION,
    _data: *const CM_NOTIFY_EVENT_DATA,
    _size: u32,
) -> u32 {
    if !context.is_null() {
        let _ = (&*context.cast::<SyncSender<Signal>>()).try_send(Signal::Changed);
    }
    ERROR_SUCCESS
}

unsafe extern "system" fn network_changed(
    context: *const c_void,
    _row: *const MIB_IPINTERFACE_ROW,
    _kind: MIB_NOTIFICATION_TYPE,
) {
    if !context.is_null() {
        let _ = (&*context.cast::<SyncSender<Signal>>()).try_send(Signal::Changed);
    }
}
