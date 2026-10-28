"""OBTAINER layer engine. Production imports this only inside its namespace.

No extracted path is opened as a host path. Data is retained in generated flat
files; logical paths exist only in the manifest. Synthetic unit tests may call
the pure parsers with generated inputs. Real inputs use the production worker.
"""
import binascii
import bz2
from collections import Counter
import copy
import hashlib
import io
import json
import lzma
import math
import os
from pathlib import Path
import re
import shutil
import stat
import struct
import subprocess
import sys
import tarfile
import unicodedata
import zipfile
import zlib

VERSION = "2.0.0"
MAX_DEPTH = 6
MAX_LAYERS = 256
MAX_RATIO = 1024
MAX_FILE = 32 * 1024**2
MAX_EXPANDED = 128 * 1024**2
MAX_SCAN_MATCHES = 4096
MAX_ENTROPY_WINDOWS = 512


class View:
    def __init__(self, path, offset=0, length=None):
        self.path = Path(path)
        self.offset = offset
        size = self.path.stat().st_size
        self.length = size - offset if length is None else length
        if min(offset, self.length) < 0 or offset + self.length > size:
            raise ValueError("Invalid bounded view")

    def read(self, offset=0, length=65536):
        if offset < 0 or offset > self.length:
            raise ValueError("Invalid view offset")
        with self.path.open("rb") as source:
            source.seek(self.offset + offset)
            return source.read(min(length, self.length - offset))

    def chunks(self, offset=0, length=None):
        remaining = self.length - offset if length is None else length
        if min(offset, remaining) < 0 or offset + remaining > self.length:
            raise ValueError("Invalid view range")
        with self.path.open("rb") as source:
            source.seek(self.offset + offset)
            while remaining:
                chunk = source.read(min(remaining, 65536))
                if not chunk:
                    raise ValueError("Truncated bounded view")
                yield chunk
                remaining -= len(chunk)

    def digest(self):
        digest = hashlib.sha256()
        for chunk in self.chunks():
            digest.update(chunk)
        return digest.hexdigest()

    def sub(self, offset, length=None):
        length = self.length - offset if length is None else length
        if min(offset, length) < 0 or offset + length > self.length:
            raise ValueError("Invalid child view")
        return View(self.path, self.offset + offset, length)


class Libconfig:
    """Closed, bounded libconfig grammar. No includes, evaluation or file I/O."""
    def __init__(self, raw, refuse):
        if len(raw) > 65536:
            raise refuse("description_limit")
        self.fail = refuse
        self.text = raw.decode("utf-8", "strict")
        self.offset = 0
        self.tokens = 0

    def token(self):
        self.tokens += 1
        if self.tokens > 8192:
            raise self.fail("description_limit")
        text = self.text
        while self.offset < len(text):
            if text[self.offset].isspace():
                self.offset += 1
            elif text.startswith("/*", self.offset):
                end = text.find("*/", self.offset + 2)
                if end < 0:
                    raise self.fail("corrupt_description")
                self.offset = end + 2
            elif text.startswith("//", self.offset) or text[self.offset] == "#":
                end = text.find("\n", self.offset)
                self.offset = len(text) if end < 0 else end + 1
            else:
                break
        if self.offset == len(text):
            return ("end", "")
        value = text[self.offset]
        if value in "{}()[]=:;,":
            self.offset += 1
            return (value, value)
        if value == '"':
            self.offset += 1
            output = []
            while self.offset < len(text):
                value = text[self.offset]
                self.offset += 1
                if value == '"':
                    return ("string", "".join(output))
                if value == "\\":
                    if self.offset == len(text):
                        break
                    escaped = text[self.offset]
                    self.offset += 1
                    codes = {'"': '"', "\\": "\\", "n": "\n", "r": "\r", "t": "\t", "f": "\f", "a": "\a", "b": "\b", "v": "\v"}
                    if escaped == "x":
                        digits = text[self.offset:self.offset + 2]
                        if not re.fullmatch("[0-9a-fA-F]{2}", digits):
                            break
                        output.append(chr(int(digits, 16)))
                        self.offset += 2
                    elif escaped in codes:
                        output.append(codes[escaped])
                    else:
                        break
                else:
                    output.append(value)
                if len(output) > 2048:
                    raise self.fail("description_limit")
            raise self.fail("corrupt_description")
        match = re.match(r"[A-Za-z*][-A-Za-z0-9_*]*|[-+]?(?:0[xX][0-9a-fA-F]+|[0-9]+)(?:\.[0-9]+)?(?:[eE][-+]?[0-9]+)?L?", text[self.offset:])
        if not match:
            # Includes and substitutions are never interpreted.
            raise self.fail("corrupt_description")
        self.offset += len(match[0])
        return ("word", match[0])

    def parse(self):
        self.current = self.token()
        result = self.group("end", 0)
        if self.current[0] != "end":
            raise self.fail("corrupt_description")
        return result

    def take(self, expected=None):
        value = self.current
        if expected and value[0] != expected:
            raise self.fail("corrupt_description")
        self.current = self.token()
        return value[1]

    def group(self, terminator, depth):
        if depth > 16:
            raise self.fail("description_limit")
        result = {}
        while self.current[0] != terminator:
            name = self.take("word")
            if not re.fullmatch(r"[A-Za-z*][-A-Za-z0-9_*]*", name) or name in result or len(result) >= 256:
                raise self.fail("corrupt_description")
            if self.current[0] not in ("=", ":"):
                raise self.fail("corrupt_description")
            self.take()
            result[name] = self.value(depth + 1)
            if self.current[0] in (";", ","):
                self.take()
        return result

    def value(self, depth):
        if depth > 16:
            raise self.fail("description_limit")
        kind = self.current[0]
        if kind == "{":
            self.take()
            result = self.group("}", depth)
            self.take("}")
            return result
        if kind in ("(", "["):
            self.take()
            end = ")" if kind == "(" else "]"
            result = []
            while self.current[0] != end:
                if len(result) >= 256:
                    raise self.fail("description_limit")
                result.append(self.value(depth + 1))
                if self.current[0] != end:
                    self.take(",")
            self.take(end)
            return result
        if kind == "string":
            result = self.take()
            while self.current[0] == "string":
                result += self.take()
                if len(result) > 2048:
                    raise self.fail("description_limit")
            return result
        value = self.take("word")
        if value.lower() in ("true", "false"):
            return value.lower() == "true"
        if re.fullmatch(r"[-+]?(?:0[xX][0-9a-fA-F]+|[0-9]+)(?:\.[0-9]+)?(?:[eE][-+]?[0-9]+)?L?", value):
            return value  # Observed scalar; never an offset, expression or command.
        raise self.fail("corrupt_description")


class Handler:
    name = "unknown"
    version = 1
    magics = ()

    def detect(self, view):
        prefix = view.read(0, 1100)
        return {"confidence":"signature", "offset":0} if any(prefix.startswith(magic) for magic in self.magics) else None

    def plan(self, engine, view):
        return {"confidence": "signature", "offset": 0, "length": view.length}

    def extract(self, engine, view, layer, prefix, depth):
        layer["completeness"] = "unsupported"
        layer["reasons"].append("No qualified decoder for this format")
        engine.unexplained(layer, 0, view.length, "unsupported format")

    def report(self, layer):
        return layer


class Cpio(Handler):
    name = "cpio"
    magics = (b"070701", b"070702")

    def plan(self, engine, view):
        if view.length < 110 or not re.fullmatch(b"[0-9A-Fa-f]{104}", view.read(6, 104)):
            raise engine.fail("corrupt_image")
        offset = 0
        for _ in range(engine.core.MAX_ENTRIES + 1):
            raw = view.read(offset, 110)
            if len(raw) != 110 or raw[:6] not in self.magics or not re.fullmatch(b"[0-9A-Fa-f]{104}", raw[6:]):
                raise engine.fail("corrupt_image")
            length, name_size = int(raw[54:62], 16), int(raw[94:102], 16)
            if not 1 < name_size <= 241:
                raise engine.fail("corrupt_image")
            name = view.read(offset + 110, name_size)
            data = (offset + 110 + name_size + 3) & ~3
            end = (data + length + 3) & ~3
            if end > view.length:
                raise engine.fail("corrupt_image")
            if name == b"TRAILER!!!\0" and not length:
                return {"confidence": "validated", "offset": 0, "length": end}
            offset = end
        raise engine.fail("entry_limit")

    def extract(self, engine, view, layer, prefix, depth):
        offset = 0
        names = set()
        files = []
        trailer = False
        while offset < view.length:
            raw = view.read(offset, 110)
            if len(raw) != 110 or raw[:6] not in self.magics or not re.fullmatch(b"[0-9A-Fa-f]{104}", raw[6:]):
                raise engine.fail("corrupt_image")
            fields = [int(raw[i:i + 8], 16) for i in range(6, 110, 8)]
            _, mode, _, _, links, _, size, _, _, _, _, name_size, checksum = fields
            if not 1 < name_size <= 241 or size > view.length - offset:
                raise engine.fail("corrupt_image")
            name_raw = view.read(offset + 110, name_size)
            if len(name_raw) != name_size or name_raw[-1:] != b"\0" or b"\0" in name_raw[:-1]:
                raise engine.fail("corrupt_image")
            name = name_raw[:-1].decode("utf-8", "strict")
            data = (offset + 110 + name_size + 3) & ~3
            end = (data + size + 3) & ~3
            if end > view.length:
                raise engine.fail("corrupt_image")
            if any(view.read(offset + 110 + name_size, data - offset - 110 - name_size)) or any(view.read(data + size, end - data - size)):
                raise engine.fail("corrupt_image")
            actual = sum(sum(chunk) for chunk in view.chunks(data, size)) & 0xffffffff
            if (raw[:6] == b"070702" and checksum != actual) or (raw[:6] == b"070701" and checksum != 0):
                raise engine.fail("corrupt_image")
            if name == "TRAILER!!!":
                if size:
                    raise engine.fail("corrupt_image")
                trailer = True
                offset = end
                break
            if name.startswith("./"):
                name = name[2:]
            if name == "." and stat.S_ISDIR(mode) and not size and not mode & 0o6000:
                offset = end
                continue  # Archive root is metadata, never a host path.
            engine.core.safe_name(name)
            folded = unicodedata.normalize("NFC", name).casefold()
            if folded in names:
                raise engine.fail("colliding_paths")
            names.add(folded)
            if mode & 0o6000 or (links != 1 and not stat.S_ISDIR(mode)):
                raise engine.fail("unsafe_mode" if mode & 0o6000 else "unsafe_link")
            destination = engine.join(prefix, name)
            kind = stat.S_IFMT(mode)
            if kind == stat.S_IFREG:
                file = engine.add_file(destination, view.sub(data, size), mode & 0o777)
                files.append((destination, file))
            elif kind == stat.S_IFDIR:
                if size:
                    raise engine.fail("corrupt_image")
                engine.add_directory(destination, mode & 0o777, explicit=True)
            elif kind == stat.S_IFLNK:
                if size > 1024:
                    raise engine.fail("unsafe_link")
                engine.add_link(destination, view.read(data, size).decode("utf-8", "strict"), mode & 0o777, archive_root=prefix)
                layer["reasons"].append("Links retained as inert metadata")
            else:
                if size:
                    raise engine.fail("corrupt_image")
                engine.parents(destination)
                engine.claim(destination,"special_omitted")
                engine.entries[destination] = {"path":destination,"kind":"special_omitted","size":0,"sha256":None,"linkTarget":None,"mode":mode & 0o777,"offset":0}
                engine.budget()
                layer["reasons"].append("Special entries rejected as output; inert metadata only, no device node created")
            offset = end
        if not trailer:
            raise engine.fail("corrupt_image")
        trailing = view.length - offset
        if trailing and not any(any(chunk) for chunk in view.chunks(offset)):
            offset = view.length
        if offset < view.length:
            engine.unexplained(layer, offset, view.length - offset, "bytes after CPIO trailer")
        layer["format"] = "cpio_crc" if view.read(0, 6) == b"070702" else "cpio_newc"
        for name, path in files:
            if name == engine.join(prefix, "sw-description"):
                engine.swupdate_description(View(path).read(0, 65537), layer)
        for name, path in files:
            engine.nested_file(path, name, layer, depth)


