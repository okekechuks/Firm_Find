"""Disposable OpenWrt OS interoperability fixture. Run under Ubuntu WSL.

Only this guest receives a generated test key and temporary network setup.
No physical device, normal OBTAINER storage, bridge or host network config is changed.
"""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / ".tools" / "phase7-vm"
EXPECTED = "f8538784d0288b4133a99ac2a68443e4922ffde6343e80df56a5882364b9d499"


def receive(sock, marker, timeout=60):
    end = time.monotonic() + timeout
    data = bytearray()
    while time.monotonic() < end:
        try:
            chunk = sock.recv(8192)
        except TimeoutError:
            continue
        if not chunk:
            raise RuntimeError("Guest serial console closed")
        data.extend(chunk)
        if len(data) > 256 * 1024:
            raise RuntimeError("Guest console output limit")
        if marker in data:
            return data.decode("utf-8", "replace")
    raise RuntimeError("Guest console deadline exceeded")


def stop():
    state = json.loads((WORK / "state.json").read_text())
    pid = state["pid"]
    command = Path(f"/proc/{pid}/cmdline")
    if command.exists():
        arguments = command.read_bytes().split(b"\0")
        if str(WORK / "openwrt.img").encode() not in b" ".join(arguments):
            raise RuntimeError("PID ownership mismatch; refusing to stop")
        if state["socket"].encode() not in b" ".join(arguments):
            raise RuntimeError("Socket ownership mismatch; refusing to stop")
        os.kill(pid, signal.SIGTERM)
    print("Owned VM stop requested")


def start(resume=False):
    WORK.mkdir(exist_ok=True)
    archive = WORK / "openwrt.img.gz"
    if hashlib.sha256(archive.read_bytes()).hexdigest() != EXPECTED:
        raise RuntimeError("Official image checksum mismatch")
    disk = WORK / "openwrt.img"
    if not resume and (disk.exists() or (WORK / "state.json").exists()):
        raise RuntimeError("Existing VM files retained; inspect and stop before reusing")
    if resume:
        state = json.loads((WORK / "state.json").read_text())
        prior = Path(f'/proc/{int(state["pid"])}/cmdline')
        if prior.exists() and b"qemu-system" in prior.read_bytes():
            raise RuntimeError("A VM process already exists; refusing duplicate launch")
        with gzip.open(archive, "rb") as source:
            if hashlib.sha256(source.read()).digest() != hashlib.sha256(disk.read_bytes()).digest():
                raise RuntimeError("Existing base disk differs from verified immutable image")
        (WORK / f'state-before-resume-{time.time_ns()}.json').write_text(json.dumps(state, indent=2))
    else:
        with gzip.open(archive, "rb") as source, disk.open("xb") as output:
            output.write(source.read(256 * 1024 * 1024))
    key = WORK / "test-key"
    if not resume:
        subprocess.run(["ssh-keygen", "-t", "ed25519", "-N", "", "-C",
                        "obtainer-disposable-vm-only", "-f", str(key)], check=True,
                       stdout=subprocess.DEVNULL)
    serial = str(Path(tempfile.mkdtemp(prefix="obtainer-phase7-")) / "serial.sock")
    log = (WORK / (f'qemu-resume-{time.time_ns()}.log' if resume else 'qemu.log')).open("xb")
    process = subprocess.Popen([
        "qemu-system-x86_64", "-accel", "tcg", "-m", "256", "-smp", "2",
        "-drive", f"file={disk},format=raw", "-snapshot",
        "-display", "none", "-monitor", "none",
        "-serial", f"unix:{serial},server=on,wait=off",
        "-netdev", "user,id=net0,restrict=on,net=192.168.77.0/24,hostfwd=tcp:127.0.0.1:22270-192.168.77.10:22",
        "-device", "e1000,netdev=net0",
    ], stdout=log, stderr=log, stdin=subprocess.DEVNULL, start_new_session=True)
    state = {"pid": process.pid, "socket": serial, "port": 22270,
             "imageSha256": EXPECTED, "hardware": False, "target": "25.12.6 x86/64"}
    (WORK / "state.json").write_text(json.dumps(state, indent=2))
    try:
        end = time.monotonic() + 15
        while not Path(serial).exists():
            if process.poll() is not None or time.monotonic() > end:
                raise RuntimeError("QEMU did not create its owned serial socket")
            time.sleep(0.1)
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            client.settimeout(0.5)
            client.connect(serial)
            boot = receive(client, b"Please press Enter to activate this console", 90)
            client.sendall(b"\n")
            boot += receive(client, b"root@")
            (WORK / "boot.log").write_text(boot)
            public = (WORK / "test-key.pub").read_text().strip()
            # Generated public key only; this modifies only the disposable QEMU overlay.
            command = (
                "for i in $(seq 1 60); do ip link show br-lan >/dev/null 2>&1 && break; sleep 1; done; "
                "mkdir -p /etc/dropbear; "
                f"printf '%s\\n' '{public}' > /etc/dropbear/authorized_keys; "
                "chmod 600 /etc/dropbear/authorized_keys; "
                "ip addr add 192.168.77.10/24 dev br-lan; "
                "/etc/init.d/dropbear restart; "
                "echo OBTAINER_SETUP_COMPLETE\n"
            )
            client.sendall(command.encode())
            setup = receive(client, b"\r\nOBTAINER_SETUP_COMPLETE")
            (WORK / "setup.log").write_text(setup)
            client.sendall(b"ubus call system board; dropbearkey -y -f /etc/dropbear/dropbear_ed25519_host_key; echo OBTAINER_IDENTITY_COMPLETE\n")
            identity = receive(client, b"\r\nOBTAINER_IDENTITY_COMPLETE")
            (WORK / "console-identity.log").write_text(identity)
        print(json.dumps(state))
        return process
    except BaseException:
        process.terminate()
        process.wait(timeout=10)
        raise


def configure():
    """Retry guest-only network setup after early boot; retain the original failed log."""
    state = json.loads((WORK / "state.json").read_text())
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(0.5)
        client.connect(state["socket"])
        client.sendall(b"\nip link show; ip addr add 192.168.77.10/24 dev br-lan; /etc/init.d/dropbear restart; echo OBTAINER_RETRY_COMPLETE\n")
        (WORK / "setup-retry.log").write_text(receive(client, b"\r\nOBTAINER_RETRY_COMPLETE"))
    print("Owned guest network setup retried")


def serve():
    """Keep an owned foreground WSL process alive until its QEMU child is stopped."""
    process = start(resume=(WORK / "state.json").exists())
    process.wait()
    print("Owned OpenWrt VM exited")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("operation", choices=["start", "stop", "configure", "serve"])
    options = parser.parse_args()
    {"start": start, "stop": stop, "configure": configure, "serve": serve}[options.operation]()
