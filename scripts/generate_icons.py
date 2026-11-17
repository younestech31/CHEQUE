#!/usr/bin/env python3
"""Generate deterministic Tauri PNG and Windows ICO assets using only Python stdlib."""
from pathlib import Path
import math
import struct
import zlib

SIZE = 256
pixels = bytearray(SIZE * SIZE * 4)

def blend(x, y, color):
    i = (y * SIZE + x) * 4
    pixels[i:i+4] = bytes(color)

def inside_round_rect(x, y, left, top, right, bottom, radius):
    cx = min(max(x, left + radius), right - radius)
    cy = min(max(y, top + radius), bottom - radius)
    return (x-cx)**2 + (y-cy)**2 <= radius**2

def line_distance(px, py, ax, ay, bx, by):
    dx, dy = bx-ax, by-ay
    if dx == 0 and dy == 0:
        return math.hypot(px-ax, py-ay)
    t = max(0.0, min(1.0, ((px-ax)*dx + (py-ay)*dy)/(dx*dx+dy*dy)))
    return math.hypot(px-(ax+t*dx), py-(ay+t*dy))

for y in range(SIZE):
    for x in range(SIZE):
        # Deep navy rounded tile with a restrained blue edge.
        if inside_round_rect(x, y, 8, 8, 248, 248, 42):
            blend(x, y, (17, 38, 68, 255))
        else:
            blend(x, y, (0, 0, 0, 0))

# Ivory cheque/document card.
for y in range(48, 207):
    for x in range(55, 202):
        if inside_round_rect(x, y, 55, 48, 202, 207, 16):
            blend(x, y, (247, 244, 233, 255))

# Gold header strip and ruled cheque lines.
for y in range(48, 78):
    for x in range(55, 202):
        if inside_round_rect(x, y, 55, 48, 202, 207, 16):
            blend(x, y, (210, 164, 76, 255))
for y, x0, x1 in [(103, 78, 177), (122, 78, 163), (141, 78, 174), (160, 78, 145)]:
    for yy in range(y-3, y+3):
        for xx in range(x0, x1):
            blend(xx, yy, (173, 180, 185, 255))

# A bold green confirmation mark at the lower right.
a, b, c = (135, 172), (153, 190), (190, 145)
for y in range(132, 201):
    for x in range(120, 204):
        d = min(line_distance(x, y, *a, *b), line_distance(x, y, *b, *c))
        if d <= 8:
            blend(x, y, (36, 145, 105, 255))

def png_bytes():
    raw = bytearray()
    for y in range(SIZE):
        raw.append(0)
        start = y * SIZE * 4
        raw.extend(pixels[start:start + SIZE * 4])
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xffffffff)
    return (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">2I5B", SIZE, SIZE, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
            + chunk(b"IEND", b""))

out = Path("src-tauri/icons")
out.mkdir(parents=True, exist_ok=True)
png = png_bytes()
(out / "icon.png").write_bytes(png)
# ICO directory entry; a PNG payload is supported by modern Windows icon loaders.
header = struct.pack("<HHH", 0, 1, 1)
entry = struct.pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(png), 22)
(out / "icon.ico").write_bytes(header + entry + png)
print("Generated src-tauri/icons/icon.png and icon.ico")
