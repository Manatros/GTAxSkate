"""Grind segments per Skate area in a baked tile set.

usage: segcheck.py TILES_DIR [LIMIT]
Skate builds one grind index per 5 x 5 tile area and holds at most 65,535
segments; over it the whole area fails and the previous collision stays
(2026-10-25 test). Exits 1 when the worst 5 x 5 window exceeds LIMIT
(default 52,000: room left for cars and props). Standard library only.
"""
import os
import re
import struct
import sys
import zlib


def rail_segments(path):
    b = open(path, 'rb').read()
    if b[:4] != b'SVWT':
        return 0  # not a tile (test fixtures); real tiles were verified natively before
    p = b if b[4:8] == (1).to_bytes(4, 'little') else zlib.decompress(b[8:])
    n = struct.unpack_from('<I', p, 20)[0]
    off = 24 + n
    nt = struct.unpack_from('<I', p, off)[0]
    off += 4 + nt * 40
    nr = struct.unpack_from('<I', p, off)[0]
    off += 4
    segs = 0
    for _ in range(nr):
        k = struct.unpack_from('<I', p, off)[0]
        off += 4 + k * 12
        segs += max(k - 1, 0)
    return segs


def main():
    tiles = sys.argv[1]
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else 52000
    counts = {}
    for name in os.listdir(tiles):
        m = re.fullmatch(r'(-?\d+)_(-?\d+)\.svwt', name)
        if m:
            counts[(int(m.group(1)), int(m.group(2)))] = rail_segments(os.path.join(tiles, name))
    worst = max(((sum(counts.get((x + i, y + j), 0) for i in range(-2, 3) for j in range(-2, 3)), (x, y))
                 for (x, y) in counts), default=(0, None))
    print(f'grind segments: {sum(counts.values())} total, worst tile {max(counts.values(), default=0)}, '
          f'worst 5x5 area {worst[0]} at {worst[1]} (limit {limit})')
    sys.exit(1 if worst[0] > limit else 0)


main()
