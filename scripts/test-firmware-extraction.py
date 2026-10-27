"""Synthetic extraction regressions. Real firmware qualification is separate."""
import gzip
import errno
import binascii
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import struct
import stat
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("worker", Path(__file__).with_name("firmware-extract-worker.py"))
worker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(worker)


def synthetic_superblock(**changes):
    # Explicit synthetic superblock; never a real firmware fixture.
    values = [0x73717368, 5, 0, 131072, 0, 4, 17, 0, 1, 4, 0, 0, 1024, 900, 0xffffffffffffffff, 100, 500, 0xffffffffffffffff, 0xffffffffffffffff]
    for index, value in changes.items():
        values[int(index)] = value
    return struct.pack("<5I6H8Q", *values) + bytes(1024 - 96)


def synthetic_trx(offsets=(28,60,0),version=1):
    raw=bytearray(struct.pack("<7I",0x30524448,60+1024,0,version<<16,*offsets)+bytes(32)+synthetic_superblock())
    struct.pack_into("<I",raw,8,binascii.crc32(raw[12:]) ^ 0xffffffff)
    return bytes(raw)


class Parsing(unittest.TestCase):
    def test_trx_requires_bounded_header_crc_version_and_ordered_partitions(self):
        with tempfile.TemporaryDirectory() as tmp:
            image=Path(tmp)/"synthetic.trx"
            image.write_bytes(synthetic_trx())
            self.assertEqual(worker.trx_header(image)["partitionOffsets"],[28,60])
            for raw in [synthetic_trx(offsets=(28,20,0)),synthetic_trx(offsets=(28,0,60)),synthetic_trx(version=2),synthetic_trx()[:-1],synthetic_trx()[:-1]+b"x"]:
                image.write_bytes(raw)
                with self.assertRaises(worker.Refusal):worker.trx_header(image)

    def test_gzip_cannot_bypass_trx_validation(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)
            (p/"image.gz").write_bytes(gzip.compress(synthetic_trx()))
            original_path=worker.Path
            with patch.object(worker,"Path",side_effect=lambda value:p/"expanded.bin" if value=="/work/expanded.bin" else original_path(value)),self.assertRaisesRegex(worker.Refusal,"unsupported_nested_format"):
                worker.GzipSquashfsBackend.prepare(p/"image.gz")

    def region(self, raw):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / "synthetic.bin"
            p.write_bytes(raw)
            return worker.squashfs_region(p)

    def test_valid_offset_is_discovered_not_hardcoded(self):
        self.assertEqual(self.region(b"synthetic-prefix" + synthetic_superblock())["offset"], 16)

    def test_false_magic_and_truncated_superblock(self):
        for raw in [b"hsqs", b"hsqs" + bytes(100), synthetic_superblock()[:100]]:
            with self.subTest(length=len(raw)), self.assertRaises(worker.Refusal):
                self.region(raw)

    def test_invalid_superblock_sizes_tables_and_compression(self):
        for changes in [{"12": 2000}, {"12": 95}, {"3": 4097}, {"6": 14}, {"9": 3}, {"1": 20001}, {"13": 2000}, {"5": 100}]:
            with self.subTest(changes=changes), self.assertRaises(worker.Refusal):
                self.region(synthetic_superblock(**changes))

    def test_ambiguous_and_opaque_input_not_success(self):
        for raw in [synthetic_superblock() * 2, b"opaque encrypted-like synthetic bytes"]:
            with self.assertRaises(worker.Refusal):
                self.region(raw)

    def test_path_escape_and_windows_aliases(self):
        for name in ["../bad", "/etc/passwd", "a//b", "a\\b", "a:stream", "NUL", "con.txt", "a\nb", "a.", "/".join(["x"]*33)]:
            with self.subTest(name=name), self.assertRaises(worker.Refusal):
                worker.safe_name(name)
        self.assertEqual(worker.safe_name("etc/config/network"), "etc/config/network")

    def test_empty_success_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp, self.assertRaises(worker.Refusal):
            worker.collect_tree(Path(tmp))

    def test_combined_budget_reserves_decoder_overhead_and_rounds_entries(self):
        worker.reserve_space(120 * 1024**2, 96 * 1024**2, 4)
        with self.assertRaisesRegex(worker.Refusal, "resource_exhausted"):
            worker.reserve_space(128 * 1024**2, 128 * 1024**2, 4)
        with self.assertRaisesRegex(worker.Refusal, "resource_exhausted"):
            worker.reserve_space(128 * 1024**2, 64 * 1024**2, 20000)

    def test_unicode_normalization_collisions_and_bidi_names_are_refused(self):
        for name in ["etc/abc\u202etxt", "etc/\u2066name", "etc/\u200fconfig"]:
            with self.subTest(name=repr(name)), self.assertRaises(worker.Refusal):
                worker.safe_name(name)
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp)
            (p / "caf\u00e9").write_bytes(b"one")
            (p / "cafe\u0301").write_bytes(b"two")
            with self.assertRaises(worker.Refusal):
                worker.collect_tree(p)

    def test_limits_and_hashes(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp)
            (p / "file").write_bytes(b"synthetic ordinary file")
            entries, total = worker.collect_tree(p)
            self.assertEqual(total, 23)
            self.assertEqual(entries[0]["sha256"], worker.digest(p / "file"))
            with patch.object(worker, "MAX_BYTES", 1), self.assertRaises(worker.Refusal):
                worker.collect_tree(p)
            with patch.object(worker, "MAX_ENTRIES", 0), self.assertRaises(worker.Refusal):
                worker.collect_tree(p)

    def test_gzip_expansion_bomb_is_bounded_before_disk_growth(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp)
            (p / "image.gz").write_bytes(gzip.compress(b"x" * 100000))
            original_path = worker.Path
            with patch.object(worker, "MAX_BYTES", 10), patch.object(worker, "Path", side_effect=lambda value: p / "expanded.bin" if value == "/work/expanded.bin" else original_path(value)), self.assertRaises(worker.Refusal):
                worker.GzipSquashfsBackend.prepare(p / "image.gz")
            self.assertEqual((p / "expanded.bin").stat().st_size, 0)

    def test_missing_or_changed_tools_refuse_before_start(self):
        with patch.object(worker, "TOOLS", {"/nonexistent/unsquashfs": "0" * 64}), patch.object(worker.os, "getuid", return_value=1000, create=True), self.assertRaises(worker.Refusal) as error:
            worker.supervise(Path("unused"), "a" * 64, 1)
        self.assertEqual(str(error.exception), "worker_unavailable")

    def test_supervisor_cancels_and_reaps_owned_child(self):
        child = unittest.mock.Mock()
        child.pid = 456
        child.poll.return_value = None
        with tempfile.TemporaryDirectory() as tmp:
            job = Path(tmp)
            (job / "cancel").touch()
            with patch.object(worker, "TOOLS", {}), patch.object(worker.os, "getuid", return_value=1000, create=True), patch.object(worker, "sandbox", return_value=["synthetic"]), patch.object(worker.subprocess, "Popen", return_value=child), patch.object(worker.signal, "SIGKILL", 9, create=True), patch.object(worker.os, "killpg", create=True) as kill, self.assertRaises(worker.Refusal):
                worker.supervise(job, "a"*64, 1)
            kill.assert_called_once()
            child.wait.assert_called_once_with(timeout=5)

    def test_start_failure_and_disk_full_propagate(self):
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(worker, "TOOLS", {}), patch.object(worker.os, "getuid", return_value=1000, create=True), patch.object(worker, "sandbox", return_value=["synthetic"]), patch.object(worker.subprocess, "Popen", side_effect=OSError("synthetic start failure")), self.assertRaises(OSError):
                worker.supervise(Path(tmp), "a"*64, 1)
        with patch.object(worker.Path, "open", side_effect=OSError(28, "synthetic disk full")), self.assertRaises(OSError):
            worker.GzipSquashfsBackend.prepare(Path("synthetic.gz"))
        self.assertTrue(worker.resource_error(OSError(errno.ENOSPC, "synthetic full tmpfs")))
        self.assertTrue(worker.resource_error(MemoryError()))
        self.assertFalse(worker.resource_error(OSError(errno.EIO, "synthetic corrupt storage")))

    def test_timeout_and_lost_owner_kill_and_reap(self):
        for lost in [False, True]:
            child = unittest.mock.Mock()
            child.pid = 456
            child.poll.return_value = None
            with tempfile.TemporaryDirectory() as tmp:
                job = Path(tmp)
                (job / "heartbeat").touch()
                if lost:
                    os.utime(job / "heartbeat", (0, 0))
                with patch.object(worker, "TOOLS", {}), patch.object(worker.os, "getuid", return_value=1000, create=True), patch.object(worker, "sandbox", return_value=["synthetic"]), patch.object(worker.subprocess, "Popen", return_value=child), patch.object(worker.signal, "SIGKILL", 9, create=True), patch.object(worker.os, "killpg", create=True) as kill, patch.object(worker.time, "monotonic", side_effect=[0, 200]), self.assertRaises(worker.Refusal) as error:
                    worker.supervise(job, "a"*64, 1)
                self.assertEqual(str(error.exception), "owner_disconnected" if lost else "timeout")
                kill.assert_called_once()
                child.wait.assert_called_once_with(timeout=5)

    def test_worker_crash_cannot_publish_success(self):
        child = unittest.mock.Mock()
        child.poll.return_value = -9
        child.returncode = -9
        child.stderr = io.BytesIO(b"synthetic crash")
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(worker, "TOOLS", {}), patch.object(worker.os, "getuid", return_value=1000, create=True), patch.object(worker, "sandbox", return_value=["synthetic"]), patch.object(worker.subprocess, "Popen", return_value=child), self.assertRaises(worker.Refusal) as error:
                worker.supervise(Path(tmp), "a"*64, 1)
            self.assertEqual(str(error.exception), "extraction_failed")