class Compression(Handler):
    name = "compression"
    magics = (b"\x1f\x8b\x08", b"\xfd7zXZ\0", b"BZh", b"\x28\xb5\x2f\xfd", b"\x04\x22\x4d\x18")

    def detect(self, view):
        raw = view.read(0, 13)
        if super().detect(view):
            return {"confidence":"signature", "offset":0}
        # LZMA-alone has no magic. Accept only a plausible complete header;
        # decoding still has to validate EOF and its declared size.
        if len(raw) != 13 or raw[0] not in (0x5d, 0x6d, 0x7d):
            return None
        dictionary = int.from_bytes(raw[1:5], "little")
        return {"confidence":"signature", "offset":0} if dictionary in [1 << i for i in range(12, 27)] else None

    def plan(self, engine, view):
        raw = view.read(0, 13)
        if raw.startswith(b"\x1f\x8b"):
            if len(raw) < 10 or raw[2] != 8 or raw[3] & 0xe0:
                raise engine.fail("corrupt_image")
            format_name = "gzip"
        elif raw.startswith(b"\xfd7zXZ\0"):
            format_name = "xz"
        elif raw.startswith(b"BZh"):
            if raw[3:4] not in [bytes([n]) for n in range(49, 58)]:
                raise engine.fail("corrupt_image")
            format_name = "bzip2"
        elif raw.startswith(b"\x28\xb5\x2f\xfd"):
            format_name = "zstd"
        elif raw.startswith(b"\x04\x22\x4d\x18"):
            format_name = "lz4"
        else:
            format_name = "lzma"
        return {"confidence": "validated", "offset": 0, "length": view.length, "format": format_name}

    def extract(self, engine, view, layer, prefix, depth):
        kind = layer["format"]
        if kind in ("zstd", "lz4"):
            return engine.external_compression(view, layer, prefix, depth)
        decoder = zlib.decompressobj(31) if kind == "gzip" else bz2.BZ2Decompressor() if kind == "bzip2" else lzma.LZMADecompressor(format=lzma.FORMAT_ALONE if kind == "lzma" else lzma.FORMAT_XZ, memlimit=64 * 1024**2)
        stage = engine.new_stage()
        total, input_seen, consumed = 0, 0, 0
        try:
            with stage.open("xb") as output:
                for chunk in view.chunks():
                    input_seen += len(chunk)
                    pending = chunk
                    while pending or (kind != "gzip" and not decoder.needs_input and not decoder.eof):
                        decoded = decoder.decompress(pending, max_length=65536)
                        total += len(decoded)
                        engine.expanded(len(decoded), total, view.length)
                        engine.stage_write(output, decoded)
                        pending = decoder.unconsumed_tail if kind == "gzip" else b""
                        if decoder.eof:
                            consumed = input_seen - len(decoder.unused_data)
                            break
                    if decoder.eof:
                        break
                if not decoder.eof or not total:
                    raise engine.fail("corrupt_image")
            layer["decodedBytes"] = total
            if total > consumed * MAX_RATIO + 1024**2:
                raise engine.fail("decompression_bomb")
            engine.walk(View(stage), layer, engine.contents(prefix), depth + 1, "expanded", 0)
            engine.unexplained(layer, consumed, view.length - consumed, "bytes after compressed stream; not part of decoded payload")
        except (zlib.error, lzma.LZMAError, EOFError, OSError) as error:
            if engine.core.resource_error(error):
                raise engine.fail("resource_exhausted") from None
            raise engine.fail("corrupt_image") from None
        finally:
            engine.drop_stage(stage)


class Delta(Handler):
    name = "delta"
    magics = (b"BSDIFF40", b"\x72\x73\x02\x36")

    def extract(self, engine, view, layer, prefix, depth):
        if view.read(0, 8) == b"BSDIFF40":
            if view.length < 32:
                raise engine.fail("corrupt_image")
            fields = []
            for offset in (8,16,24):
                raw = view.read(offset,8)
                value = int.from_bytes(raw,"little") & 0x7fffffffffffffff
                if raw[7] & 0x80:
                    value = -value
                fields.append(value)
            control,difference,target = fields
            if min(fields) < 0 or 32 + control + difference > view.length:
                raise engine.fail("corrupt_image")
            layer["reasons"].append(f"BSDIFF40 declares control={control}, difference={difference}, target={target} bytes; header declarations only")
        layer["completeness"] = "unsupported"
        layer["reasons"].append("delta, base required; no patch application was attempted")
        engine.unexplained(layer, 0, view.length, "delta, exact base required")


class Uimage(Handler):
    """Legacy U-Boot CRC header. Boot addresses are never used."""
    name = "uimage"
    magics = (b"\x27\x05\x19\x56",)

    def plan(self, engine, view):
        raw = view.read(0, 64)
        if len(raw) != 64:
            raise engine.fail("corrupt_image")
        _, header_crc, _, size, _, _, data_crc = struct.unpack(">7I", raw[:28])
        if not size or size > view.length - 64 or binascii.crc32(raw[:4] + bytes(4) + raw[8:]) != header_crc:
            raise engine.fail("corrupt_image")
        crc = 0
        for chunk in view.chunks(64, size):
            crc = binascii.crc32(chunk, crc)
        if crc != data_crc:
            raise engine.fail("corrupt_image")
        return {"confidence": "validated", "offset": 0, "length": 64 + size}

    def extract(self, engine, view, layer, prefix, depth):
        size = self.plan(engine, view)["length"] - 64
        engine.walk(view.sub(64, size), layer, engine.join(prefix, "payload"), depth + 1, "source", 64)
        if 64 + size < view.length:
            engine.walk(view.sub(64 + size), layer, prefix, depth + 1, "source", 64 + size)
            layer["reasons"].append("Trailing image bytes inspected independently of the uImage payload")


