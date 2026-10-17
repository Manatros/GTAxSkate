"""Read-only extraction of compiler references from the pinned official bundle.

Format: dotnet/runtime v6.0.36 Microsoft.NET.HostModel/Bundle/{Manifest,FileEntry}.cs.
Never executes the server or follows names from the manifest as output paths.
"""
import hashlib
import io
import json
from pathlib import Path
import struct
import zlib

ROOT = Path(__file__).resolve().parents[2]
SERVER = ROOT / "build/coop-lab/ragecoop-server/RageCoop.Server.exe"
OUTPUT = ROOT / "build/coop-lab/server-references"


def extract():
    raw = SERVER.read_bytes()
    if hashlib.sha256(raw).hexdigest() != "5c2e94778560ad5097bce6654c1b24083845af442954e2acc26727b6ddaafa07":
        raise ValueError("Server executable differs from reviewed official nightly")
    signature = bytes.fromhex("8b1202b96a612038727b930214d7a03213f5b9e6efae3318ee3b2dce24b36aae")
    marker = raw.find(signature)
    if marker < 8 or raw.find(signature, marker + 1) != -1:
        raise ValueError("Ambiguous/missing .NET bundle marker")
    start = struct.unpack_from("<q", raw, marker - 8)[0]
    if not 0 < start < len(raw) - 12:
        raise ValueError("Invalid bundle header")
    stream = io.BytesIO(raw)
    stream.seek(start)
    major, minor, count = struct.unpack("<III", stream.read(12))
    if (major, minor) != (6, 0) or not 0 < count < 1000:
        raise ValueError("Unsupported bundle version/count")

    def string():
        length = 0
        for shift in range(0, 35, 7):
            value = stream.read(1)
            if not value:
                raise ValueError("Truncated string")
            length |= (value[0] & 127) << shift
            if value[0] < 128:
                if length > 4096:
                    raise ValueError("Manifest string limit")
                return stream.read(length).decode("utf-8")
        raise ValueError("Invalid string length")

    string()  # opaque bundle ID
    stream.seek(40, 1)  # deps/runtimeconfig location and flags
    selected = {}
    for _ in range(count):
        offset, size, compressed = struct.unpack("<qqq", stream.read(24))
        kind = stream.read(1)[0]
        name = string()
        if name not in {"RageCoop.Server.dll", "RageCoop.Core.dll"}:
            continue
        if name in selected or kind != 1 or size < 1 or size > 16000000:
            raise ValueError("Invalid compiler reference")
        length = compressed or size
        if offset < 0 or length < 0 or offset + length > start:
            raise ValueError("Reference outside bundle data")
        payload = raw[offset:offset + length]
        if compressed:
            decoder = zlib.decompressobj(-15)
            payload = decoder.decompress(payload, size + 1)
            if not decoder.eof or decoder.unused_data:
                raise ValueError("Invalid compressed reference")
        if len(payload) != size or payload[:2] != b"MZ":
            raise ValueError("Reference length/PE mismatch")
        selected[name] = payload
    if "RageCoop.Server.dll" not in selected:
        raise ValueError("Server assembly missing")
    OUTPUT.mkdir(parents=True, exist_ok=True)
    report = {"serverExeSha256": hashlib.sha256(raw).hexdigest(), "references": {}}
    for name, payload in selected.items():
        target = OUTPUT / name
        if target.exists() and target.read_bytes() != payload:
            raise ValueError("Existing compiler reference differs")
        target.write_bytes(payload)
        report["references"][name] = hashlib.sha256(payload).hexdigest()
    (OUTPUT / "provenance.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    extract()
