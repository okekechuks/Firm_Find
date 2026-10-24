"""Independent canonical binding vectors. Prints computed values only; no I/O or verdicts."""
import hashlib
import ipaddress
import struct


def field(value):
    return struct.pack(">I", len(value)) + value


def address(value, scope=0):
    ip = ipaddress.ip_address(value)
    return bytes([ip.version]) + ip.packed + struct.pack(">I", scope)


def vector(source, scope, endpoint, gateway, prefix, dhcp=None):
    data = (
        field(b"OBTAINER/plain-http-binding/v1") + field(b"synthetic-device")
        + field(b"epoch") + struct.pack(">Q", 7)
        + field(bytes.fromhex("00112233445566778899aabbccddeeff"))
        + struct.pack(">Q", 1234) + field(endpoint.encode("ascii"))
        + field(address(source, scope)) + bytes([prefix]) + b"\x00"
        + struct.pack(">I", 1) + field(address(gateway, scope))
        + struct.pack(">I", 0)
        + (b"\x00" if dhcp is None else b"\x01" + field(address(dhcp)))
        + struct.pack(">I", 1) + field(b"\x00")
    )
    return {"length": len(data), "sha256": hashlib.sha256(data).hexdigest(), "hex": data.hex()}


if __name__ == "__main__":
    import json
    print(json.dumps({
        "ipv4_unknown_dhcp": vector("192.168.100.2", 0, "http://192.168.100.1/", "192.168.100.1", 24),
        "ipv4_known_dhcp": vector("192.168.100.2", 0, "http://192.168.100.1/", "192.168.100.1", 24, "192.168.100.1"),
        "ipv6_scope_7": vector("fe80::2", 7, "http://[fe80::1]/", "fe80::1", 64),
    }, indent=2))
