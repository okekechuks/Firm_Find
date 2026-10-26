"""Security regressions use synthetic data only. Live firmware/build evidence is separate."""
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent


def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / filename)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


host = module("dns_host", "build-openwrt-dns.py")
worker = module("dns_worker", "openwrt-dns-worker.py") if sys.platform == "linux" else None


class DNSInputs(unittest.TestCase):
    def test_dns_rejects_shell_injection_urls_names_scopes_duplicates_and_non_global_ips(self):
        for values in [[], ["1.1.1.1", "8.8.8.8", "9.9.9.9"], ["1.1.1.1;reboot"],
                       ["1.1.1.1' || echo unsafe"], ["https://1.1.1.1"], ["router.local"],
                       ["fe80::1%eth0"], ["127.0.0.1"], ["192.168.1.1"], ["224.0.0.1"],
                       ["::"], ["1.1.1.1", "1.1.1.1"]]:
            with self.subTest(values=values), self.assertRaises(ValueError):
                host.dns_servers(values)

    def test_only_fixed_upstream_configuration_is_emitted(self):
        script = host.defaults_script(["1.1.1.1", "2001:4860:4860::8888"])
        self.assertIn("network.wan.peerdns='0'", script)
        self.assertNotIn("dhcp.lan", script)
        self.assertNotIn("sysupgrade", script)
        self.assertNotIn("mtd", script)

    def test_file_validation_is_bounded_and_input_is_not_modified(self):
        with tempfile.TemporaryDirectory() as directory:
            file = Path(directory) / "synthetic.bin"
            raw = b"harmless synthetic validation input"
            file.write_bytes(raw)
            with self.assertRaises(ValueError):
                host.digest(file, 1)
            self.assertEqual(host.digest(file, 1024), host.hashlib.sha256(raw).hexdigest())
            self.assertEqual(file.read_bytes(), raw)

    def test_windows_paths_reject_network_relative_traversal_and_alternate_streams(self):
        for path in [r"\\server\share\image.bin", r"C:image.bin", r"C:\safe\..\image.bin",
                     r"C:\safe\image.bin:payload", r"\\?\C:\safe\image.bin", "relative.bin"]:
            with self.subTest(path=path), self.assertRaises(ValueError):
                host.windows_local_path(path)
        self.assertEqual(str(host.windows_local_path(r"E:\safe\image.bin")), r"E:\safe\image.bin")