class AndroidSparse(Handler):
    name = "android_sparse"
    magics = (b"\x3a\xff\x26\xed",)

    def extract(self, engine, view, layer, prefix, depth):
        raw = view.read(0, 28)
        if len(raw) != 28:
            raise engine.fail("corrupt_image")
        _, major, minor, header_size, chunk_size, block, blocks, chunks, checksum = struct.unpack("<I4H4I", raw)
        if major != 1 or minor != 0 or header_size != 28 or chunk_size != 12 or not block or block % 4 or not chunks or chunks > 20000:
            raise engine.fail("corrupt_image")
        expected = blocks * block
        if not expected or expected > MAX_EXPANDED:
            raise engine.fail("output_limit")
        stage = engine.new_stage()
        offset, total, crc = header_size, 0, 0
        try:
            with stage.open("xb") as output:
                for _ in range(chunks):
                    chunk = view.read(offset, 12)
                    if len(chunk) != 12:
                        raise engine.fail("corrupt_image")
                    kind, reserved, count, length = struct.unpack("<2H2I", chunk)
                    size = count * block
                    if reserved or length < 12 or offset + length > view.length or total + size > expected:
                        raise engine.fail("corrupt_image")
                    if kind == 0xcac1 and length == 12 + size:
                        source = view.chunks(offset + 12, size)
                    elif kind in (0xcac2, 0xcac3) and length == (16 if kind == 0xcac2 else 12):
                        pattern = view.read(offset + 12, 4) if kind == 0xcac2 else bytes(4)
                        source = (pattern * (min(65536, size - index) // 4) for index in range(0, size, 65536))
                        if kind == 0xcac3:
                            layer["reasons"].append("Sparse don't-care blocks rendered as zeroes; original storage bytes unknown")
                    elif kind == 0xcac4 and not count and length == 16:
                        if struct.unpack("<I", view.read(offset + 12, 4))[0] != crc:
                            raise engine.fail("corrupt_image")
                        source = ()
                    else:
                        raise engine.fail("corrupt_image")
                    for data in source:
                        engine.expanded(len(data), total + len(data), view.length)
                        engine.stage_write(output, data)
                        crc = binascii.crc32(data, crc)
                        total += len(data)
                    offset += length
            if total != expected or offset != view.length or (checksum and crc != checksum):
                raise engine.fail("corrupt_image")
            layer["confidence"] = "validated"
            layer["decodedBytes"] = total
            engine.walk(View(stage), layer, engine.contents(prefix), depth + 1, "expanded", 0)
        finally:
            engine.drop_stage(stage)


class AndroidBoot(Handler):
    name = "android_boot"
    magics = (b"ANDROID!",)

    def extract(self, engine, view, layer, prefix, depth):
        raw = view.read(0, 4096)
        if len(raw) < 1580:
            raise engine.fail("corrupt_image")
        version = int.from_bytes(raw[40:44], "little")
        if version == 0:
            page = int.from_bytes(raw[36:40], "little")
            if page not in (2048, 4096, 8192, 16384):
                raise engine.fail("corrupt_image")
            parts = [("kernel", int.from_bytes(raw[8:12], "little")), ("ramdisk", int.from_bytes(raw[16:20], "little")), ("second", int.from_bytes(raw[24:28], "little"))]
        elif version in (3, 4):
            page = 4096
            header_size = int.from_bytes(raw[20:24], "little")
            if header_size != (1580 if version == 3 else 1584) or any(raw[24:40]):
                raise engine.fail("corrupt_image")
            parts = [("kernel", int.from_bytes(raw[8:12], "little")), ("ramdisk", int.from_bytes(raw[12:16], "little"))]
            if version == 4:
                parts.append(("signature", int.from_bytes(raw[1580:1584], "little")))
        else:
            layer["completeness"] = "unsupported"
            layer["reasons"].append("Android boot header version not qualified (v0, v3, v4 only)")
            engine.unexplained(layer, 0, view.length, "unsupported boot header variant")
            return
        offset = page
        if view.length < page or not any(size for _, size in parts):
            raise engine.fail("corrupt_image")
        for name, size in parts:
            padded = (size + page - 1) // page * page
            if offset + padded > view.length:
                raise engine.fail("corrupt_image")
            if size:
                child = view.sub(offset, size)
                # Keep genuine component bytes as well as any decoded tree.
                file = engine.add_file(engine.join(prefix, name), child)
                engine.nested_file(file, engine.join(prefix, name), layer, depth)
            offset += padded
        engine.unexplained(layer, offset, view.length - offset, "bytes after boot components")
        layer["confidence"] = "validated"
        layer["reasons"].append("Boot components only; no boot signature verification or execution")


class Fit(Handler):
    name = "fit"
    magics = (b"\xd0\x0d\xfe\xed",)

    def plan(self, engine, view):
        raw = view.read(0, 40)
        if len(raw) != 40:
            raise engine.fail("corrupt_image")
        _, total, structure, strings, reserve, version, compatible, _, strings_size, structure_size = struct.unpack(">10I", raw)
        if version != 17 or compatible > 17 or not 40 <= total <= view.length or strings_size > 1024**2 or structure_size > MAX_EXPANDED or reserve < 40 or reserve % 8:
            raise engine.fail("corrupt_image")
        if not 40 <= structure <= total - structure_size or not 40 <= strings <= total - strings_size or not (structure + structure_size <= strings or strings + strings_size <= structure):
            raise engine.fail("corrupt_image")
        return {"confidence": "validated", "offset": 0, "length": total}

    def extract(self, engine, view, layer, prefix, depth):
        raw = view.read(0, 40)
        if len(raw) != 40:
            raise engine.fail("corrupt_image")
        _, total, structure, strings, reserve, version, compatible, _, strings_size, structure_size = struct.unpack(">10I", raw)
        if version != 17 or compatible > 17 or not 40 <= total <= view.length or strings_size > 1024**2 or structure_size > MAX_EXPANDED or reserve < 40 or reserve % 8:
            raise engine.fail("corrupt_image")
        if not 40 <= structure <= total - structure_size or not 40 <= strings <= total - strings_size or not (structure + structure_size <= strings or strings + strings_size <= structure):
            raise engine.fail("corrupt_image")
        # Validate bounded memory reservation list, but never act on addresses.
        position = reserve
        while True:
            if position + 16 > min(structure, strings) or position - reserve > 65536:
                raise engine.fail("corrupt_image")
            pair = view.read(position, 16)
            position += 16
            if not any(pair):
                break
        names = view.read(strings, strings_size)
        nodes, stack, position, finished, tokens = {}, [], structure, False, 0
        while position < structure + structure_size:
            tag = int.from_bytes(view.read(position, 4), "big")
            position += 4
            tokens += 1
            if tokens > 40000:
                raise engine.fail("entry_limit")
            if tag == 1:
                name_raw = view.read(position, min(241, structure + structure_size - position))
                end = name_raw.find(b"\0")
                if end < 0 or len(stack) >= 32:
                    raise engine.fail("corrupt_image")
                name = name_raw[:end].decode("utf-8", "strict")
                if stack:
                    engine.core.safe_name(name)
                    if "/" in name:
                        raise engine.fail("unsafe_path")
                elif name or nodes:
                    raise engine.fail("corrupt_image")
                stack.append(name)
                key = tuple(stack)
                if key in nodes:
                    raise engine.fail("corrupt_image")
                nodes[key] = {}
                position += (end + 1 + 3) & ~3
            elif tag == 2:
                if not stack:
                    raise engine.fail("corrupt_image")
                stack.pop()
            elif tag == 3:
                if not stack or position + 8 > structure + structure_size:
                    raise engine.fail("corrupt_image")
                length, name_offset = struct.unpack(">2I", view.read(position, 8))
                position += 8
                end = names.find(b"\0", name_offset)
                if end < name_offset or name_offset >= len(names) or length > structure + structure_size - position:
                    raise engine.fail("corrupt_image")
                name = names[name_offset:end].decode("ascii", "strict")
                properties = nodes[tuple(stack)]
                if name in properties or len(properties) > 256:
                    raise engine.fail("corrupt_image")
                properties[name] = view.sub(position, length)
                position += (length + 3) & ~3
            elif tag == 4:
                continue
            elif tag == 9 and not stack:
                finished = True
                break
            else:
                raise engine.fail("corrupt_image")
        if not finished:
            raise engine.fail("corrupt_image")
        count = 0
        for key, properties in sorted(nodes.items()):
            if len(key) != 3 or key[1] != "images":
                continue
            data = properties.get("data")
            if data is None:
                layer["reasons"].append("External FIT data reference not qualified; payload not fetched or inferred")
                continue
            count += 1
            for hash_key, values in nodes.items():
                if len(hash_key) != 4 or hash_key[:3] != key or not hash_key[3].startswith("hash"):
                    continue
                if "algo" not in values or "value" not in values:
                    raise engine.fail("corrupt_image")
                algo = values["algo"].read(0, 32)
                expected = values["value"].read(0, 65)
                if algo == b"sha256\0":
                    actual = bytes.fromhex(data.digest())
                elif algo == b"crc32\0":
                    checksum = 0
                    for chunk in data.chunks():
                        checksum = binascii.crc32(chunk, checksum)
                    actual = struct.pack(">I", checksum)
                else:
                    layer["reasons"].append("FIT hash algorithm not qualified")
                    continue
                if actual != expected:
                    raise engine.fail("corrupt_image")
            name = engine.join(prefix, key[-1])
            file = engine.add_file(name, data)
            engine.nested_file(file, name, layer, depth)
        layer["confidence"] = "validated"
        layer["reasons"].append("FIT component view; signatures and boot configuration trust are not verified")
        if not count:
            layer["completeness"] = "unsupported"
            engine.unexplained(layer, 0, view.length, "FDT without qualified inline FIT payloads")
        engine.unexplained(layer, total, view.length - total, "bytes outside FDT declared size")


class Squashfs(Handler):
    name = "squashfs"
    magics = (b"hsqs",)

    def plan(self, engine, view):
        region = engine.core.squashfs_region(view.path, view.offset, view.offset + view.length)
        if region["offset"] != view.offset:
            raise engine.fail("corrupt_image")
        return {"confidence": "validated", "offset": 0, "length": region["length"], "region": region}

    def extract(self, engine, view, layer, prefix, depth):
        plan = self.plan(engine, view)
        region = plan["region"]
        engine.use_tool("/usr/bin/unsquashfs", engine.core.TOOLS["/usr/bin/unsquashfs"], "unsquashfs version 4.7.5", ["-version"])
        engine.serial += 1
        work = engine.workspace / f"squashfs-{engine.serial:06d}"
        root = work / "tree"
        entries, total = engine.core.extract_squashfs(view.path, region, root, work, engine.staged_bytes + engine.tree_bytes, len(engine.entries))
        files = []
        for entry in entries:
            name = engine.join(engine.contents(prefix), entry["path"])
            if entry["kind"] == "file":
                file = engine.adopt_file(name, root / entry["path"], entry.get("mode", 0o600))
                files.append((name, file))
            elif entry["kind"] == "directory":
                engine.add_directory(name, entry.get("mode", 0o755))
            elif entry["kind"] == "symlink_metadata":
                # unsquashfs excluded the link before writing the tree. Retain
                # only targets that remain within this filesystem's root.
                try:
                    engine.add_link(name, entry["linkTarget"], entry.get("mode", 0), archive_root=engine.contents(prefix))
                except engine.fail as error:
                    if str(error) != "unsafe_link":
                        raise
                    engine.parents(name)
                    engine.claim(name, "special_omitted")
                    engine.entries[name] = {"path": name, "kind": "special_omitted", "size": 0, "sha256": None, "linkTarget": None, "mode": 0, "offset": 0}
                    engine.budget()
                    layer["reasons"].append("Unsafe link target omitted; no link created or target exposed")
            else:
                # Special entries were excluded before unsquashfs extraction.
                engine.parents(name)
                engine.claim(name, entry["kind"])
                engine.entries[name] = {**entry, "path": name, "mode": entry.get("mode", 0), "offset": 0}
                engine.budget()
        layer["decodedBytes"] = total
        layer["reasons"].append("Filesystem metadata, journals/slack, xattrs and omitted links are not reconstructed")
        if region["length"] < view.length:
            engine.unexplained(layer, region["length"], view.length - region["length"], "bytes outside declared SquashFS filesystem")
        engine.filesystems.append({"format": "squashfs", "offset": layer["offset"], "length": region["length"], "compression": region["compression"]})
        for name, file in files:
            engine.nested_file(file, name, layer, depth)


class Trx(Handler):
    name = "trx"
    magics = (b"HDR0",)

    def extract(self, engine, view, layer, prefix, depth):
        path = engine.materialize(view) if view.offset or view.length != view.path.stat().st_size else view.path
        try:
            info = engine.core.trx_header(path)
            if not Path("/otrx").is_file():
                layer["completeness"] = "unsupported"
                layer["reasons"].append("Qualified OpenWrt otrx parser unavailable")
                engine.unexplained(layer, 0, view.length, "TRX parser unavailable")
                return
            engine.use_tool("/otrx", engine.core.OTRX_BINARY_SHA, None)
            engine.core.run_tool(["/otrx", "check", str(path)])
            layer["confidence"] = "validated"
            engine.trx_info = info if layer["parentId"] is None else engine.trx_info
            parts = info["partitionOffsets"]
            for index, offset in enumerate(parts):
                end = parts[index + 1] if index + 1 < len(parts) else info["length"]
                child_view = view.sub(offset, end - offset)
                if child_view.read(0, 4) == b"hsqs":
                    child_prefix = prefix  # Preserve the established direct root tree.
                else:
                    child_prefix = engine.join(prefix, f"partition-{index}")
                engine.walk(child_view, layer, child_prefix, depth + 1, "source", offset)
            if info["length"] < view.length:
                engine.unexplained(layer, info["length"], view.length - info["length"], "bytes after TRX declared length")
        finally:
            if path != view.path:
                engine.drop_stage(path)


class Tar(Handler):
    name = "tar"
    magics = ()

    def detect(self, view):
        return {"confidence":"signature", "offset":0} if view.length >= 512 and view.read(257, 5) == b"ustar" else None

    def extract(self, engine, view, layer, prefix, depth):
        stage = engine.materialize(view)
        files = []
        count = 0
        try:
            with tarfile.open(stage, mode="r:") as archive:
                for member in archive:
                    count += 1
                    if count > engine.core.MAX_ENTRIES:
                        raise engine.fail("entry_limit")
                    name = member.name.rstrip("/")
                    if name.startswith("./"):
                        name = name[2:]
                    engine.core.safe_name(name)
                    destination = engine.join(prefix, name)
                    if member.mode & 0o6000:
                        raise engine.fail("unsafe_mode")
                    if member.isdir():
                        engine.add_directory(destination, member.mode, explicit=True)
                    elif member.isfile() and member.sparse is None:
                        with archive.extractfile(member) as source:
                            file = engine.add_stream(destination, source, member.size, member.mode)
                        files.append((destination, file))
                    else:
                        raise engine.fail("unsafe_link" if member.islnk() or member.issym() else "unsafe_special_file")
                end = archive.offset
                if view.read(end, 1024) != bytes(1024):
                    raise engine.fail("corrupt_image")
                cursor = end + 1024
                for chunk in view.chunks(cursor):
                    if any(chunk):
                        first = next(i for i, value in enumerate(chunk) if value)
                        engine.unexplained(layer, cursor + first, view.length - cursor - first, "bytes after TAR terminator and padding")
                        break
                    cursor += len(chunk)
            for name, file in files:
                engine.nested_file(file, name, layer, depth)
            layer["confidence"] = "validated"
        except tarfile.TarError:
            raise engine.fail("corrupt_image") from None
        finally:
            engine.drop_stage(stage)


class Zip(Handler):
    name = "zip"
    magics = (b"PK\x03\x04", b"PK\x05\x06")

    def extract(self, engine, view, layer, prefix, depth):
        # zipfile reads only this bounded materialized view. It never extracts
        # paths, follows links or executes a member; data is streamed to flat files.
        stage = engine.materialize(view)
        files = []
        try:
            # Ambiguous end markers, including those in comments, fail closed.
            # This also rejects ZIP64 until its boundaries are independently qualified.
            tail_offset = max(0, view.length - (65535 + 22))
            tail = view.read(tail_offset, view.length - tail_offset)
            if tail.count(b"PK\x05\x06") != 1:
                raise engine.fail("corrupt_image")
            end_offset = tail_offset + tail.index(b"PK\x05\x06")
            end = view.read(end_offset, 22)
            if len(end) != 22:
                raise engine.fail("corrupt_image")
            fields = struct.unpack("<4s4H2LH", end)
            if any(value == 0xffff for value in fields[1:5]) or any(value == 0xffffffff for value in fields[5:7]):
                raise engine.fail("unsupported_format")
            archive_end = end_offset + 22 + fields[7]
            if archive_end > view.length:
                raise engine.fail("corrupt_image")
            with zipfile.ZipFile(stage) as archive:
                if len(archive.infolist()) > engine.core.MAX_ENTRIES:
                    raise engine.fail("entry_limit")
                seen = set()
                for info in archive.infolist():
                    name = info.filename.rstrip("/")
                    engine.core.safe_name(name)
                    folded = unicodedata.normalize("NFC", name).casefold()
                    if folded in seen:
                        raise engine.fail("colliding_paths")
                    seen.add(folded)
                    mode = info.external_attr >> 16
                    if info.flag_bits & 1:
                        layer["reasons"].append("Encrypted ZIP member omitted")
                        layer["completeness"] = "partial"
                        continue
                    if mode & 0o6000 or stat.S_IFMT(mode) not in (0, stat.S_IFREG, stat.S_IFDIR):
                        raise engine.fail("unsafe_link" if stat.S_ISLNK(mode) else "unsafe_mode")
                    destination = engine.join(prefix, name)
                    if info.is_dir():
                        engine.add_directory(destination, mode & 0o777 or 0o755, explicit=True)
                    else:
                        if info.file_size > MAX_FILE or info.file_size > info.compress_size * MAX_RATIO + 1024**2:
                            raise engine.fail("output_limit")
                        with archive.open(info) as source:
                            file = engine.add_stream(destination, source, info.file_size, mode & 0o777 or 0o600)
                            if source.read(1):
                                raise engine.fail("corrupt_image")
                        files.append((destination, file))
                layer["confidence"] = "validated"
            engine.unexplained(layer, archive_end, view.length - archive_end, "bytes after ZIP end record and comment")
            for name, path in files:
                engine.nested_file(path, name, layer, depth)
        except (zipfile.BadZipFile, RuntimeError, NotImplementedError):
            raise engine.fail("corrupt_image") from None
        finally:
            engine.drop_stage(stage)


class Cramfs(Handler):
    name = "cramfs"
    magics = (b"\x45\x3d\xcd\x28", b"\x28\xcd\x3d\x45")

    def plan(self, engine, view):
        raw = view.read(0, 76)
        if len(raw) != 76:
            raise engine.fail("corrupt_image")
        endian = "big" if raw[:4] == self.magics[1] else "little"
        size = int.from_bytes(raw[4:8], endian)
        if not 76 <= size <= view.length:
            raise engine.fail("corrupt_image")
        return {"confidence":"validated", "offset":0, "length":size}

    def extract(self, engine, view, layer, prefix, depth):
        size = self.plan(engine, view)["length"]
        raw = view.read(0, 76)
        flags = int.from_bytes(raw[8:12], "little")
        if raw[:4] != self.magics[0] or flags & ~0x103 or not flags & 1:
            layer["completeness"] = "unsupported"
            layer["reasons"].append("CramFS requires little-endian fsid-v2 with basic/holes flags; extended pointers and shifted roots are unqualified")
            engine.unexplained(layer, 0, view.length, "unqualified CramFS variant")
            return
        if raw[16:32] != b"Compressed ROMFS" or any(raw[12:16]):
            raise engine.fail("corrupt_image")
        binary = "/usr/sbin/fsck.cramfs"
        pin = engine.core.OPTIONAL_TOOLS[binary]
        if json.loads(os.environ.get("OBTAINER_QUALIFIED_TOOLS", "{}")).get(binary) != pin:
            return super().extract(engine, view, layer, prefix, depth)
        engine.use_tool(binary, pin, "util-linux 2.41.3")
        crc = 0
        for offset in range(0, size, 65536):
            block = bytearray(view.read(offset, min(65536, size - offset)))
            if offset == 0:
                block[32:36] = bytes(4)
            crc = binascii.crc32(block, crc)
        if crc != int.from_bytes(raw[32:36], "little"):
            raise engine.fail("corrupt_image")
        source = engine.materialize(view.sub(0, size))
        try:
            try:
                engine.core.run_tool([binary, "-v", str(source)])  # no -x, no mount
            except engine.fail as error:
                if str(error) == "extraction_failed":
                    raise engine.fail("corrupt_image") from None
                raise
            def inode(offset):
                values = view.read(offset, 12)
                if len(values) != 12:
                    raise engine.fail("corrupt_image")
                a, b, c = struct.unpack("<3I", values)
                return a & 0xffff, b & 0xffffff, (c & 63) * 4, (c >> 6) * 4
            root_mode, root_size, _, root_offset = inode(64)
            if not stat.S_ISDIR(root_mode) or root_mode & 0o6000:
                raise engine.fail("corrupt_image")
            queue, visited, files, count = [(root_offset, root_size, "", 0)], set(), [], 1
            while queue:
                start, length, parent, level = queue.pop(0)
                if level > 32 or start in visited or (length and (start < 76 or start + length > size)):
                    raise engine.fail("corrupt_image")
                if not length:
                    continue
                visited.add(start)
                offset = start
                while offset < start + length:
                    mode, member_size, name_size, data_offset = inode(offset)
                    count += 1
                    if count > engine.core.MAX_ENTRIES or not name_size or offset + 12 + name_size > start + length or mode & 0o6000:
                        raise engine.fail("unsafe_mode" if mode & 0o6000 else "corrupt_image")
                    name_raw = view.read(offset + 12, name_size)
                    name = name_raw.rstrip(b"\0").decode("utf-8", "strict")
                    engine.core.safe_name(name)
                    if "/" in name:
                        raise engine.fail("unsafe_path")
                    relative = engine.join(parent, name)
                    destination = engine.join(prefix, relative)
                    if stat.S_ISDIR(mode):
                        engine.add_directory(destination, mode & 0o777, explicit=True)
                        queue.append((data_offset, member_size, relative, level + 1))
                    elif stat.S_IFMT(mode) in (stat.S_IFREG, stat.S_IFLNK):
                        if data_offset in visited and member_size:
                            raise engine.fail("unsafe_link")
                        if member_size:
                            visited.add(data_offset)
                        blocks = (member_size + 4095) // 4096
                        first = data_offset + blocks * 4
                        if member_size > MAX_FILE or (member_size and (data_offset < 76 or first > size)):
                            raise engine.fail("output_limit")
                        stage, total = engine.new_stage(), 0
                        try:
                            with stage.open("xb") as output:
                                for index in range(blocks):
                                    end = int.from_bytes(view.read(data_offset + index * 4, 4), "little")
                                    expected = min(4096, member_size - total)
                                    if end < first or end > size or end - first > 16384:
                                        raise engine.fail("corrupt_image")
                                    if end == first and flags & 0x100:
                                        data = bytes(expected)
                                    else:
                                        decoder = zlib.decompressobj()
                                        try:
                                            data = decoder.decompress(view.read(first, end - first), expected + 1)
                                        except zlib.error:
                                            raise engine.fail("corrupt_image") from None
                                        if len(data) != expected or not decoder.eof or decoder.unused_data:
                                            raise engine.fail("corrupt_image")
                                    engine.expanded(len(data), total + len(data), view.length)
                                    engine.stage_write(output, data)
                                    total += len(data)
                                    first = end
                            if stat.S_ISLNK(mode):
                                if member_size > 1024:
                                    raise engine.fail("unsafe_link")
                                engine.add_link(destination, stage.read_text(encoding="utf-8"), mode & 0o777, archive_root=prefix)
                                layer["reasons"].append("Link retained as inert metadata")
                            else:
                                engine.staged_bytes -= total
                                file = engine.adopt_file(destination, stage, mode & 0o777)
                                files.append((destination, file))
                        finally:
                            engine.drop_stage(stage)
                    else:
                        raise engine.fail("unsafe_special_file")
                    offset += 12 + name_size
            for name, file in files:
                engine.nested_file(file, name, layer, depth)
            layer["confidence"] = "validated"
            layer["decodedBytes"] = sum(engine.entries[name]["size"] for name, _ in files)
            layer["reasons"].append("CramFS ordinary file view; ownership and filesystem padding are not restored")
            engine.unexplained(layer, size, view.length - size, "bytes outside declared CramFS size")
        finally:
            engine.drop_stage(source)


class Ext(Handler):
    """Read-only e2fsprogs, inode-number commands only. Never mounts/replays."""
    name = "ext"
    magics = ()  # Superblock magic is at +1080, handled by scan's offset rule.

    def detect(self, view):
        return {"confidence":"signature", "offset":0} if view.length >= 2048 and view.read(1080, 2) == b"\x53\xef" else None

    def plan(self, engine, view):
        superblock = view.read(1024, 1024)
        inodes, blocks = struct.unpack_from("<2I", superblock)
        logarithm = int.from_bytes(superblock[24:28], "little")
        revision = int.from_bytes(superblock[76:80], "little")
        incompat = int.from_bytes(superblock[96:100], "little")
        if incompat & 0x80:
            blocks += int.from_bytes(superblock[336:340], "little") << 32
        if not inodes or not blocks or logarithm > 6 or revision > 1:
            raise engine.fail("corrupt_image")
        length = blocks * (1024 << logarithm)
        if not 2048 <= length <= view.length:
            raise engine.fail("corrupt_image")
        return {"confidence":"validated", "offset":0, "length":length}

    def extract(self, engine, view, layer, prefix, depth):
        debugfs, fsck = "/usr/sbin/debugfs", "/usr/sbin/e2fsck"
        allowed = json.loads(os.environ.get("OBTAINER_QUALIFIED_TOOLS", "{}"))
        if any(allowed.get(tool) != engine.core.OPTIONAL_TOOLS[tool] for tool in (debugfs, fsck)):
            return super().extract(engine, view, layer, prefix, depth)
        engine.use_tool(debugfs, allowed[debugfs], "debugfs 1.47.2", ["-V"])
        engine.use_tool(fsck, allowed[fsck], "e2fsck 1.47.2", ["-V"])
        source = engine.materialize(view) if view.offset or view.length != view.path.stat().st_size else view.path
        # -n is mandatory. A filesystem requiring journal recovery/correction
        # is refused, not silently repaired or treated as clean.
        try:
            try:
                engine.core.run_tool([fsck, "-f", "-n", str(source)])
            except engine.fail as error:
                if str(error) == "extraction_failed":
                    raise engine.fail("corrupt_image") from None
                raise
            visited, queue, files = {2}, [(2, "", 0)], []
            while queue:
                inode, parent, level = queue.pop(0)
                if level > 32 or len(visited) > engine.core.MAX_ENTRIES:
                    raise engine.fail("entry_limit")
                listing = engine.capture([debugfs, "-R", f"ls -p -l <{inode}>", str(source)])
                for line in listing.decode("utf-8", "strict").splitlines():
                    if not line:
                        continue
                    fields = line.split("/")
                    if len(fields) != 8 or fields[0] or fields[-1] or not fields[1].isdigit() or not re.fullmatch("[0-7]+", fields[2]):
                        raise engine.fail("corrupt_image")
                    number, mode, name = int(fields[1]), int(fields[2], 8), fields[5]
                    if name in (".", ".."):
                        continue
                    if not number:
                        continue
                    engine.core.safe_name(name)
                    if number in visited or mode & 0o6000:
                        raise engine.fail("unsafe_link" if number in visited else "unsafe_mode")
                    visited.add(number)
                    relative = engine.join(parent, name)
                    destination = engine.join(engine.contents(prefix), relative)
                    if stat.S_ISDIR(mode):
                        engine.add_directory(destination, mode & 0o777, explicit=True)
                        queue.append((number, relative, level + 1))
                    elif stat.S_ISREG(mode):
                        if not fields[6].isdigit():
                            raise engine.fail("corrupt_image")
                        size = int(fields[6])
                        if size > MAX_FILE:
                            raise engine.fail("output_limit")
                        attributes = engine.capture([debugfs, "-R", f"stat <{number}>", str(source)])
                        links = re.findall(rb"Links:\s+(\d+)\b", attributes)
                        if links != [b"1"]:
                            raise engine.fail("unsafe_link")
                        stage = engine.new_stage()
                        # Reserve peak output BEFORE the external parser writes.
                        engine.core.reserve_space(engine.staged_bytes + size, engine.tree_bytes, len(engine.entries) + 1)
                        engine.core.run_tool([debugfs, "-R", f"dump <{number}> {stage}", str(source)])
                        if not stage.is_file() or stage.stat().st_size != size:
                            raise engine.fail("corrupt_image")
                        file = engine.adopt_file(destination, stage, mode & 0o777)
                        files.append((destination, file))
                    else:
                        # No symlinks/devices are sent to a filesystem writer.
                        layer["reasons"].append("Non-ordinary ext inode omitted; no link or device node created")
                        engine.omitted_members.append({"layerId": layer["id"], "path": relative, "declaredBytes": 0, "reason": "non-ordinary inode"})
            for name, file in files:
                engine.nested_file(file, name, layer, depth)
            layer["confidence"] = "validated"
            layer["decodedBytes"] = sum(engine.entries[name]["size"] for name, _ in files)
            layer["reasons"].append("Read-only ext file view; journals, xattrs, deleted inodes and slack not reconstructed")
        finally:
            if source != view.path:
                engine.drop_stage(source)


class Ubi(Handler):
    """Conservative UBI v1 volume reader, not a UBIFS journal replay tool.

    Requires unique fixed geometry and contiguous, non-duplicated LEBs. No
    wear-level recovery, bad-block guessing or sequence arbitration is attempted.
    """
    name = "ubi"
    magics = (b"UBI#",)

    def extract(self, engine, view, layer, prefix, depth):
        if view.length < 4096:
            raise engine.fail("corrupt_image")
        candidates = []
        for shift in range(12, 21):
            size = 1 << shift
            blocks = view.length // size
            if not blocks:
                continue
            if (blocks > 1 and view.read(size, 4) == b"UBI#" and all(view.read(offset, 4) in (b"UBI#", b"\xff" * 4) for offset in range(0, blocks * size, size))) or (blocks == 1 and view.length == size):
                # A larger multiple can mask real headers. Require that no
                # aligned smaller geometry also qualifies (choose smallest).
                candidates.append(size)
        if not candidates:
            raise engine.fail("corrupt_image")
        size = min(candidates)
        volumes, geometry = {}, None
        end = view.length // size * size
        engine.unexplained(layer, end, view.length - end, "bytes outside complete UBI physical eraseblocks")
        for offset in range(0, end, size):
            ec = view.read(offset, 64)
            if ec == b"\xff" * 64:
                if any(chunk != b"\xff" * len(chunk) for chunk in view.chunks(offset, size)):
                    raise engine.fail("corrupt_image")
                engine.unexplained(layer, offset, size, "erased UBI physical eraseblock")
                continue
            if ec[:5] != b"UBI#\x01" or any(ec[5:8]) or any(ec[28:60]) or (binascii.crc32(ec[:60]) ^ 0xffffffff) != int.from_bytes(ec[60:], "big"):
                raise engine.fail("corrupt_image")
            vid_offset, data_offset, sequence = struct.unpack(">3I", ec[16:28])
            current = (vid_offset, data_offset, sequence)
            if (geometry is not None and geometry != current) or not 64 <= vid_offset < data_offset - 63 or data_offset >= size:
                raise engine.fail("corrupt_image")
            geometry = current
            vid = view.read(offset + vid_offset, 64)
            if vid == b"\xff" * 64:
                engine.unexplained(layer, offset, size, "unused UBI physical eraseblock")
                continue
            if vid[:5] != b"UBI!\x01" or vid[5] not in (1, 2) or vid[6] or any(vid[16:20] + vid[36:40] + vid[48:60]) or (binascii.crc32(vid[:60]) ^ 0xffffffff) != int.from_bytes(vid[60:], "big"):
                raise engine.fail("corrupt_image")
            volume, number = struct.unpack(">2I", vid[8:16])
            declared, used, pad, crc = struct.unpack(">4I", vid[20:36])
            if volume >= 0x7fffefff:
                engine.unexplained(layer, offset, size, "internal UBI volume metadata not reconstructed")
                continue
            if volume >= 128 or vid[7] or number >= 20000 or pad >= size - data_offset:
                raise engine.fail("corrupt_image")
            available = size - data_offset - pad
            length = declared if vid[5] == 2 else available
            if length > available or (vid[5] == 2 and (not used or number >= used)) or (vid[5] == 1 and (declared or used)):
                raise engine.fail("corrupt_image")
            data = view.sub(offset + data_offset, length)
            if vid[5] == 2:
                actual = 0
                for chunk in data.chunks():
                    actual = binascii.crc32(chunk, actual)
                if actual ^ 0xffffffff != crc:
                    raise engine.fail("corrupt_image")
            items = volumes.setdefault(volume, {})
            if number in items:
                raise engine.fail("ambiguous_image")
            items[number] = (data, used, vid[5])
        if not volumes:
            layer["reasons"].append("No qualified user volume found")
        for volume, items in sorted(volumes.items()):
            if sorted(items) != list(range(len(items))) or len({(used, kind) for _, used, kind in items.values()}) != 1:
                layer["reasons"].append(f"Volume {volume} has missing logical eraseblocks or mixed metadata; not reconstructed")
                for data, _, _ in items.values():
                    engine.unexplained(layer, data.offset - view.offset, data.length, "UBI volume mapping incomplete; no erased-byte inference")
                continue
            if next(iter(items.values()))[2] == 2 and next(iter(items.values()))[1] != len(items):
                raise engine.fail("corrupt_image")
            stage = engine.new_stage()
            total = 0
            try:
                with stage.open("xb") as output:
                    for number in sorted(items):
                        for chunk in items[number][0].chunks():
                            total += len(chunk)
                            engine.expanded(len(chunk), total, view.length)
                            engine.stage_write(output, chunk)
                engine.walk(View(stage), layer, engine.join(prefix, f"volume-{volume}"), depth + 1, "expanded", 0)
                layer["decodedBytes"] += total
            finally:
                engine.drop_stage(stage)
        layer["confidence"] = "validated"
        layer["reasons"].append("UBI volume bytes reconstructed; erase history, internal metadata and unallocated storage are not a filesystem dump")


class Rar5(Handler):
    """libarchive streaming reader. No archive_write/extract API is exposed.

    Select bounded ordinary members independently; an oversized package is not
    permission to expand all of it. Headers and omitted members remain explicit.
    """
    name = "rar5"
    magics = (b"Rar!\x1a\x07\x01\0",)
    library = "/usr/lib/x86_64-linux-gnu/libarchive.so.13.8.5"
    digest = "959dcce452c258ad7016f74b33b6df688cdad8a6caa1a625b1bfbebe85b332b0"

    def plan(self, engine, view):
        def vint(raw, position, limit=10):
            value = 0
            for index in range(limit):
                if position >= len(raw):
                    raise engine.fail("corrupt_image")
                byte = raw[position]
                position += 1
                value |= (byte & 127) << (7 * index)
                if not byte & 128:
                    if value > 0xffffffffffffffff:
                        raise engine.fail("corrupt_image")
                    return value,position
            raise engine.fail("corrupt_image")
        offset, count, main, ended, constraint = 8, 0, False, False, None
        members = {}
        while offset < view.length:
            raw = view.read(offset + 4, 3)
            length, length_bytes = vint(raw, 0, 3)
            end = offset + 4 + length_bytes + length
            if length < 2 or length > 2 * 1024**2 or end > view.length:
                raise engine.fail("corrupt_image")
            encoded = view.read(offset + 4, length_bytes + length)
            if binascii.crc32(encoded) != int.from_bytes(view.read(offset,4),"little"):
                raise engine.fail("corrupt_image")
            raw, position = encoded[length_bytes:], 0
            kind,position = vint(raw,position)
            flags,position = vint(raw,position)
            if flags & ~0x7f:
                raise engine.fail("corrupt_image")
            extra, packed = 0, 0
            if flags & 1:
                extra,position = vint(raw,position)
            if flags & 2:
                packed,position = vint(raw,position)
            if position + extra > len(raw) or end + packed > view.length:
                raise engine.fail("corrupt_image")
            if flags & 0x38:
                constraint = "Split/dependent RAR members are unqualified; no hidden expansion or volume inference"
            if kind == 4:
                return {"confidence":"validated","offset":0,"length":view.length,"constraint":"Encrypted RAR headers; no credentials or decoder attempted"}
            if kind == 1:
                if main or count:
                    raise engine.fail("corrupt_image")
                main = True
                archive_flags,position = vint(raw,position)
                if archive_flags & 7:
                    constraint = "Solid/multivolume RAR is unqualified for bounded member skipping; no hidden expansion attempted"
            elif kind in (2,3):
                if not main:
                    raise engine.fail("corrupt_image")
                file_flags,position = vint(raw,position)
                unpacked,position = vint(raw,position)
                _,position = vint(raw,position)  # attributes
                position += (4 if file_flags & 2 else 0) + (4 if file_flags & 4 else 0)
                compression,position = vint(raw,position)
                if compression & 0x40:
                    constraint = "Solid RAR member is unqualified for bounded skipping"
                if (compression & 63) != 0 or (128 * 1024 << ((compression >> 10) & 31)) > 64 * 1024**2 or file_flags & 8:
                    constraint = "RAR algorithm/window or unknown declared size exceeds qualified bounds"
                _,position = vint(raw,position)  # host OS; not an execution platform
                name_length,position = vint(raw,position)
                if not 0 < name_length <= 240 or position + name_length > len(raw) - extra:
                    raise engine.fail("unsafe_path")
                name = raw[position:position+name_length].decode("utf-8","strict").rstrip("/")
                if kind == 2:
                    engine.core.safe_name(name)
                    if name in members:
                        raise engine.fail("colliding_paths")
                    members[name] = {"unpacked":unpacked,"packed":packed,"directory":bool(file_flags & 1)}
            elif kind == 5:
                if not main:
                    raise engine.fail("corrupt_image")
                ended = True
                if end + packed != view.length:
                    constraint = "RAR trailing data not qualified for complete archive extraction"
                break
            elif not flags & 4:
                constraint = "Unknown mandatory RAR header"
            count += 1
            if count > engine.core.MAX_ENTRIES + 2:
                raise engine.fail("entry_limit")
            offset = end + packed
        if not ended:
            raise engine.fail("corrupt_image")
        return {"confidence":"validated","offset":0,"length":view.length,"constraint":constraint,"members":members}

    def extract(self, engine, view, layer, prefix, depth):
        plan = self.plan(engine, view)
        if plan["constraint"]:
            layer["completeness"] = "encrypted-or-unknown" if plan["constraint"].startswith("Encrypted") else "unsupported"
            layer["reasons"].append(plan["constraint"])
            engine.unexplained(layer,0,view.length,plan["constraint"])
            return
        if json.loads(os.environ.get("OBTAINER_QUALIFIED_TOOLS", "{}")).get(self.library) != self.digest:
            return super().extract(engine, view, layer, prefix, depth)
        import ctypes as c
        engine.use_tool(self.library, self.digest, None)
        lib = c.CDLL(self.library)
        def bind(name, restype, *argtypes):
            function = getattr(lib, name)
            function.restype, function.argtypes = restype, list(argtypes)
            return function
        version = bind("archive_version_string", c.c_char_p)()
        if version != b"libarchive 3.8.5":
            raise engine.fail("worker_unavailable")
        engine.tools[self.library]["version"] = version.decode("ascii")
        new = bind("archive_read_new", c.c_void_p)
        support = bind("archive_read_support_format_rar5", c.c_int, c.c_void_p)
        opened = bind("archive_read_open_filename", c.c_int, c.c_void_p, c.c_char_p, c.c_size_t)
        next_header = bind("archive_read_next_header", c.c_int, c.c_void_p, c.POINTER(c.c_void_p))
        pathname = bind("archive_entry_pathname_utf8", c.c_char_p, c.c_void_p)
        mode_of = bind("archive_entry_mode", c.c_uint, c.c_void_p)
        size_of = bind("archive_entry_size", c.c_int64, c.c_void_p)
        nlink = bind("archive_entry_nlink", c.c_uint, c.c_void_p)
        symlink = bind("archive_entry_symlink", c.c_char_p, c.c_void_p)
        hardlink = bind("archive_entry_hardlink", c.c_char_p, c.c_void_p)
        encrypted = bind("archive_entry_is_encrypted", c.c_int, c.c_void_p)
        read = bind("archive_read_data", c.c_ssize_t, c.c_void_p, c.c_void_p, c.c_size_t)
        skip = bind("archive_read_data_skip", c.c_int, c.c_void_p)
        free = bind("archive_read_free", c.c_int, c.c_void_p)
        source = engine.materialize(view) if view.offset or view.length != view.path.stat().st_size else view.path
        archive = new()
        if not archive:
            raise engine.fail("resource_exhausted")
        count, retained, omitted = 0, 0, 0
        files, seen = [], set()
        try:
            if support(archive) != 0 or opened(archive, os.fsencode(source), 65536) != 0:
                raise engine.fail("corrupt_image")
            member = c.c_void_p()
            buffer = c.create_string_buffer(65536)
            while True:
                result = next_header(archive, c.byref(member))
                if result == 1:  # ARCHIVE_EOF
                    break
                if result != 0:
                    raise engine.fail("corrupt_image")
                count += 1
                if count > engine.core.MAX_ENTRIES:
                    raise engine.fail("entry_limit")
                raw_name = pathname(member)
                if not raw_name:
                    raise engine.fail("unsafe_path")
                name = raw_name.decode("utf-8", "strict").rstrip("/")
                engine.core.safe_name(name)
                folded = unicodedata.normalize("NFC", name).casefold()
                if folded in seen:
                    raise engine.fail("colliding_paths")
                seen.add(folded)
                mode, size = mode_of(member), size_of(member)
                declared = plan["members"].get(name)
                if declared is None or declared["unpacked"] != size or declared["directory"] != stat.S_ISDIR(mode):
                    raise engine.fail("corrupt_image")
                if size < 0 or mode & 0o6000 or nlink(member) > 1 or symlink(member) or hardlink(member) or stat.S_IFMT(mode) not in (stat.S_IFREG, stat.S_IFDIR):
                    raise engine.fail("unsafe_mode")
                destination = engine.join(prefix, name)
                if stat.S_ISDIR(mode):
                    engine.add_directory(destination, mode & 0o777, explicit=True)
                elif encrypted(member) or size > MAX_FILE or retained + size > 64 * 1024**2 or size > declared["packed"] * MAX_RATIO + 1024**2:
                    omitted += 1
                    engine.omitted_members.append({"layerId": layer["id"], "path": name, "declaredBytes": size, "reason": "encrypted member" if encrypted(member) else "per-member or aggregate selection budget"})
                    if len(engine.omitted_members) > 4096:
                        raise engine.fail("entry_limit")
                else:
                    class Stream:
                        def read(self, amount):
                            result = read(archive, buffer, min(amount, 65536))
                            if result < 0:
                                raise engine.fail("corrupt_image")
                            return buffer.raw[:result]
                    file = engine.add_stream(destination, Stream(), size, mode & 0o777)
                    if read(archive, buffer, 1) != 0:
                        raise engine.fail("corrupt_image")
                    engine.expanded(size, retained + size, view.length)
                    retained += size
                    files.append((destination, file))
                if skip(archive) != 0:
                    raise engine.fail("corrupt_image")
            layer["confidence"] = "validated"
            layer["decodedBytes"] = retained
            if omitted:
                layer["reasons"].append(f"{omitted} RAR members omitted by bounded selection or encryption; archive is partial")
                engine.unexplained(layer, 0, view.length, "compressed member byte mapping unavailable; omitted members listed separately")
            for name, file in files:
                engine.nested_file(file, name, layer, depth)
        finally:
            free(archive)
            if source != view.path:
                engine.drop_stage(source)


class Unsupported(Handler):
    name = "unsupported"
    magics = (b"UBI#", b"\x31\x18\x10\x06", b"\x85\x19", b"\x19\x85", b"\x45\x3d\xcd\x28", b"\x28\xcd\x3d\x45", b"Rar!\x1a\x07\x01\0")

    def plan(self, engine, view):
        raw = view.read(0, 8)
        names = {b"UBI#": "ubi", b"\x31\x18\x10\x06": "ubifs", b"\x85\x19": "jffs2", b"\x19\x85": "jffs2", b"\x45\x3d\xcd\x28": "cramfs", b"\x28\xcd\x3d\x45": "cramfs", b"Rar!\x1a\x07\x01\0": "rar5"}
        return {"confidence": "signature", "offset": 0, "length": view.length, "format": next(value for magic, value in names.items() if raw.startswith(magic))}


HANDLERS = (Cpio(), Compression(), Squashfs(), Trx(), Ubi(), Cramfs(), Ext(), Uimage(), Fit(), AndroidBoot(), AndroidSparse(), Delta(), Tar(), Zip(), Rar5(), Unsupported())


class Engine:
    def __init__(self, core, workspace):
        self.core = core
        self.fail = core.Refusal
        self.workspace = Path(workspace)
        self.workspace.mkdir(parents=True, exist_ok=True)
        self.entries = {}
        self.files = {}
        self.folded = {}
        self.explicit_directories = set()
        self.layers = []
        self.unexplained_regions = []
        self.swupdate = []
        self.omitted_members = []
        self.expanded_seen = 0
        self.tree_bytes = 0
        self.staged_bytes = 0
        self.serial = 0
        self.tools = {}
        self.filesystems = []
        self.trx_info = None
        if "OBTAINER_QUALIFIED_TOOLS" in os.environ:
            self.use_tool("/usr/bin/python3.14",core.TOOLS["/usr/bin/python3.14"],"Python 3.14.4",["-I","--version"])

    def budget(self):
        if len(self.entries) > self.core.MAX_ENTRIES or self.tree_bytes > self.core.MAX_BYTES:
            raise self.fail("output_limit")
        self.core.reserve_space(self.staged_bytes, self.tree_bytes, len(self.entries))

    def join(self, prefix, name):
        result = prefix + "/" + name if prefix else name
        self.core.safe_name(result)
        return result

    def contents(self, prefix):
        if prefix and self.entries.get(prefix, {}).get("kind") == "file":
            return prefix + ".contents"
        return prefix

    def parents(self, name):
        parts = name.split("/")
        for i in range(1, len(parts)):
            self.add_directory("/".join(parts[:i]))

    def claim(self, name, kind):
        self.core.safe_name(name)
        folded = unicodedata.normalize("NFC", name).casefold()
        if folded in self.folded and (self.folded[folded] != name or name not in self.entries or self.entries[name]["kind"] != kind):
            raise self.fail("colliding_paths")
        self.folded[folded] = name

    def add_directory(self, name, mode=0o755, explicit=False):
        if not name:
            return
        self.parents(name)
        self.claim(name, "directory")
        if name in self.entries:
            if explicit and name in self.explicit_directories:
                raise self.fail("colliding_paths")
        else:
            self.entries[name] = {"path": name, "kind": "directory", "size": 0, "sha256": None, "linkTarget": None, "mode": mode, "offset": 0}
        if explicit:
            self.explicit_directories.add(name)
            self.entries[name]["mode"] = mode
        self.budget()

    def add_file(self, name, view, mode=0o600):
        with view.path.open("rb") as source:
            source.seek(view.offset)
            return self.add_stream(name, source, view.length, mode)

    def add_stream(self, name, source, length, mode=0o600):
        if length > MAX_FILE or length < 0 or mode & ~0o777:
            raise self.fail("output_limit" if length > MAX_FILE else "unsafe_mode")
        self.parents(name)
        self.claim(name, "file")
        if name in self.entries:
            raise self.fail("colliding_paths")
        self.serial += 1
        output_path = self.workspace / f"data-{self.serial:06d}"
        digest = hashlib.sha256()
        remaining = length
        with output_path.open("xb") as output:
            while remaining:
                chunk = source.read(min(remaining, 65536))
                if not chunk:
                    raise self.fail("corrupt_image")
                remaining -= len(chunk)
                self.tree_bytes += len(chunk)
                self.budget()
                output.write(chunk)
                digest.update(chunk)
        self.entries[name] = {"path": name, "kind": "file", "size": length, "sha256": digest.hexdigest(), "linkTarget": None, "mode": mode, "offset": 0}
        self.files[name] = output_path
        self.budget()
        return output_path

    def add_link(self, name, target, mode=0o777, archive_root=""):
        if not target or len(target.encode()) > 1024 or target.startswith(("/", "\\")) or any(unicodedata.category(c) in ("Cc", "Cf") for c in target) or "\\" in target or ":" in target:
            raise self.fail("unsafe_link")
        if archive_root and not name.startswith(archive_root + "/"):
            raise self.fail("unsafe_link")
        depth = len(name.split("/")) - 1 - (len(archive_root.split("/")) if archive_root else 0)
        for part in target.split("/"):
            depth += -1 if part == ".." else 0 if part in ("", ".") else 1
            if depth < 0:
                raise self.fail("unsafe_link")
        self.parents(name)
        self.claim(name, "symlink_metadata")
        if name in self.entries:
            raise self.fail("colliding_paths")
        self.entries[name] = {"path": name, "kind": "symlink_metadata", "size": 0, "sha256": None, "linkTarget": target, "mode": mode, "offset": 0}
        self.budget()

    def adopt_file(self, name, path, mode):
        meta = path.lstat()
        if not stat.S_ISREG(meta.st_mode) or meta.st_nlink != 1 or mode & 0o6000 or meta.st_size > MAX_FILE:
            raise self.fail("unsafe_mode")
        self.parents(name)
        self.claim(name, "file")
        if name in self.entries:
            raise self.fail("colliding_paths")
        self.serial += 1
        destination = self.workspace / f"data-{self.serial:06d}"
        if destination.exists():
            raise self.fail("colliding_paths")
        self.tree_bytes += meta.st_size
        self.budget()
        path.rename(destination)
        self.files[name] = destination
        self.entries[name] = {"path": name, "kind": "file", "size": meta.st_size, "sha256": self.core.digest(destination), "linkTarget": None, "mode": mode, "offset": 0}
        self.budget()
        return destination

    def use_tool(self, path, expected_hash, version_prefix, version_args=None):
        try:
            allowed = json.loads(os.environ.get("OBTAINER_QUALIFIED_TOOLS", "{}"))
            meta = Path(path).lstat()
            safe = stat.S_ISREG(meta.st_mode) and not meta.st_mode & (stat.S_ISUID | stat.S_ISGID | 0o022)
        except (ValueError, OSError):
            safe = False
            allowed = {}
        if not safe or allowed.get(path) != expected_hash or self.core.digest(Path(path)) != expected_hash:
            raise self.fail("worker_unavailable")
        version = "pinned read-only parser"
        if version_prefix is not None:
            result = subprocess.run([path] + (version_args or ["--version"]), stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=5, check=False)
            if len(result.stdout) > 8192 or version_prefix.encode() not in result.stdout:
                raise self.fail("worker_unavailable")
            version = version_prefix
        self.tools[path] = {"name": path, "version": version, "sha256": expected_hash}

    def new_stage(self):
        self.serial += 1
        return self.workspace / f"stage-{self.serial:06d}"

    def capture(self, command):
        stage = self.new_stage()
        try:
            with stage.open("xb") as output:
                self.core.run_tool(command, output, listing=True)
            if stage.stat().st_size > self.core.MAX_MANIFEST:
                raise self.fail("manifest_limit")
            return stage.read_bytes()
        finally:
            stage.unlink(missing_ok=True)  # Listing has its own 8 MiB reservation.

    def stage_write(self, output, chunk):
        self.staged_bytes += len(chunk)
        self.budget()
        output.write(chunk)

    def materialize(self, view):
        stage = self.new_stage()
        with stage.open("xb") as output:
            for chunk in view.chunks():
                self.stage_write(output, chunk)
        return stage

    def drop_stage(self, stage):
        if stage.exists():
            self.staged_bytes -= stage.stat().st_size
            stage.unlink()

    def expanded(self, size, member_size, input_size):
        self.expanded_seen += size
        if member_size > input_size * MAX_RATIO + 1024**2:
            raise self.fail("decompression_bomb")
        if self.expanded_seen > MAX_EXPANDED:
            raise self.fail("output_limit")

    def unexplained(self, layer, offset, length, reason):
        if length:
            if offset < 0 or length < 0 or offset + length > layer["length"]:
                raise self.fail("corrupt_image")
            self.unexplained_regions.append({"layerId": layer["id"], "offset": offset, "length": length, "reason": reason})
            if len(self.unexplained_regions) > 4096:
                raise self.fail("entry_limit")

    def swupdate_description(self, raw, layer):
        parsed = Libconfig(raw, self.fail).parse()
        declarations = []
        versions = []
        def visit(value, depth=0):
            if depth > 16:
                raise self.fail("description_limit")
            if isinstance(value, dict):
                if isinstance(value.get("version"), str):
                    versions.append(value["version"][:128])
                if isinstance(value.get("filename"), str):
                    declaration = {key: value[key][:512] for key in ("filename", "type", "sha256", "size", "name") if isinstance(value.get(key), str)}
                    properties = value.get("properties")
                    if isinstance(properties, dict):
                        declaration["baseRequirements"] = {key: text[:512] for key, text in properties.items() if isinstance(text, str) and ("base" in key.lower() or "hash" in key.lower() or "size" in key.lower())}
                    declarations.append(declaration)
                    if len(declarations) > 256:
                        raise self.fail("description_limit")
                    if any(term in str(value.get("type", "")).lower() for term in ("rdiff", "delta", "bsdiff")):
                        layer["reasons"].append("delta, base required; SWUpdate declaration, not reconstructed firmware")
                for child in value.values():
                    visit(child, depth + 1)
            elif isinstance(value, list):
                for child in value:
                    visit(child, depth + 1)
        visit(parsed)
        self.swupdate.append({"layerId": layer["id"], "versions": sorted(set(versions))[:32], "declarations": declarations, "authority": "package_declarations_unverified", "signatureVerified": False})

    def nested_file(self, path, name, layer, depth):
        view = View(path)
        handler = next((handler for handler in HANDLERS if handler.detect(view)),None)
        if handler:
            saved = self.checkpoint()
            try:
                self.walk(view, layer, name, depth + 1, "member", 0)
            except self.fail as error:
                if str(error) not in ("corrupt_image","unsafe_path","unsafe_link","unsafe_mode","unsafe_special_file","colliding_paths","corrupt_description"):
                    raise
                self.rollback(saved)
                blocked = {"id":f"layer-{len(self.layers):04d}","parentId":layer["id"],"space":"member","offset":0,"length":view.length,"sourcePath":name,"inputSha256":view.digest(),"handler":handler.name,"handlerVersion":handler.version,"format":handler.name,"confidence":"signature","decodedBytes":0,"completeness":"unsupported","reasons":[f"Nested layer rejected: {error}; outer ordinary file retained inert, no child output admitted"]}
                self.layers.append(blocked)
                self.unexplained(blocked,0,view.length,"nested decoder rejected input; no child bytes published")

    def checkpoint(self):
        # Decoder work already performed is never refunded. Discarding a bad
        # candidate releases its temporary files, not its global expansion cost.
        fields = ("entries", "files", "folded", "explicit_directories", "unexplained_regions", "swupdate", "omitted_members", "tree_bytes", "staged_bytes", "filesystems", "trx_info")
        return (set(self.workspace.iterdir()), {field:copy.deepcopy(getattr(self,field)) for field in fields},len(self.layers))

    def rollback(self, saved):
        existing, fields, count = saved
        for path in set(self.workspace.iterdir()) - existing:
            if path.is_dir() and not path.is_symlink():
                shutil.rmtree(path)  # Generated names in private workspace; no host paths.
            else:
                path.unlink()
        for field,value in fields.items():
            setattr(self,field,value)
        del self.layers[count:]

    def root_source(self, layer):
        while layer:
            if layer["space"] != "source":
                return False
            layer = next((item for item in self.layers if item["id"] == layer["parentId"]),None)
        return True

    def external_compression(self, view, layer, prefix, depth):
        name = layer["format"]
        pins = {"zstd": ("/usr/bin/zstd", "fe50fd600cad89b775dc5b4b10bc8ff95e2d33ddb4f7632966464b1fbdab8598", "v1.5.7"),
                "lz4": ("/usr/bin/lz4", "4be960d6f6b0d7ef69e01a9e1a056591c17b8687e9851db128018b2ac5f01da0", "v1.10.0")}
        binary, expected, version = pins[name]
        if json.loads(os.environ.get("OBTAINER_QUALIFIED_TOOLS", "{}")).get(binary) != expected:
            layer["completeness"] = "unsupported"
            layer["reasons"].append("Qualified external decompressor unavailable")
            self.unexplained(layer, 0, view.length, "external decoder unavailable")
            return
        self.use_tool(binary, expected, version)
        source = self.materialize(view)
        output = self.new_stage()
        total = 0
        child = subprocess.Popen([binary, "-d", "-c", "-q", "--", str(source)], stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        try:
            with output.open("xb") as destination:
                while chunk := child.stdout.read(65536):
                    total += len(chunk)
                    self.expanded(len(chunk), total, view.length)
                    self.stage_write(destination, chunk)
            if child.wait(timeout=5) != 0 or not total:
                raise self.fail("corrupt_image")
            layer["decodedBytes"] = total
            self.walk(View(output), layer, self.contents(prefix), depth + 1, "expanded", 0)
        finally:
            child.stdout.close()
            if child.poll() is None:
                child.kill()
                child.wait(timeout=5)
            self.drop_stage(source)
            self.drop_stage(output)

    def walk(self, view, parent=None, prefix="", depth=0, space="source", offset=0):
        if len(self.layers) >= MAX_LAYERS:
            raise self.fail("entry_limit")
        source_path = prefix or "<root>"
        prefix = self.contents(prefix)
        layer = {"id": f"layer-{len(self.layers):04d}", "parentId": parent["id"] if parent else None, "space": space, "offset": offset, "length": view.length, "sourcePath": source_path, "inputSha256": view.digest(), "handler": "unknown", "handlerVersion": 1, "format": "unknown", "confidence": "signature", "decodedBytes": 0, "completeness": "complete", "reasons": []}
        self.layers.append(layer)
        if depth > MAX_DEPTH:
            layer["completeness"] = "partial"
            layer["reasons"].append("layer depth limit reached")
            self.unexplained(layer, 0, view.length, "depth limit")
            return layer
        handler = next((handler for handler in HANDLERS if handler.detect(view)), None)
        if handler:
            plan = handler.plan(self, view)
            layer.update(handler=handler.name, handlerVersion=handler.version, format=plan.get("format", handler.name), confidence=plan["confidence"])
            handler.extract(self, view, layer, prefix, depth)
        else:
            layer["completeness"] = "encrypted-or-unknown"
            candidates, _ = scan(view, entropy=False)
            cursor = 0
            rejected = 0
            for position in sorted(set(candidate[0] for candidate in candidates)):
                if position == 0 or position < cursor:
                    continue
                child_view = view.sub(position)
                child_handler = next((item for item in HANDLERS if item.detect(child_view)), None)
                if child_handler is None or isinstance(child_handler, Unsupported):
                    continue
                try:
                    plan = child_handler.plan(self, child_view)
                except (self.fail, ValueError):
                    continue
                child_view = child_view.sub(0, plan["length"])
                # A magic hit in an opaque binary is only a hypothesis. Roll
                # back ALL speculative output on corrupt syntax. Safety and
                # budget refusals remain fatal, never skipped as near misses.
                saved = self.checkpoint()
                try:
                    child_prefix = "" if isinstance(child_handler,Squashfs) and not self.filesystems and self.root_source(layer) else self.join(prefix, f"carved-{position:08x}")
                    child = self.walk(child_view, layer, child_prefix, depth + 1, "source", position)
                except self.fail as error:
                    if str(error) != "corrupt_image":
                        raise
                    self.rollback(saved)
                    rejected += 1
                    continue
                self.unexplained(layer, cursor, position - cursor, "unrecognized wrapper bytes")
                cursor = position + child["length"]
            self.unexplained(layer, cursor, view.length - cursor, "encrypted, compressed or unknown bytes; encryption not established")
            if cursor:
                layer["completeness"] = "partial"
                layer["reasons"].append("unrecognized outer wrapper; candidate regions extracted independently")
            if rejected:
                layer["reasons"].append(f"{rejected} signature candidates rejected by structural validation")
        children = [child for child in self.layers if child["parentId"] == layer["id"]]
        layer["reasons"] = list(dict.fromkeys(layer["reasons"]))
        if layer["completeness"] == "complete" and (layer["reasons"] or any(child["completeness"] != "complete" for child in children) or any(region["layerId"] == layer["id"] for region in self.unexplained_regions)):
            layer["completeness"] = "partial"
        return handler.report(layer) if handler else layer


def scan(view, entropy=True):
    """Search every source byte; sample each bounded entropy-map window."""
    magics = tuple(dict.fromkeys(magic for handler in HANDLERS for magic in handler.magics))
    matches = []
    windows = []
    tail = b""
    position = 0
    for chunk in view.chunks():
        block = tail + chunk
        base = position - len(tail)
        for marker, relative, label in ((b"\x53\xef",1080,"ext-superblock"),(b"ustar",257,"ustar-header")):
            start = 0
            while True:
                found = block.find(marker, start)
                if found < 0:
                    break
                absolute = base + found
                if absolute >= relative and absolute + len(marker) > position and len(matches) < MAX_SCAN_MATCHES:
                    matches.append((absolute - relative, label))
                start = found + 1
        for magic in magics:
            start = 0
            while True:
                found = block.find(magic, start)
                if found < 0:
                    break
                absolute = base + found
                if absolute + len(magic) > position and len(matches) < MAX_SCAN_MATCHES:
                    matches.append((absolute, magic.hex()))
                start = found + 1
        if entropy and position % (1024**2) == 0 and len(windows) < MAX_ENTROPY_WINDOWS:
            counts = Counter(chunk)
            value = -sum((n / len(chunk)) * math.log2(n / len(chunk)) for n in counts.values()) if chunk else 0.0
            value = round(value,6)
            windows.append({"offset": position, "length": min(1024**2, view.length - position), "sampleBytes": len(chunk), "bitsPerByte": round(value, 6), "classification": "high_entropy_compressed_or_encrypted" if value >= 7.6 else "mixed_or_low_entropy"})
        tail = block[-31:]
        position += len(chunk)
    return sorted(set(matches)), windows


def extract(core, image, workspace):
    view = View(image)
    if not 0 < view.length <= core.MAX_INPUT:
        raise core.Refusal("invalid_input")
    engine = Engine(core, workspace)
    root = engine.walk(view)
    matches, entropy = scan(view)
    entries = sorted(engine.entries.values(), key=lambda entry: entry["path"])
    completeness = root["completeness"]
    if entries and completeness in ("unsupported", "encrypted-or-unknown"):
        completeness = "partial"
    source_hash = view.digest()
    root_unexplained = [{"offset": region["offset"], "length": region["length"]} for region in engine.unexplained_regions if region["layerId"] == root["id"]]
    region = engine.filesystems[0] if engine.filesystems else {"format": root["format"], "offset": 0, "length": view.length, "compression": 0}
    report = {"version": 2, "inputSha256": source_hash, "inputBytes": view.length, "container": "raw" if root["format"] == "squashfs" else root["format"], "expandedBytes": engine.expanded_seen or view.length,
              "region": region, "extractor": "OBTAINER layered engine 2", "toolSha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "unextractedRegions": root_unexplained, "entries": entries, "totalBytes": engine.tree_bytes, "fileCount": sum(entry["kind"] == "file" for entry in entries), "directoryCount": sum(entry["kind"] == "directory" for entry in entries),
              "filesystemRoots": ["/"], "workerExit": 0, "status": "extracted_verified" if completeness == "complete" else "partially_extracted" if engine.files else "inspection_only" if completeness == "partial" else "unsupported" if completeness == "unsupported" else "encrypted-or-unknown",
              "completeness": completeness, "layers": engine.layers, "unexplainedRegions": engine.unexplained_regions, "entropyMap": entropy, "scan": {"bytes": view.length, "allOffsetsScanned": True, "candidateLimit": MAX_SCAN_MATCHES, "candidatesRetained": len(matches), "candidatesCapped": len(matches) >= MAX_SCAN_MATCHES, "entropySampled": True}, "swupdate": engine.swupdate, "omittedMembers": engine.omitted_members,
              "provenance": {"engineVersion": VERSION, "engineSha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), "pythonVersion": ".".join(map(str, sys.version_info[:3])), "tools": sorted(engine.tools.values(), key=lambda item: item["name"])},
              "budgets": {"maxLayerDepth": MAX_DEPTH, "maxLayers": MAX_LAYERS, "expandedBytes": engine.expanded_seen, "maxExpandedBytes": MAX_EXPANDED, "maxFileBytes": MAX_FILE, "ratioLimit": MAX_RATIO},
              "limitations": ["No extracted code is executed. No device identity, physical acquisition, modification, rebuild or installation authority.", "Completeness describes decoded container/file coverage, not a full physical-device backup. High entropy does not prove encryption.", "Nested files use logical .contents directories. Links are inert metadata; ownership, xattrs, filesystem journals and slack are not restored."],
              "isolation": {"network": False, "hostDrives": False, "capabilities": "0000000000000000", "tmpfsBytes": core.TMPFS_BYTES, "spaceReserveBytes": core.SPACE_RESERVE, "rootReadOnly": True, "maxBytes": core.MAX_BYTES, "maxEntries": core.MAX_ENTRIES, "maxDepth": 32}}
    if engine.trx_info:
        report["containerInfo"] = engine.trx_info
    if len(json.dumps(report, sort_keys=True, separators=(",", ":")).encode()) > core.MAX_MANIFEST:
        raise core.Refusal("manifest_limit")
    return report, engine.files
