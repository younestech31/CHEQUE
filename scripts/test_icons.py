#!/usr/bin/env python3
"""Structural checks for generated Tauri PNG and Windows ICO assets."""
from pathlib import Path
import struct

root = Path("src-tauri/icons")
png = (root / "icon.png").read_bytes()
ico = (root / "icon.ico").read_bytes()

assert png.startswith(b"\x89PNG\r\n\x1a\n"), "PNG signature is invalid"
assert len(png) > 100, "PNG file is unexpectedly small"
reserved, kind, count = struct.unpack_from("<HHH", ico, 0)
assert (reserved, kind, count) == (0, 1, 1), "ICO directory header is invalid"
width, height, colors, reserved_entry, planes, bpp, size, offset = struct.unpack_from("<BBBBHHII", ico, 6)
assert (width, height, colors, reserved_entry, planes, bpp) == (0, 0, 0, 0, 1, 32), "ICO entry metadata is invalid"
assert offset == 22 and size == len(ico) - offset, "ICO payload bounds are invalid"
header_size, dib_width, dib_height, dib_planes, dib_bpp, compression, image_size = struct.unpack_from("<IiiHHII", ico, offset)
assert (header_size, dib_width, dib_height, dib_planes, dib_bpp, compression) == (40, 256, 512, 1, 32, 0), "ICO DIB header is invalid"
expected_xor = 256 * 256 * 4
expected_mask = ((256 + 31) // 32) * 4 * 256
assert image_size == expected_xor + expected_mask, "ICO bitmap plane sizes are invalid"
assert size == 40 + expected_xor + expected_mask, "ICO payload length is invalid"
print("Icon validation passed: PNG signature and Windows DIB ICO structure.")
