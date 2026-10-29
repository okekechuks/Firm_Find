//! Local Windows adapter and route queries. No sockets and no routing changes.
use super::{BindingFailure, RouteFacts};
use crate::discovery::NetworkAdapter;
use std::{ffi::CStr, mem::size_of, net::Ipv4Addr, ptr};
use windows_sys::Win32::{
    Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS},
    NetworkManagement::{IpHelper::*, Ndis::IfOperStatusUp},
    Networking::WinSock::*,
};

pub(super) fn selected_route(
    network: &NetworkAdapter,
    source: Ipv4Addr,
    destination: Ipv4Addr,
) -> Result<RouteFacts, BindingFailure> {
    let expected_luid = network
        .luid
        .parse::<u64>()
        .map_err(|_| BindingFailure::InvalidInterface)?;
    if expected_luid == 0 || network.interface_index == 0 {
        return Err(BindingFailure::InvalidInterface);
    }
    // Re-resolve the actual Windows GUID/LUID/index association rather than trusting
    // a cached friendly name or caller-provided source address.
    verify_adapter(network, source, expected_luid)?;
    let mut src = SOCKADDR_INET::default();
    src.Ipv4.sin_family = AF_INET;
    src.Ipv4.sin_addr.S_un.S_addr = u32::from_ne_bytes(source.octets());
    let mut dst = SOCKADDR_INET::default();
    dst.Ipv4.sin_family = AF_INET;
    dst.Ipv4.sin_addr.S_un.S_addr = u32::from_ne_bytes(destination.octets());
    let mut best = MIB_IPFORWARD_ROW2::default();
    let mut actual_source = SOCKADDR_INET::default();
    let code = unsafe {
        GetBestRoute2(
            ptr::null(),
            network.interface_index,
            &src,
            &dst,
            0,
            &mut best,
            &mut actual_source,
        )
    };
    if code != ERROR_SUCCESS
        || best.InterfaceIndex != network.interface_index
        || unsafe { best.InterfaceLuid.Value } != expected_luid
        || unsafe { actual_source.si_family } != AF_INET
        || unsafe { Ipv4Addr::from(actual_source.Ipv4.sin_addr.S_un.S_addr.to_ne_bytes()) }
            != source
        || unsafe { best.NextHop.si_family } != AF_INET
    {
        return Err(BindingFailure::RouteUnproven);
    }
    Ok(RouteFacts {
        source,
        index: best.InterfaceIndex,
        luid: expected_luid,
        next_hop: unsafe { Ipv4Addr::from(best.NextHop.Ipv4.sin_addr.S_un.S_addr.to_ne_bytes()) },
    })
}

fn normalized_guid(value: &str) -> String {
    value.trim_matches(['{', '}']).to_ascii_uppercase()
}

fn verify_adapter(
    network: &NetworkAdapter,
    source: Ipv4Addr,
    luid: u64,
) -> Result<(), BindingFailure> {
    let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
    let mut required = 15_000u32;
    for _ in 0..3 {
        if required > 1_048_576 {
            return Err(BindingFailure::RouteUnproven);
        }
        let mut buffer = vec![0u64; (required as usize).div_ceil(size_of::<u64>())];
        let head = buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        let status = unsafe {
            GetAdaptersAddresses(AF_INET as u32, flags, ptr::null(), head, &mut required)
        };
        if status == ERROR_BUFFER_OVERFLOW {
            continue;
        }
        if status != ERROR_SUCCESS {
            return Err(BindingFailure::RouteUnproven);
        }
        let mut cursor = head;
        for _ in 0..4096 {
            if cursor.is_null() {
                break;
            }
            // Windows guarantees all linked rows belong to the owning buffer.
            let row = unsafe { &*cursor };
            let name = if row.AdapterName.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(row.AdapterName.cast()) }
                    .to_string_lossy()
                    .into_owned()
            };
            if normalized_guid(&name) == normalized_guid(&network.id) {
                if row.OperStatus != IfOperStatusUp
                    || unsafe { row.Luid.Value } != luid
                    || unsafe { row.Anonymous1.Anonymous.IfIndex } != network.interface_index
                {
                    return Err(BindingFailure::DeviceChanged);
                }
                let mut address = row.FirstUnicastAddress;
                for _ in 0..4096 {
                    if address.is_null() {
                        break;
                    }
                    let unicast = unsafe { &*address };
                    if !unicast.Address.lpSockaddr.is_null()
                        && unicast.Address.iSockaddrLength as usize >= size_of::<SOCKADDR_IN>()
                    {
                        let socket = unsafe { &*unicast.Address.lpSockaddr.cast::<SOCKADDR_IN>() };
                        if socket.sin_family == AF_INET
                            && unsafe { Ipv4Addr::from(socket.sin_addr.S_un.S_addr.to_ne_bytes()) }
                                == source
                            && network.prefixes.iter().any(|p| {
                                p.address == source.to_string()
                                    && p.length == unicast.OnLinkPrefixLength
                            })
                        {
                            return Ok(());
                        }
                    }
                    address = unicast.Next;
                }
                return Err(BindingFailure::DeviceChanged);
            }
            cursor = row.Next;
        }
        return Err(BindingFailure::DeviceChanged);
    }
    Err(BindingFailure::RouteUnproven)
}
