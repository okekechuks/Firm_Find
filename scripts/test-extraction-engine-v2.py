"""Small SYNTHETIC fixtures. Real images run separately inside production isolation."""
import bz2
import binascii
import gzip
import hashlib
import importlib.util
import io
import json
import lzma
from pathlib import Path
import stat
import subprocess
import struct
import sys
import tarfile
import tempfile
import unittest
import zipfile
from unittest.mock import patch


def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


core = load("extraction_core", "firmware-extract-worker.py")
engine = load("extraction_v2", "extraction-engine-v2.py")


def cpio(records, crc=True):
    """Synthetic newc headers; no production/device fixture identity."""
    output = bytearray()
    for index, (name, content, mode, links) in enumerate(records + [("TRAILER!!!", b"", stat.S_IFREG, 1)]):
        raw_name = name.encode() + b"\0"
        fields = [index + 1, mode, 0, 0, links, 0, len(content), 0, 0, 0, 0, len(raw_name), sum(content) & 0xffffffff if crc else 0]
        output += (b"070702" if crc else b"070701") + b"".join(f"{value:08x}".encode() for value in fields)
        output += raw_name
        output += bytes((-len(output)) % 4)
        output += content
        output += bytes((-len(output)) % 4)
    return bytes(output)


def ordinary(name="etc/config", content=b"synthetic data\n"):
    return (name, content, stat.S_IFREG | 0o644, 1)

def rar_block(kind, body):
    # Small synthetic RAR5 main/end headers, no compression or vendor data.
    header = bytes([kind,0]) + body
    encoded = bytes([len(header)]) + header
    return struct.pack("<I",binascii.crc32(encoded)) + encoded


