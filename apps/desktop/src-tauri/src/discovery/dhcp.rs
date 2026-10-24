//! Bounded parser of the owned GetAdaptersAddresses buffer; never guesses another field.
use std::{
    mem::{offset_of, size_of},
    ptr,
};
use windows_sys::Win32::{NetworkManagement::IpHelper::*, Networking::WinSock::*};
pub(super) fn server(
    adapter: *const IP_ADAPTER_ADDRESSES_LH,
    buffer: &[u64],
    used: usize,
) -> Option<String> {
    let capacity = std::mem::size_of_val(buffer);
    if used > capacity {
        return None;
    }
    let base = buffer.as_ptr() as usize;
    let offset = (adapter as usize).checked_sub(base)?;
    // Only create a slice over owned initialized memory. Every native field is copied
    // unaligned after range checking, including both levels of pointer indirection.
    let bytes = unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), used) };
    fn read<T: Copy>(bytes: &[u8], offset: usize) -> Option<T> {
        let field = bytes.get(offset..offset.checked_add(size_of::<T>())?)?;
        Some(unsafe { ptr::read_unaligned(field.as_ptr().cast::<T>()) })
    }
    let length = read::<u32>(bytes, offset)? as usize;
    let needed = offset_of!(IP_ADAPTER_ADDRESSES_LH, Dhcpv4Server) + size_of::<SOCKET_ADDRESS>();
    if length < needed || offset.checked_add(length)? > used {
        return None;
    }
    let flags = read::<u32>(
        bytes,
        offset.checked_add(offset_of!(IP_ADAPTER_ADDRESSES_LH, Anonymous2))?,
    )?;
    if flags & IP_ADAPTER_DHCP_ENABLED == 0 {
        return None;
    }
    let socket = read::<SOCKET_ADDRESS>(
        bytes,
        offset.checked_add(offset_of!(IP_ADAPTER_ADDRESSES_LH, Dhcpv4Server))?,
    )?;
    if socket.lpSockaddr.is_null() || socket.iSockaddrLength < size_of::<SOCKADDR_IN>() as i32 {
        return None;
    }
    let address_offset = (socket.lpSockaddr as usize).checked_sub(base)?;
    if address_offset.checked_add(socket.iSockaddrLength as usize)? > used {
        return None;
    }
    let address = read::<SOCKADDR_IN>(bytes, address_offset)?;
    if address.sin_family != AF_INET {
        return None;
    }
    let ip = std::net::Ipv4Addr::from(unsafe { address.sin_addr.S_un.S_addr }.to_ne_bytes());
    if ip.is_unspecified() || ip.is_multicast() || ip.is_broadcast() || ip.is_loopback() {
        return None;
    }
    Some(ip.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stage4_dhcp_native_bounds_flags_and_address_validation() {
        let size = size_of::<IP_ADAPTER_ADDRESSES_LH>();
        let mut buffer = vec![0u64; (size + size_of::<SOCKADDR_IN>()).div_ceil(8)];
        let used = buffer.len() * 8;
        let adapter = buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        let addr = unsafe {
            buffer
                .as_mut_ptr()
                .cast::<u8>()
                .add(size)
                .cast::<SOCKADDR_IN>()
        };
        unsafe {
            (*adapter).Anonymous1.Anonymous.Length = size as u32;
            (*adapter).Anonymous2.Flags = IP_ADAPTER_DHCP_ENABLED;
            (*adapter).Dhcpv4Server = SOCKET_ADDRESS {
                lpSockaddr: addr.cast(),
                iSockaddrLength: size_of::<SOCKADDR_IN>() as i32,
            };
            (*addr).sin_family = AF_INET;
            (*addr).sin_addr.S_un.S_addr = u32::from_ne_bytes([192, 168, 100, 1]);
        }
        assert_eq!(
            server(adapter, &buffer, used).as_deref(),
            Some("192.168.100.1")
        );
        for fault in 0..10 {
            unsafe {
                (*adapter).Anonymous1.Anonymous.Length = size as u32;
                (*adapter).Anonymous2.Flags = IP_ADAPTER_DHCP_ENABLED;
                (*adapter).Dhcpv4Server = SOCKET_ADDRESS {
                    lpSockaddr: addr.cast(),
                    iSockaddrLength: size_of::<SOCKADDR_IN>() as i32,
                };
                (*addr).sin_family = AF_INET;
                (*addr).sin_addr.S_un.S_addr = u32::from_ne_bytes([192, 168, 100, 1]);
                match fault {
                    0 => {
                        (*adapter).Anonymous1.Anonymous.Length =
                            offset_of!(IP_ADAPTER_ADDRESSES_LH, Dhcpv4Server) as u32
                    }
                    1 => (*adapter).Anonymous2.Flags = 0,
                    2 => (*adapter).Dhcpv4Server.lpSockaddr = ptr::null_mut(),
                    3 => (*adapter).Dhcpv4Server.iSockaddrLength = 1,
                    4 => (*adapter).Dhcpv4Server.iSockaddrLength = -1,
                    5 => (*addr).sin_family = AF_INET6,
                    6 => (*addr).sin_addr.S_un.S_addr = 0,
                    7 => (*addr).sin_addr.S_un.S_addr = u32::from_ne_bytes([224, 0, 0, 1]),
                    8 => (*adapter).Dhcpv4Server.lpSockaddr = usize::MAX as _,
                    _ => (*adapter).Anonymous1.Anonymous.Length = u32::MAX,
                }
            }
            assert!(server(adapter, &buffer, used).is_none(), "fault {fault}");
        }
        assert!(server(adapter, &buffer, 2).is_none());
        assert!(server(ptr::null(), &buffer, used).is_none());
    }
}
