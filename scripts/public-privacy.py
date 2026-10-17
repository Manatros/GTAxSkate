#!/usr/bin/env python3
"""Read-only publication scanner. Findings contain locations and classes, never values.

Private rules live in a separate UTF-8 file (one regular expression per line).
Blank lines and lines starting with # are ignored. Do not publish that rule file.
The scanner is a release gate, not a guarantee that old public copies disappear.
"""
import argparse
from collections import Counter
import io
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import zipfile


RULES = {
    "profile_path": r"(?i)(?:[a-z]:[\\/]+Users[\\/]+[^\\/\s\x00]+|/(?:Users|home)/[^/\s\x00]+)",
    "game_install_path": r"(?i)(?<![a-z0-9_])(?:[a-z]:[\\/]+(?:Games|SteamLibrary|XboxGames)[\\/]|[a-z]:[\\/]+[^\r\n\x00]{0,160}?(?:steamapps[\\/]+common|Rockstar Games)[\\/])",
    "private_key": r"-----BEGIN (?:RSA |EC |DSA |OPENSSH |ENCRYPTED )?PRIVATE KEY-----",
    "access_token": r"(?:gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|sk-(?:proj-)?[A-Za-z0-9_-]{32,}|AKIA[A-Z0-9]{16})",
}
RETAIL_EXTENSIONS = {".rpf", ".big", ".xex", ".bik", ".bk2", ".awc", ".iso"}
RETAIL_NAMES = {"gta5.exe", "gta5_enhanced.exe", "playgta5.exe", "default.xex"}
MAX_MEMBER_BYTES = 512 * 1024 * 1024
MAX_ARCHIVE_BYTES = 2 * 1024 * 1024 * 1024


def compile_rules(private_rules=None):
    rules = [(kind, re.compile(pattern)) for kind, pattern in RULES.items()]
    if private_rules:
        alternatives = []
        literal_keys = set()
        for line in Path(private_rules).read_text(encoding="utf-8-sig").splitlines():
            if line.strip() and not line.lstrip().startswith("#"):
                compiled = re.compile(line, re.IGNORECASE)
                # Captures/backreferences must keep their original numbering.
                if compiled.groups:
                    rules.append(("forbidden_identity", compiled))
                    continue
                flags, pattern = "i", line
                while (match := re.match(r"^\(\?([aiLmsux]+)\)", pattern)):
                    flags += "".join(c for c in match[1] if c not in flags)
                    pattern = pattern[match.end():]
                literal = re.sub(r"\\(.)", r"\1", pattern)
                if flags == "i" and literal.isascii() and re.escape(literal) == pattern:
                    key = literal.lower()
                    if key in literal_keys:
                        continue
                    literal_keys.add(key)
                alternatives.append(pattern if flags == "i" else "(?" + flags + ":" + pattern + ")")
        if alternatives:
            # One search preserves detection while avoiding hundreds of scans
            # of each binary/vendor source file. Counts are match occurrences,
            # not the number of overlapping private rules.
            rules.append(("forbidden_identity", re.compile("|".join(alternatives), re.IGNORECASE)))
    return rules


def scan_bytes(data, rules):
    """Examine ASCII/UTF-8 and both UTF-16 byte alignments without printing bytes."""
    counts = Counter()
    views = [data.decode("utf-8", errors="replace")]
    for encoding in ("utf-16-le", "utf-16-be"):
        for offset in (0, 1):
            views.append(data[offset:].decode(encoding, errors="ignore"))
    # The same token can occur in multiple decoding views; use the largest count.
    for kind, pattern in rules:
        counts[kind] += max((sum(1 for _ in pattern.finditer(view)) for view in views), default=0)
    return {kind: count for kind, count in counts.items() if count}


def path_findings(name):
    counts = {}
    normalized = name.replace("\\", "/")
    path = PurePosixPath(normalized)
    if (normalized.startswith("/") or re.match(r"^[a-zA-Z]:", normalized)
            or ".." in path.parts or "\x00" in name):
        counts["unsafe_archive_path"] = 1
    if path.suffix.lower() == ".pdb":
        counts["debug_symbols"] = 1
    if path.suffix.lower() in RETAIL_EXTENSIONS or path.name.lower() in RETAIL_NAMES:
        counts["retail_game_payload"] = 1
    return counts


def load_reviewed_exceptions(path):
    if not path:
        return {}
    document = json.loads(Path(path).read_text(encoding="utf-8"))
    if not isinstance(document, dict) or set(document) != {"format", "version", "exceptions"} or document["format"] != "reviewed-upstream-paths" or document["version"] != 1 or not isinstance(document["exceptions"], list):
        raise ValueError("invalid exceptions")
    result = {}
    for item in document["exceptions"]:
        if not isinstance(item, dict) or set(item) != {"path", "sha256", "classes", "reason", "upstream"}:
            raise ValueError("invalid exception")
        name = item["path"]
        if not isinstance(name, str) or not name or "\\" in name or PurePosixPath(name).as_posix() != name or path_findings(name) or ".git" in PurePosixPath(name).parts:
            raise ValueError("invalid exception path")
        if not isinstance(item["sha256"], str) or not re.fullmatch("[0-9a-f]{64}", item["sha256"]):
            raise ValueError("invalid exception hash")
        if not isinstance(item["classes"], list) or not item["classes"] or any(c not in {"profile_path", "game_install_path"} for c in item["classes"]) or len(set(item["classes"])) != len(item["classes"]):
            raise ValueError("unsafe exception class")
        if not isinstance(item["reason"], str) or len(item["reason"].strip()) < 20 or not isinstance(item["upstream"], str) or not re.fullmatch(r"https://[^\s]+", item["upstream"]):
            raise ValueError("missing exception evidence")
        if name in result:
            raise ValueError("duplicate exception")
        result[name] = item
    return result