@unittest.skipUnless(sys.platform.startswith("linux"), "Linux kernel namespace tests")
class Linux(unittest.TestCase):
    def test_root_is_read_only_and_only_work_has_writable_file_storage(self):
        if not shutil.which("bwrap"):self.skipTest("bubblewrap unavailable")
        with tempfile.TemporaryDirectory() as tmp:
            image=Path(tmp)/"synthetic-input"
            image.write_bytes(b"synthetic input")
            command=worker.sandbox(Path(worker.__file__).resolve(),image)
            command=command[:command.index("/usr/bin/python3")]+["/usr/bin/python3","-I","-B","-c",
                "import os,pathlib; assert os.statvfs('/').f_flag & os.ST_RDONLY; assert not os.statvfs('/work').f_flag & os.ST_RDONLY; pathlib.Path('/work/allowed').write_text('synthetic'); assert not pathlib.Path('/mnt/c').exists(); assert not pathlib.Path('/home').exists()"]
            result=subprocess.run(command,capture_output=True,timeout=20)
            self.assertEqual(result.returncode,0,result.stderr.decode())

    def test_privileged_or_writable_parser_is_not_qualified_even_with_matching_hash(self):
        with tempfile.TemporaryDirectory() as tmp:
            binary=Path(tmp)/"synthetic-parser"
            binary.write_bytes(b"synthetic executable")
            expected=worker.digest(binary)
            for mode,allowed in [(stat.S_IFREG|0o755,True),(stat.S_IFREG|0o4755,False),(stat.S_IFREG|0o2755,False),(stat.S_IFREG|0o777,False),(stat.S_IFLNK|0o755,False)]:
                with patch.object(worker.Path,"lstat",return_value=SimpleNamespace(st_mode=mode,st_uid=0)),patch.object(worker.os,"getxattr",side_effect=OSError(errno.ENODATA,"synthetic no capabilities")):
                    self.assertEqual(worker.qualified_binary(binary,expected),allowed)
            with patch.object(worker.Path,"lstat",return_value=SimpleNamespace(st_mode=stat.S_IFREG|0o755,st_uid=0)),patch.object(worker.os,"getxattr",return_value=b"synthetic capability"):
                self.assertFalse(worker.qualified_binary(binary,expected))

    def test_near_limit_gzip_and_tree_share_one_real_tmpfs_budget(self):
        if not shutil.which("mksquashfs") or not subprocess.run(["unsquashfs", "-version"],capture_output=True).stdout.startswith(b"unsquashfs version 4.7.5"):
            self.skipTest("Qualified fixture generator unavailable")
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp)
            tree = p / "source"
            tree.mkdir()
            for i in range(4):
                with (tree / str(i)).open("wb") as file:
                    file.truncate(32 * 1024**2)
            for count, expected in [(4, "resource_exhausted"), (3, None)]:
                if count == 3:
                    (tree / "3").unlink()
                image = p / f"synthetic-{count}.sqfs"
                subprocess.run(["mksquashfs",str(tree),str(image),"-noappend","-processors","1","-quiet"],check=True,stdout=subprocess.DEVNULL)
                with image.open("r+b") as file:
                    file.truncate(120 * 1024**2)
                wrapped = p / f"synthetic-{count}.gz"
                with image.open("rb") as source, gzip.open(wrapped,"wb") as output:
                    shutil.copyfileobj(source,output,65536)
                original_hash = worker.digest(wrapped)
                command = worker.sandbox(Path(worker.__file__).resolve(),wrapped) + ["--sha256",original_hash,"--size",str(wrapped.stat().st_size)]
                with (p / f"output-{count}").open("w+b") as output:
                    result = subprocess.run(command,stdout=output,stderr=subprocess.PIPE,timeout=120)
                    if expected:
                        self.assertNotEqual(result.returncode,0)
                        self.assertEqual(result.stderr.strip(),expected.encode())
                        self.assertEqual(output.tell(),0,"Failure must not return a successful bundle")
                    else:
                        self.assertEqual(result.returncode,0,result.stderr)
                        output.seek(0)
                        header=output.read(10)
                        report=json.loads(output.read(struct.unpack("<I",header[6:])[0]))
                        self.assertEqual(report["totalBytes"],96 * 1024**2)
                        self.assertEqual(report["fileCount"],3)
                self.assertEqual(worker.digest(wrapped),original_hash)

    def test_links_are_metadata_and_never_followed(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp)
            (p / "ordinary").write_text("synthetic")
            (p / "escape").symlink_to("/etc/shadow")
            entries, total = worker.collect_tree(p)
            self.assertEqual(total, 9)
            self.assertEqual(entries[0]["kind"], "symlink_metadata")

    def test_special_file_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            os.mkfifo(Path(tmp) / "pipe")
            with self.assertRaises(worker.Refusal):
                worker.collect_tree(Path(tmp))

    def test_kernel_sandbox_and_real_synthetic_squashfs(self):
        if any(not shutil.which(t) for t in ("bwrap", "mksquashfs", "unsquashfs")):
            self.skipTest("Linux fixture tools unavailable")
        version = subprocess.run(["unsquashfs", "-version"], text=True, capture_output=True, check=False).stdout
        if not version.startswith("unsquashfs version 4.7.5"):
            self.skipTest("Qualified SquashFS 4.7.5 runtime unavailable; full extraction tested in native qualification")
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp)
            source = p / "source"
            source.mkdir()
            (source / "etc").mkdir()
            (source / "etc/config").write_text("synthetic fixture contents\n")
            (source / "link").symlink_to("/etc/shadow")
            image = p / "fixture.sqfs"
            subprocess.run(["mksquashfs", str(source), str(image), "-noappend", "-processors", "1", "-quiet"], check=True, stdout=subprocess.DEVNULL)
            script = Path(worker.__file__).resolve()
            cmd = worker.sandbox(script, image) + ["--sha256", worker.digest(image), "--size", str(image.stat().st_size)]
            result = subprocess.run(cmd, capture_output=True, timeout=30)
            # CI may use a different qualified tool version; pin enforcement is
            # tested separately in the outer supervisor. Inner parser is real.
            self.assertEqual(result.returncode, 0, result.stderr.decode())
            length = struct.unpack("<I", result.stdout[6:10])[0]
            report = json.loads(result.stdout[10:10+length])
            self.assertEqual(report["totalBytes"], 27)
            self.assertIn(b"synthetic fixture contents\n", result.stdout[10+length:])
            self.assertEqual(report["isolation"]["network"], False)


if __name__ == "__main__":
    unittest.main(verbosity=2)
