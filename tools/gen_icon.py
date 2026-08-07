#!/usr/bin/env python3
"""Generate App.ico (bell icon) with pure stdlib (zlib + struct)."""
import struct
import zlib


def make_png(size, pixel_fn):
    stride = size * 4
    raw = bytearray()
    for y in range(size):
        raw.append(0)
        for x in range(size):
            raw += bytes(pixel_fn(x, y, size))
    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        c += struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        return c
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", ihdr)
            + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
            + chunk(b"IEND", b""))


def in_round(x, y, size, r):
    cx = size / 2.0
    cy = size / 2.0
    dx = abs(x + 0.5 - cx) - (size / 2.0 - r)
    dy = abs(y + 0.5 - cy) - (size / 2.0 - r)
    if dx <= 0 or dy <= 0:
        return True
    return dx * dx + dy * dy <= r * r


def bell_hit(x, y, size):
    s = size / 64.0
    def px(v):
        return v * s
    cx, cy = 32.0 * s, 26.0 * s
    r = 9.0 * s
    if (x + 0.5 - cx) ** 2 + (y + 0.5 - cy) ** 2 <= r * r:
        return True
    if 19 * s <= x + 0.5 <= 45 * s and 27 * s <= y + 0.5 <= 40 * s:
        return True
    if 15 * s <= x + 0.5 <= 49 * s and 40 * s <= y + 0.5 <= 45 * s:
        return True
    cr = 3.0 * s
    if (x + 0.5 - 32 * s) ** 2 + (y + 0.5 - 46 * s) ** 2 <= cr * cr:
        return True
    return False


def pixel(x, y, size):
    bg = (11, 100, 242, 255)
    if not in_round(x, y, size, max(3, size / 4.0)):
        return (0, 0, 0, 0)
    if bell_hit(x, y, size):
        return (255, 255, 255, 255)
    return bg


def build_ico():
    sizes = [16, 32, 48, 64]
    entries = []
    blobs = []
    for sz in sizes:
        png = make_png(sz, pixel)
        entries.append(struct.pack("<BBBBIIII", sz, sz, 0, 0, 1, 32, len(png), 0))
        blobs.append(png)
    offset = 6 + 16 * len(sizes)
    data = b""
    for i, e in enumerate(entries):
        e = struct.pack("<BBBBIIII", e[0], e[1], e[2], e[3], e[4], e[5], e[6], offset)
        offset += len(blobs[i])
        data += e
    return b"\x00\x00\x01\x00" + data + b"".join(blobs)


with open("/workspace/Assets/App.ico", "wb") as f:
    f.write(build_ico())
print("App.ico written")