def reviewed_content(data, name, rules, exceptions, applied):
    counts = scan_bytes(data, rules)
    item = (exceptions or {}).get(name)
    if item and hashlib.sha256(data).hexdigest() == item["sha256"]:
        removed = {kind: counts.pop(kind) for kind in item["classes"] if kind in counts}
        if removed and applied is not None:
            applied.append({"path": safe_location(name, rules), "classes": removed})
    return counts


def scan_zip(path, rules, exceptions=None, applied=None):
    findings = []
    with zipfile.ZipFile(path) as archive:
        total = 0
        for index, member in enumerate(archive.infolist(), 1):
            if member.is_dir():
                continue
            total += member.file_size
            counts = Counter(path_findings(member.filename))
            counts.update(scan_bytes(member.filename.encode("utf-8", errors="replace"), rules))
            # Unix symlinks in an archive are not accepted as release payloads.
            if (member.external_attr >> 16) & 0o170000 == 0o120000:
                counts["archive_symlink"] += 1
            if member.file_size > MAX_MEMBER_BYTES or total > MAX_ARCHIVE_BYTES:
                counts["archive_size_limit"] += 1
            elif member.flag_bits & 1:
                counts["encrypted_archive_member"] += 1
            else:
                data = archive.read(member)
                # Do not silently approve a compressed payload hidden one level deeper.
                if zipfile.is_zipfile(io.BytesIO(data)):
                    counts["nested_archive_requires_separate_scan"] += 1
                counts.update(reviewed_content(data, member.filename, rules, exceptions, applied))
            if counts:
                # Member names can themselves contain sensitive values.
                findings.append({"path": safe_location(member.filename, rules, index),
                                 "classes": dict(counts)})
    return findings


def safe_location(name, rules, index=None):
    if scan_bytes(name.encode("utf-8", errors="replace"), rules):
        return "[redacted location]" + (f" member #{index}" if index else "")
    return name


def scan_path(path, rules, exceptions=None, applied=None):
    path = Path(path)
    if path.is_file() and zipfile.is_zipfile(path):
        return scan_zip(path, rules, exceptions, applied)
    files = sorted(path.rglob("*")) if path.is_dir() else [path]
    findings = []
    for file in files:
        relative = file.relative_to(path).as_posix() if path.is_dir() else file.name
        if ".git" in PurePosixPath(relative).parts:
            continue
        if file.is_symlink():
            findings.append({"path": safe_location(relative, rules), "classes": {"payload_symlink": 1}})
            continue
        if not file.is_file():
            continue
        counts = Counter(path_findings(relative))
        counts.update(scan_bytes(relative.encode("utf-8"), rules))
        if zipfile.is_zipfile(file):
            nested = scan_zip(file, rules, exceptions, applied)
            for item in nested:
                item["path"] = safe_location(relative, rules) + "!" + item["path"]
            findings.extend(nested)
        else:
            counts.update(reviewed_content(file.read_bytes(), relative, rules, exceptions, applied))
        if counts:
            findings.append({"path": safe_location(relative, rules), "classes": dict(counts)})
    return findings


def scan_git(path, refs, rules, own_identity_pattern=None):
    identity = re.compile(own_identity_pattern, re.IGNORECASE) if own_identity_pattern else None
    result = subprocess.run(["git", "-C", str(path), "log", *refs,
                             "--format=%H%x00%an%x00%ae%x00%cn%x00%ce%x00%B%x00%x1e"],
                            check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    findings = []
    for entry in result.stdout.decode("utf-8", errors="replace").split("\x1e"):
        fields = entry.strip("\r\n").split("\x00")
        if len(fields) < 6:
            continue
        commit, author, email, committer, committer_email, message = fields[:6]
        counts = Counter(scan_bytes("\n".join(fields[1:6]).encode("utf-8"), rules))
        if identity:
            for name, address in ((author, email), (committer, committer_email)):
                if identity.search(name + " " + address):
                    if not re.fullmatch(r"[^@\s]+@users\.noreply\.github\.com", address, re.I):
                        counts["own_private_email"] += 1
        if counts:
            findings.append({"path": "git:" + commit[:12], "classes": dict(counts)})
    return findings


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--path", required=True, type=Path)
    parser.add_argument("--forbidden-patterns-file", type=Path)
    parser.add_argument("--reviewed-upstream-exceptions", type=Path, help="Exact path/hash exceptions for reviewed upstream example paths only.")
    parser.add_argument("--git-scan", action="store_true", help="Also scan commit metadata/messages (not historical trees).")
    parser.add_argument("--git-ref", action="append", default=None, help="Repeatable; defaults to HEAD.")
    parser.add_argument("--own-identity-pattern", help="Regex identifying own Git author/committer; other contributors remain allowed.")
    args = parser.parse_args(argv)
    try:
        if not args.path.exists():
            raise ValueError("missing input")
        rules = compile_rules(args.forbidden_patterns_file)
        exceptions = load_reviewed_exceptions(args.reviewed_upstream_exceptions)
        applied = []
        findings = scan_path(args.path, rules, exceptions, applied)
        if args.git_scan:
            findings.extend(scan_git(args.path, args.git_ref or ["HEAD"], rules, args.own_identity_pattern))
    except (OSError, ValueError, TypeError, KeyError, re.error, zipfile.BadZipFile, RuntimeError, subprocess.CalledProcessError):
        # Exception messages can disclose paths, private patterns, or Git stderr.
        print(json.dumps({"ok": False, "error": "Input could not be scanned completely."}))
        return 2
    print(json.dumps({"ok": not findings, "findings": findings, "applied_upstream_exceptions": applied, "applied_upstream_exception_count": len(applied)}, indent=2))
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
