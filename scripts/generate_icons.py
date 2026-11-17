#!/usr/bin/env python3
"""Generate deterministic PNG and Windows DIB-backed ICO assets using Python stdlib."""
from pathlib import Path
import math
import struct
import zlib

SIZE = 256
pixels = bytearray(SIZE * SIZE * 4)

def blend(x, y, color):
    i = (y * SIZE + x) * 4
    pixels[i:i + 4] = bytes(color)

def inside_round_rect(x, y, left, top, right, bottom, radius):
    cx = min(max(x, left + radius), right - radius)
    cy = min(max(y, top + radius), bottom - radius)
    return (x - cx) ** 2 + (y - cy) ** 2 <= radius ** 2

def line_distance(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    if dx == 0 and dy == 0:
        return math.hypot(px - ax, py - ay)
    t = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return math.hypot(px - (ax + t * dx), py - (ay + t * dy))

for y in range(SIZE):
    for x in range(SIZE):
        if inside_round_rect(x, y, 8, 8, 248, 248, 42):
            blend(x, y, (17, 38, 68, 255))
        else:
            blend(x, y, (0, 0, 0, 0))

# Cheque/document card.
for y in range(48, 207):
    for x in range(55, 202):
        if inside_round_rect(x, y, 55, 48, 202, 207, 16):
            blend(x, y, (247, 244, 233, 255))
for y in range(48, 78):
    for x in range(55, 202):
        if inside_round_rect(x, y, 55, 48, 202, 207, 16):
            blend(x, y, (210, 164, 76, 255))
for y, x0, x1 in [(103, 78, 177), (122, 78, 163), (141, 78, 174), (160, 78, 145)]:
    for yy in range(y - 3, y + 3):
        for xx in range(x0, x1):
            blend(xx, yy, (173, 180, 185, 255))

# Green confirmation check.
a, b, c = (135, 172), (153, 190), (190, 145)
for y in range(132, 201):
    for x in range(120, 204):
        if min(line_distance(x, y, *a, *b), line_distance(x, y, *b, *c)) <= 8:
            blend(x, y, (36, 145, 105, 255))

def png_bytes():
    raw = bytearray()
    for y in range(SIZE):
        raw.append(0)
        start = y * SIZE * 4
        raw.extend(pixels[start:start + SIZE * 4])
    def chunk(kind, data):
        return (struct.pack(">I", len(data)) + kind + data
                + struct.pack(">I", zlib.crc32(kind + data) & 0xffffffff))
    return (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">2I5B", SIZE, SIZE, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
            + chunk(b"IEND", b""))

def ico_bytes():
    # Windows ICO uses a BITMAPINFOHEADER, bottom-up BGRA XOR bitmap,
    # and a 1-bit AND transparency mask. The DIB height stores both planes.
    xor_bitmap = bytearray()
    and_mask = bytearray()
    mask_row_bytes = ((SIZE + 31) // 32) * 4
    for y in range(SIZE - 1, -1, -1):
        for x in range(SIZE):
            r, g, b, alpha = pixels[(y * SIZE + x) * 4:(y * SIZE + x) * 4 + 4]
            xor_bitmap.extend((b, g, r, alpha))
        row = bytearray(mask_row_bytes)
        for x in range(SIZE):
            alpha = pixels[(y * SIZE + x) * 4 + 3]
            if alpha == 0:
                row[x // 8] |= 1 << (7 - (x % 8))
        and_mask.extend(row)

    image_size = len(xor_bitmap) + len(and_mask)
    dib_header = struct.pack(
        "<IiiHHIIiiII",
        40, SIZE, SIZE * 2, 1, 32, 0, image_size, 0, 0, 0, 0
    )
    image = dib_header + xor_bitmap + and_mask
    directory = struct.pack("<HHH", 0, 1, 1)
    entry = struct.pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(image), 22)
    return directory + entry + image

out = Path("src-tauri/icons")
out.mkdir(parents=True, exist_ok=True)
(out / "icon.png").write_bytes(png_bytes())
(out / "icon.ico").write_bytes(ico_bytes())
print("Generated valid PNG and DIB-backed Windows ICO icons.")
