"""Create HungerStrike's original plate icon using only the Python standard library."""

from pathlib import Path
import struct
import zlib

size = 64
pixels = bytearray()
for y in range(size):
    pixels.append(0)  # PNG scanline filter
    for x in range(size):
        dx, dy = x - 32, y - 32
        radius = dx * dx + dy * dy
        if radius < 23 * 23:
            color = (226, 244, 201, 255)
        elif radius < 29 * 29:
            color = (27, 88, 65, 255)
        else:
            color = (19, 61, 48, 255)
        if 18 <= x <= 21 and 18 <= y <= 47 or 43 <= x <= 46 and 18 <= y <= 47:
            color = (27, 88, 65, 255)
        if 18 <= x <= 45 and 30 <= y <= 34:
            color = (27, 88, 65, 255)
        pixels.extend(color)

def chunk(name: bytes, payload: bytes) -> bytes:
    data = name + payload
    return struct.pack(">I", len(payload)) + data + struct.pack(">I", zlib.crc32(data))

png = (
    b"\x89PNG\r\n\x1a\n"
    + chunk(b"IHDR", struct.pack(">2I5B", size, size, 8, 6, 0, 0, 0))
    + chunk(b"IDAT", zlib.compress(bytes(pixels)))
    + chunk(b"IEND", b"")
)
output = Path(__file__).parents[1] / "src-tauri" / "icons"
output.mkdir(exist_ok=True)
(output / "icon.png").write_bytes(png)
ico = struct.pack("<3H", 0, 1, 1) + struct.pack("<4B2H2I", size, size, 0, 0, 1, 32, len(png), 22) + png
(output / "icon.ico").write_bytes(ico)