@unittest.skipUnless(worker is not None, "Linux sandbox/worker qualification runs separately in WSL")
class LinuxWorker(unittest.TestCase):
    def test_real_offline_single_file_readback_with_synthetic_squashfs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            payload = host.defaults_script(["1.1.1.1"]).encode()
            defaults = root / "source/etc/uci-defaults/99-obtainer-dns"
            defaults.parent.mkdir(parents=True)
            defaults.write_bytes(payload)
            filesystem = root / "synthetic.squashfs"
            subprocess.run(["mksquashfs", str(root / "source"), str(filesystem), "-noappend",
                            "-processors", "1", "-mem", "64M"], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, timeout=20)
            image = root / "synthetic.bin"
            image.write_bytes(b"synthetic harmless prefix" * 4 + filesystem.read_bytes())
            self.assertEqual(worker.readback(image, len(b"synthetic harmless prefix") * 4), payload)

    def test_compressor_limits_preserve_build_options_and_require_unique_invocation(self):
        original = "prefix\n\t-comp $(SQUASHFSCOMP) $(SQUASHFSOPT)\nsuffix\n"
        changed = worker.bound_compressor(original)
        self.assertIn("$(SQUASHFSCOMP) $(SQUASHFSOPT) -processors 1 -mem 256M", changed)
        self.assertTrue(changed.startswith("prefix\n"))
        self.assertTrue(changed.endswith("suffix\n"))
        for value in ["", original + original, changed]:
            with self.assertRaises(ValueError):
                worker.bound_compressor(value)

    def test_builder_ownership_is_mapped_inside_namespace_without_host_privilege(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            probe = """import os,json
p='/work/owned'
open(p,'w').close()
os.chown(p,0,0)
status=open('/proc/self/status').read().splitlines()
print(json.dumps({'uid':os.getuid(),'caps':next(x for x in status if x.startswith('CapEff:')).split()[1]}))
"""
            result = json.loads(subprocess.check_output(worker.sandbox(work=root) + ["python3", "-c", probe], text=True, timeout=15))
            self.assertEqual(result["uid"], 0)
            self.assertEqual(int(result["caps"], 16), 0)
            self.assertNotEqual(worker.os.getuid(), 0)
            self.assertEqual((root / "owned").stat().st_uid, worker.os.getuid())

    def test_cancelled_process_is_reaped_and_next_owned_operation_works(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            cancel=root / "cancel.requested"
            cancel.write_text("synthetic cancellation")
            with self.assertRaises(InterruptedError):
                worker.run_logged([sys.executable,"-c","import time; time.sleep(60)"],root/"cancelled.log",10,cancel)
            cancel.unlink()
            worker.run_logged([sys.executable,"-c","print('computed child exit')"],root/"next.log",10,cancel)
            self.assertIn("computed child exit",(root/"next.log").read_text())

    def test_timed_out_process_is_reaped_and_next_owned_operation_works(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            children = Path(f"/proc/self/task/{worker.os.getpid()}/children")
            before = children.read_text()
            with self.assertRaises(TimeoutError):
                worker.run_logged([sys.executable, "-c", "import time; time.sleep(60)"], root / "timeout.log", 0)
            self.assertEqual(children.read_text(), before)
            worker.run_logged([sys.executable, "-c", "print(42)"], root / "next.log", 10)
            self.assertEqual(children.read_text(), before)
    def test_overlay_contract_rejects_added_commands_and_noncanonical_data(self):
        raw = host.defaults_script(["1.1.1.1", "9.9.9.9"]).encode()
        self.assertEqual(worker.validate_overlay(raw), ["1.1.1.1", "9.9.9.9"])
        for changed in [raw + b"reboot\n", raw.replace(b"1.1.1.1", b"1.1.1.1;reboot"), raw.replace(b"network.wan", b"network.lan")]:
            with self.assertRaises(ValueError):
                worker.validate_overlay(changed)

    def test_archive_path_traversal_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tar = root / "synthetic.tar"
            with tarfile.open(tar, "w") as archive:
                member = tarfile.TarInfo("../outside-sentinel")
                member.size = 1
                archive.addfile(member, io.BytesIO(b"x"))
            compressed = root / "synthetic.tar.zst"
            with compressed.open("wb") as output:
                subprocess.run(["zstd", "--stdout", str(tar)], stdout=output, stderr=subprocess.DEVNULL, check=True)
            destination = root / "extract"
            destination.mkdir()
            with self.assertRaises(tarfile.TarError):
                worker.extract_tools(compressed, destination)
            self.assertFalse((root / "outside-sentinel").exists())

    def test_only_exact_pinned_sdk_absolute_links_are_omitted(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for target, rejected in [("/usr/bin/file", False), ("/etc/shadow", True)]:
                tar = root / "synthetic.tar"
                with tarfile.open(tar, "w") as archive:
                    member = tarfile.TarInfo("openwrt-imagebuilder-25.12.6-ath79-generic.Linux-x86_64/staging_dir/host/bin/file")
                    member.type = tarfile.SYMTYPE
                    member.linkname = target
                    archive.addfile(member)
                compressed = root / "synthetic.tar.zst"
                with compressed.open("wb") as output:
                    subprocess.run(["zstd", "--stdout", str(tar)], stdout=output, stderr=subprocess.DEVNULL, check=True)
                destination = root / ("rejected" if rejected else "omitted")
                destination.mkdir()
                if rejected:
                    with self.assertRaises(ValueError):
                        worker.extract_tools(compressed, destination)
                else:
                    self.assertEqual(worker.extract_tools(compressed, destination), (1, 0))
                self.assertEqual(list(destination.rglob("*")), [])

    def test_readback_sandbox_has_no_host_drives_readonly_input_and_distinct_network(self):
        with tempfile.TemporaryDirectory() as directory:
            file = Path(directory) / "synthetic.bin"
            file.write_bytes(b"synthetic sandbox sentinel")
            probe = """import os,json,resource
try:
    os.open('/input.bin',os.O_WRONLY)
    writable=True
except OSError:
    writable=False
print(json.dumps({'network':os.readlink('/proc/self/ns/net'),'windowsDrives':os.path.exists('/mnt/c'),'inputWritable':writable,'memoryLimit':resource.getrlimit(resource.RLIMIT_AS)[0]}))
"""
            command = worker.sandbox(image=file) + ["prlimit", "--as=536870912", "--cpu=10", "--", "python3", "-c", probe]
            result = json.loads(subprocess.check_output(command, text=True, timeout=15))
            self.assertNotEqual(result["network"], worker.os.readlink("/proc/self/ns/net"))
            self.assertFalse(result["windowsDrives"])
            self.assertFalse(result["inputWritable"])
            self.assertEqual(result["memoryLimit"], 536870912)


if __name__ == "__main__":
    unittest.main(verbosity=2)
