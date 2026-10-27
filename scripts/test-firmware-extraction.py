"""Synthetic extraction regressions. Real firmware qualification is separate."""
import gzip
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
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


class Parsing(unittest.TestCase):
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