class Formats(unittest.TestCase):
    def run_image(self, raw, workspace=None):
        if workspace is not None:
            path = workspace / "input.bin"
            path.write_bytes(raw)
            return engine.extract(core, path, workspace / "work")
        with tempfile.TemporaryDirectory() as tmp:
            return self.run_image(raw, Path(tmp))

    def test_newc_and_crc_retain_actual_bytes_modes_hashes_and_provenance(self):
        for crc in (False, True):
            report, _ = self.run_image(cpio([ordinary()], crc))
            entry = next(e for e in report["entries"] if e["path"] == "etc/config")
            self.assertEqual(entry["sha256"], hashlib.sha256(b"synthetic data\n").hexdigest())
            self.assertEqual(entry["mode"], 0o644)
            self.assertEqual(report["version"], 2)
            self.assertEqual(report["completeness"], "complete")
            self.assertEqual(report["inputSha256"], report["layers"][0]["inputSha256"])

    def test_archive_trailers_are_unexplained_and_regular_files_are_retained(self):
        buf = io.BytesIO()
        with tarfile.open(fileobj=buf, mode="w") as archive:
            entry = tarfile.TarInfo("synthetic.txt")
            entry.size = 4
            archive.addfile(entry, io.BytesIO(b"data"))
        tar = buf.getvalue()
        buf = io.BytesIO()
        with zipfile.ZipFile(buf, "w") as archive:
            archive.writestr("synthetic.txt", b"data")
            archive.comment = b"synthetic comment"
        zip_data = buf.getvalue()
        for original in (tar, zip_data):
            with self.subTest(format=original[:4]):
                report, _ = self.run_image(original)
                self.assertEqual(report["completeness"], "complete")
                report, _ = self.run_image(original + b"UNEXPLAINED-SECRET")
                self.assertEqual(report["completeness"], "partial")
                self.assertEqual(report["fileCount"], 1)
                self.assertTrue(any(r["offset"] == len(original) and r["length"] == 18 for r in report["unexplainedRegions"]))

    def test_zip_truncated_comment_and_ambiguous_end_markers_are_rejected(self):
        buf = io.BytesIO()
        with zipfile.ZipFile(buf, "w") as archive:
            archive.writestr("synthetic.txt", b"data")
            archive.comment = b"synthetic comment"
        data = buf.getvalue()
        for raw in (data[:-2], data + b"PK\x05\x06" + bytes(18)):
            with self.subTest(length=len(raw)), self.assertRaises(core.Refusal):
                self.run_image(raw)

    def test_cpio_crc_truncation_traversal_hardlink_special_and_setuid_are_rejected(self):
        good = cpio([ordinary()])
        bad_crc = bytearray(good)
        index = good.index(b"synthetic data")
        bad_crc[index] ^= 1
        bad = [bytes(bad_crc), good[:-6], cpio([ordinary("../escape")]), cpio([ordinary("/absolute")]),
               cpio([("hardlink", b"", stat.S_IFREG | 0o644, 2)]),
               cpio([("suid", b"data", stat.S_IFREG | 0o4755, 1)]),
               cpio([("escape", b"../../etc/shadow", stat.S_IFLNK | 0o777, 1)])]
        for raw in bad:
            with self.subTest(size=len(raw)), self.assertRaises(core.Refusal):
                self.run_image(raw)

    def test_cpio_device_nodes_are_rejected_as_output_and_retained_only_as_inert_metadata(self):
        report, files = self.run_image(cpio([("device", b"", stat.S_IFCHR | 0o666, 1)]))
        self.assertEqual(files,{})
        self.assertEqual(report["fileCount"],0)
        self.assertEqual(report["entries"][0]["kind"],"special_omitted")
        self.assertEqual(report["completeness"],"partial")

    def test_nested_cpio_links_cannot_escape_their_own_archive_root(self):
        safe = cpio([("sub/link", b"../target", stat.S_IFLNK | 0o777, 1)])
        escaped = cpio([("sub/link", b"../../other", stat.S_IFLNK | 0o777, 1)])
        safe_report, _ = self.run_image(cpio([ordinary("inner", safe)]))
        self.assertTrue(any(e["path"] == "inner.contents/sub/link" and e["linkTarget"] == "../target" for e in safe_report["entries"]))
        escaped_report, _ = self.run_image(cpio([ordinary("inner", escaped)]))
        self.assertFalse(any(e["path"] == "inner.contents/sub/link" for e in escaped_report["entries"]))
        self.assertTrue(any("unsafe_link" in reason for layer in escaped_report["layers"] for reason in layer["reasons"]))

    def test_squashfs_unsafe_links_are_omitted_without_losing_ordinary_files(self):
        # Synthetic decoder output; no filesystem is mounted or opened.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            image = root / "input.bin"
            image.write_bytes(b"synthetic squashfs")
            instance = engine.Engine(core, root / "work")
            layer = {"id": "layer-0000", "offset": 0, "parentId": None, "reasons": []}
            instance.layers.append(layer)
            def decoded(_image, _region, tree, _workspace, *_rest):
                tree.mkdir(parents=True)
                (tree / "normal").write_bytes(b"ordinary content")
                return ([
                    {"path": "normal", "kind": "file", "size": 16},
                    {"path": "sub/safe", "kind": "symlink_metadata", "size": 0, "sha256": None, "linkTarget": "../target"},
                    {"path": "sub/escape", "kind": "symlink_metadata", "size": 0, "sha256": None, "linkTarget": "../../outside"},
                    {"path": "absolute", "kind": "symlink_metadata", "size": 0, "sha256": None, "linkTarget": "/etc/shadow"},
                ], 16)
            with patch.object(engine.Squashfs, "plan", return_value={"region": {"offset": 0, "length": image.stat().st_size, "compression": 1}}), \
                 patch.object(instance, "use_tool"), patch.object(core, "extract_squashfs", side_effect=decoded):
                engine.Squashfs().extract(instance, engine.View(image), layer, "archive.contents", 0)
            self.assertEqual(instance.entries["archive.contents/sub/safe"]["linkTarget"], "../target")
            for name in ("sub/escape", "absolute"):
                entry = instance.entries["archive.contents/" + name]
                self.assertEqual(entry["kind"], "special_omitted")
                self.assertIsNone(entry["linkTarget"])
            self.assertEqual(instance.files["archive.contents/normal"].read_bytes(), b"ordinary content")
            self.assertTrue(any("Unsafe link target omitted" in reason for reason in layer["reasons"]))

    def test_uimage_trailing_archive_is_a_separate_layer(self):
        payload = b"synthetic opaque kernel"
        header = bytearray(struct.pack(">7I4B32s",0x27051956,0,0,len(payload),0,0,binascii.crc32(payload),5,2,2,0,b"synthetic"))
        struct.pack_into(">I",header,4,binascii.crc32(header))
        report, _ = self.run_image(bytes(header)+payload+cpio([ordinary()]))
        self.assertTrue(any(e["path"].endswith("etc/config") for e in report["entries"]))

    def test_recursive_gzip_xz_bzip2_and_lzma_are_detected_from_bytes(self):
        raw = cpio([ordinary()])
        for wrap in (gzip.compress, lzma.compress, bz2.compress, lambda data: lzma.compress(data, format=lzma.FORMAT_ALONE)):
            report, _ = self.run_image(wrap(raw))
            self.assertTrue(any(e["path"].endswith("etc/config") for e in report["entries"]))
            self.assertGreaterEqual(len(report["layers"]), 2)

    def test_valid_compressed_stream_with_container_padding_keeps_tail_unexplained(self):
        for wrap in (gzip.compress,lzma.compress,bz2.compress):
            stream = wrap(cpio([ordinary()]))
            report, _ = self.run_image(stream + bytes(32))
            self.assertTrue(any(e["path"].endswith("etc/config") for e in report["entries"]))
            self.assertTrue(any(region["offset"] == len(stream) and region["length"] == 32 for region in report["unexplainedRegions"]))
            self.assertEqual(report["completeness"], "partial")

    def test_swupdate_delta_declares_base_required_and_does_not_invent_a_filesystem(self):
        description = b'software={version="synthetic"; images:({filename="system.delta";type="rdiff_image";properties:{rdiffbase="base-sha";};});};'
        raw = cpio([ordinary("sw-description", description), ordinary("system.delta", b"BSDIFF40" + bytes(24))])
        report, _ = self.run_image(raw)
        self.assertEqual(report["completeness"], "partial")
        self.assertTrue(any("base required" in reason for layer in report["layers"] for reason in layer["reasons"]))
        self.assertEqual(report["swupdate"][0]["declarations"][0]["filename"], "system.delta")
        self.assertFalse(any(layer["format"] == "squashfs" for layer in report["layers"]))

    def test_include_directives_and_ambiguous_swupdate_descriptions_fail_closed(self):
        for text in [b'@include "/etc/shadow"', b'software={version="a";version="b";};']:
            with self.assertRaises(core.Refusal):
                self.run_image(cpio([ordinary("sw-description", text)]))

    def test_unknown_and_near_miss_have_explicit_unexplained_ranges(self):
        for raw in [b"070703" + bytes(110), b"BSDIFF41" + bytes(32), b"unrecognized synthetic wrapper"]:
            report, _ = self.run_image(raw)
            self.assertEqual(report["completeness"], "encrypted-or-unknown")
            self.assertEqual(report["entries"], [])
            self.assertEqual(report["unexplainedRegions"][0]["length"], len(raw))

    def test_false_embedded_fdt_does_not_hide_later_valid_carved_archive(self):
        prefix = b"synthetic-kernel" + b"\xd0\x0d\xfe\xed" + bytes(80)
        report, _ = self.run_image(prefix + cpio([ordinary()]))
        self.assertTrue(any(e["path"].endswith("etc/config") for e in report["entries"]))
        self.assertEqual(report["completeness"], "partial")

    def test_false_carved_compressed_stream_rolls_back_before_next_candidate(self):
        raw = b"synthetic prefix" + b"BZh1" + bytes(80) + cpio([ordinary()])
        report, _ = self.run_image(raw)
        self.assertEqual(report["fileCount"], 1)
        self.assertTrue(any(e["path"].endswith("etc/config") for e in report["entries"]))
        self.assertFalse(any(layer["format"] == "bzip2" for layer in report["layers"]))

    def test_rejected_candidates_cannot_refund_the_cumulative_expansion_budget(self):
        bad = bytearray(gzip.compress(b"x"*(512*1024),mtime=0))
        bad[-8] ^= 1  # Valid deflate bytes followed by a false CRC.
        raw = b"synthetic wrapper" + bytes(bad)*3 + cpio([ordinary()])
        with patch.object(engine,"MAX_EXPANDED",1024**2),self.assertRaisesRegex(core.Refusal,"output_limit"):
            self.run_image(raw)

    def test_vendor_header_carving_finds_nested_cpio_and_records_prefix(self):
        prefix = b"synthetic vendor prefix" * 17
        report, _ = self.run_image(prefix + cpio([ordinary()]))
        self.assertTrue(any(layer["offset"] == len(prefix) and layer["handler"] == "cpio" for layer in report["layers"]))
        self.assertTrue(any(region["offset"] == 0 and region["length"] == len(prefix) for region in report["unexplainedRegions"]))
        self.assertEqual(report["completeness"], "partial")

    def test_carving_keeps_two_independent_archives_and_the_gap(self):
        first, second = cpio([ordinary("first")]), cpio([ordinary("second")])
        report, _ = self.run_image(b"wrapper" + first + b"gap" + second)
        self.assertEqual(report["fileCount"], 2)
        self.assertTrue(any(entry["path"].endswith("/second") for entry in report["entries"]))

    def test_manifest_is_deterministic_and_resource_limits_are_shared(self):
        raw = gzip.compress(cpio([ordinary()]), mtime=0)
        first, _ = self.run_image(raw)
        second, _ = self.run_image(raw)
        self.assertEqual(json.dumps(first, sort_keys=True), json.dumps(second, sort_keys=True))
        nested = cpio([ordinary()])
        for _ in range(9):
            nested = gzip.compress(nested, mtime=0)
        report, _ = self.run_image(nested)
        self.assertEqual(report["completeness"], "partial")
        self.assertTrue(any("depth limit" in reason for layer in report["layers"] for reason in layer["reasons"]))

    def test_zip_slip_is_rejected_before_any_unsafe_output(self):
        stream = io.BytesIO()
        with zipfile.ZipFile(stream, "w") as archive:
            archive.writestr("../../outside", b"synthetic")
        with self.assertRaises(core.Refusal):
            self.run_image(stream.getvalue())

    def test_rejected_nested_archive_keeps_safe_outer_bytes_without_escaped_output(self):
        stream = io.BytesIO()
        with zipfile.ZipFile(stream,"w") as archive:
            archive.writestr("../escape",b"never admitted")
        report, _ = self.run_image(cpio([ordinary("nested.zip",stream.getvalue()),ordinary()]))
        self.assertEqual(report["fileCount"],2)
        self.assertFalse(any("escape" in entry["path"] for entry in report["entries"]))
        self.assertEqual(report["completeness"],"partial")
        self.assertTrue(any("unsafe_path" in reason for layer in report["layers"] for reason in layer["reasons"]))

    def test_uimage_checks_both_crcs_and_recurses_without_executing_payload(self):
        payload = gzip.compress(cpio([ordinary()]), mtime=0)
        header = bytearray(struct.pack(">7I4B32s", 0x27051956, 0, 0, len(payload), 0, 0, binascii.crc32(payload), 5, 2, 2, 1, b"synthetic"))
        struct.pack_into(">I", header, 4, binascii.crc32(header))
        report, _ = self.run_image(bytes(header) + payload)
        self.assertEqual(report["layers"][0]["format"], "uimage")
        self.assertTrue(any(e["path"].endswith("etc/config") for e in report["entries"]))
        for offset in (4, 24, len(header) + 5):
            broken = bytearray(header + payload)
            broken[offset] ^= 1
            with self.assertRaises(core.Refusal):
                self.run_image(broken)

    def test_android_sparse_checks_chunk_sizes_crc_and_expansion_budget(self):
        payload = cpio([ordinary()])
        payload += bytes((-len(payload)) % 4)
        header = struct.pack("<I4H4I", 0xed26ff3a, 1, 0, 28, 12, 4, len(payload) // 4, 1, binascii.crc32(payload))
        raw = header + struct.pack("<2H2I", 0xcac1, 0, len(payload) // 4, len(payload) + 12) + payload
        report, _ = self.run_image(raw)
        self.assertTrue(any(layer["format"] == "android_sparse" for layer in report["layers"]))
        self.assertTrue(any(e["path"].endswith("etc/config") for e in report["entries"]))
        for broken in (raw[:-1], raw[:24] + struct.pack("<I", 42) + raw[28:],
                       struct.pack("<I4H4I", 0xed26ff3a, 1, 0, 28, 12, 4096, 40000, 1, 0) + struct.pack("<2H2I", 0xcac3, 0, 40000, 12)):
            with self.assertRaises(core.Refusal):
                self.run_image(broken)

    def test_cpio_root_directory_is_metadata_and_directory_link_count_is_not_a_hardlink(self):
        report, _ = self.run_image(cpio([(".", b"", stat.S_IFDIR | 0o755, 2), ("etc", b"", stat.S_IFDIR | 0o755, 2), ordinary()]))
        self.assertEqual(report["fileCount"], 1)
        self.assertEqual(report["directoryCount"], 1)

    def test_compression_bomb_fails_without_admitting_expanded_bytes(self):
        with patch.object(engine, "MAX_RATIO", 64), self.assertRaisesRegex(core.Refusal, "decompression_bomb"):
            self.run_image(gzip.compress(bytes(4 * 1024**2), mtime=0))

    def test_known_unqualified_filesystems_are_never_reported_as_extracted(self):
        for magic, name in [(b"\x31\x18\x10\x06", "ubifs"),
                            (b"\x85\x19", "jffs2")]:
            report, _ = self.run_image(magic + bytes(128))
            self.assertEqual(report["layers"][0]["format"], name)
            self.assertNotEqual(report["completeness"], "complete")
            self.assertEqual(report["fileCount"], 0)

    def test_ubi_static_volume_crc_geometry_and_duplicate_logical_blocks(self):
        payload = cpio([ordinary()])
        def block(number, content):
            ec = bytearray(64)
            ec[:5] = b"UBI#\x01"
            struct.pack_into(">3I", ec, 16, 64, 128, 7)
            struct.pack_into(">I", ec, 60, binascii.crc32(ec[:60]) ^ 0xffffffff)
            vid = bytearray(64)
            vid[:8] = b"UBI!\x01\x02\0\0"
            struct.pack_into(">2I", vid, 8, 0, number)
            struct.pack_into(">4I", vid, 20, len(content), 1, 0, binascii.crc32(content) ^ 0xffffffff)
            struct.pack_into(">I", vid, 60, binascii.crc32(vid[:60]) ^ 0xffffffff)
            return bytes(ec + vid) + content + b"\xff" * (4096 - 128 - len(content))
        raw = block(0, payload)
        report, _ = self.run_image(raw)
        self.assertEqual(report["layers"][0]["handler"], "ubi")
        self.assertTrue(any(e["path"].endswith("etc/config") for e in report["entries"]))
        for broken in (raw[:-1], raw + raw, raw[:128] + bytes([raw[128] ^ 1]) + raw[129:]):
            with self.assertRaises(core.Refusal):
                self.run_image(broken)

    def test_rar5_has_a_separate_handler_and_missing_decoder_is_explicit(self):
        report, _ = self.run_image(b"Rar!\x1a\x07\x01\0" + rar_block(1,b"\0") + rar_block(5,b"\0"))
        self.assertEqual(report["layers"][0]["handler"], "rar5")
        self.assertEqual(report["completeness"], "unsupported")

    def test_rar_header_crc_bounds_and_solid_skip_are_checked_before_decoder(self):
        raw = b"Rar!\x1a\x07\x01\0" + rar_block(1,b"\x04") + rar_block(5,b"\0")
        report, _ = self.run_image(raw)
        self.assertTrue(any("solid" in reason.lower() for reason in report["layers"][0]["reasons"]))
        for broken in (raw[:-1],raw[:8]+bytes(4)+raw[12:],b"Rar!\x1a\x07\x01\0"+bytes(4)+b"\x80"*11):
            with self.assertRaises(core.Refusal):
                self.run_image(broken)

    def test_android_boot_v0_v3_v4_components_are_bounded_and_recursed(self):
        payload = cpio([ordinary()])
        for version in (0, 3, 4):
            header = bytearray(4096)
            header[:8] = b"ANDROID!"
            struct.pack_into("<I", header, 40, version)
            if not version:
                struct.pack_into("<I", header, 16, len(payload))
                struct.pack_into("<I", header, 36, 4096)
            else:
                struct.pack_into("<I", header, 12, len(payload))
                struct.pack_into("<I", header, 20, 1584 if version == 4 else 1580)
            raw = bytes(header) + payload + bytes((-len(payload)) % 4096)
            report, _ = self.run_image(raw)
            self.assertEqual(report["layers"][0]["handler"], "android_boot")
            self.assertTrue(any(e["path"].endswith("etc/config") for e in report["entries"]))
            with self.assertRaises(core.Refusal):
                self.run_image(raw[:4096 + len(payload) - 1])

    def test_fdt_fit_inline_image_sha256_and_invalid_hash(self):
        payload = cpio([ordinary()])
        strings = b"data\0algo\0value\0"
        def node(name):
            raw = name.encode() + b"\0"
            return struct.pack(">I", 1) + raw + bytes((-len(raw)) % 4)
        def prop(index, data):
            return struct.pack(">3I", 3, len(data), index) + data + bytes((-len(data)) % 4)
        structure = node("") + node("images") + node("rootfs") + prop(0, payload) + node("hash-1") + prop(5, b"sha256\0") + prop(10, hashlib.sha256(payload).digest()) + struct.pack(">5I", 2, 2, 2, 2, 9)
        total = 56 + len(structure) + len(strings)
        raw = struct.pack(">10I", 0xd00dfeed, total, 56, 56 + len(structure), 40, 17, 16, 0, len(strings), len(structure)) + bytes(16) + structure + strings
        report, _ = self.run_image(raw)
        self.assertEqual(report["layers"][0]["handler"], "fit")
        self.assertTrue(any(e["path"].endswith("etc/config") for e in report["entries"]))
        broken = raw.replace(hashlib.sha256(payload).digest(), bytes(32))
        with self.assertRaises(core.Refusal):
            self.run_image(broken)


@unittest.skipUnless(sys.platform.startswith("linux"), "Production filesystem isolation requires Linux")
class FilesystemIsolation(unittest.TestCase):
    def test_lz4_and_zstd_stream_inside_the_same_namespace(self):
        pins = [("/usr/bin/lz4", "4be960d6f6b0d7ef69e01a9e1a056591c17b8687e9851db128018b2ac5f01da0"),
                ("/usr/bin/zstd", "fe50fd600cad89b775dc5b4b10bc8ff95e2d33ddb4f7632966464b1fbdab8598")]
        if any(not core.qualified_binary(Path(name),sha) for name,sha in pins):
            self.skipTest("Pinned compression fixture/runtime unavailable")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "synthetic.cpio"
            source.write_bytes(cpio([ordinary()]))
            for binary,_ in pins:
                image = root / Path(binary).name
                with image.open("wb") as output:
                    subprocess.run([binary,"-q","-c",str(source)],check=True,stdout=output)
                command = core.sandbox(Path(core.__file__).resolve(),image) + ["--sha256",core.digest(image),"--size",str(image.stat().st_size)]
                result = subprocess.run(command,capture_output=True,timeout=30)
                self.assertEqual(result.returncode,0,result.stderr)
                length = struct.unpack("<I",result.stdout[6:10])[0]
                report = json.loads(result.stdout[10:10+length])
                self.assertEqual(report["layers"][0]["format"],Path(binary).name)
                self.assertEqual(report["fileCount"],1)

    def test_cramfs_readonly_synthetic_image(self):
        if not Path("/usr/sbin/mkfs.cramfs").exists() or not core.qualified_binary(Path("/usr/sbin/fsck.cramfs"), core.OPTIONAL_TOOLS["/usr/sbin/fsck.cramfs"]):
            self.skipTest("Pinned util-linux fixture/runtime unavailable")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "synthetic-source"
            source.mkdir()
            (source / "config").write_bytes(b"synthetic cramfs contents\n")
            image = root / "synthetic.cramfs"
            subprocess.run(["/usr/sbin/mkfs.cramfs",str(source),str(image)],check=True,stdout=subprocess.DEVNULL)
            command = core.sandbox(Path(core.__file__).resolve(), image) + ["--sha256", core.digest(image), "--size", str(image.stat().st_size)]
            result = subprocess.run(command,capture_output=True,timeout=30)
            self.assertEqual(result.returncode,0,result.stderr)
            length = struct.unpack("<I",result.stdout[6:10])[0]
            report = json.loads(result.stdout[10:10+length])
            self.assertEqual(report["layers"][0]["handler"],"cramfs")
            self.assertEqual(report["fileCount"],1)
            self.assertIn(b"synthetic cramfs contents\n",result.stdout[10+length:])

    def test_ext2_ext3_ext4_readers_use_real_readonly_synthetic_images(self):
        if not Path("/usr/sbin/mke2fs").exists() or not core.qualified_binary(Path("/usr/sbin/debugfs"), core.OPTIONAL_TOOLS["/usr/sbin/debugfs"]):
            self.skipTest("Pinned e2fsprogs fixture/runtime unavailable")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "synthetic-source"
            source.mkdir()
            (source / "config").write_bytes(b"synthetic ext contents\n")
            for filesystem in ("ext2", "ext3", "ext4"):
                image = root / (filesystem + ".img")
                subprocess.run(["/usr/sbin/mke2fs", "-q", "-F", "-t", filesystem, "-d", str(source), str(image), "16384"], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                command = core.sandbox(Path(core.__file__).resolve(), image) + ["--sha256", core.digest(image), "--size", str(image.stat().st_size)]
                result = subprocess.run(command, capture_output=True, timeout=60)
                self.assertEqual(result.returncode, 0, result.stderr)
                length = struct.unpack("<I", result.stdout[6:10])[0]
                report = json.loads(result.stdout[10:10 + length])
                self.assertEqual(report["layers"][0]["handler"], "ext")
                self.assertEqual(report["fileCount"], 1)
                self.assertIn(b"synthetic ext contents\n", result.stdout[10 + length:])


if __name__ == "__main__":
    unittest.main(verbosity=2)
