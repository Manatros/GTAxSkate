#!/usr/bin/env python3
"""Build a separate, sanitized bare Git repository; never change the source.

Rules (keep outside public repositories):
  {"replacements": [{"from": "private text", "to": "public text",
                     "whole_word": false}],
   "identities": ["Private Name <private@example.invalid>"],
   "public_identity": "Handle <123+Handle@users.noreply.github.com>"}

Pass every public branch/tag explicitly using repeated --ref. Only those refs
are created in the output. Binary/protected-file/path matches fail closed.
Exact, case-sensitive rules cannot discover private information not supplied.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import zlib


def git(repo, *args):
    return subprocess.check_output(["git", "-C", str(repo), *args])


class Sanitizer:
    def __init__(self, source, output, rules):
        self.source, self.output = Path(source), Path(output)
        if git(source, "rev-parse", "--show-object-format").strip() != b"sha1":
            raise ValueError("Only SHA-1 repositories are supported")
        self.pairs = [(r["from"], r["to"], bool(r.get("whole_word", False)))
                      for r in rules["replacements"]]
        if any(not old or old == new for old, new, _ in self.pairs):
            raise ValueError("Replacement sources must be nonempty and differ from targets")
        self.patterns = [(re.compile((r"(?<!\w)" if whole else "") + re.escape(old)
                                     + (r"(?!\w)" if whole else "")), new)
                         for old, new, whole in self.pairs]
        self.identities = {s.encode("utf-8") for s in rules["identities"]}
        self.public = rules["public_identity"].encode("utf-8")
        if not re.fullmatch(rb"[^<>\r\n]+ <[^<>\s]+>", self.public):
            raise ValueError("Invalid public identity")
        self.cache, self.written = {}, set()
        self.stats = {"commits": 0, "changed_blobs": 0, "identities": 0,
                      "removed_signatures": 0, "tags": 0}
        self.reader = None

    def read(self, oid):
        self.reader.stdin.write(oid.encode("ascii") + b"\n")
        self.reader.stdin.flush()
        header = self.reader.stdout.readline().split()
        if len(header) != 3:
            raise ValueError("Cannot read Git object")
        size = int(header[2])
        data = self.reader.stdout.read(size)
        if len(data) != size or self.reader.stdout.read(1) != b"\n":
            raise ValueError("Truncated Git object")
        return header[1].decode("ascii"), data

    def write(self, kind, data):
        raw = kind.encode("ascii") + b" " + str(len(data)).encode("ascii") + b"\0" + data
        oid = hashlib.sha1(raw).hexdigest()
        if oid not in self.written:
            path = self.output / "objects" / oid[:2] / oid[2:]
            path.parent.mkdir(exist_ok=True)
            # Output is newly created and exclusively owned by this invocation.
            with path.open("xb") as stream:
                stream.write(zlib.compress(raw))
            self.written.add(oid)
        return oid

    def contains_private(self, text):
        return any(pattern.search(text) for pattern, _ in self.patterns)

    def replace(self, text):
        for pattern, new in self.patterns:
            text = pattern.sub(lambda match: new, text)
        if self.contains_private(text):
            raise ValueError("Replacement rules leave private text behind")
        return text

    def identity(self, line):
        match = re.fullmatch(rb"(.+ <[^<>]+>)( -?\d+ [+-]\d{4})", line)
        if not match:
            raise ValueError("Malformed Git identity")
        identity, date = match.groups()
        if identity in self.identities:
            self.stats["identities"] += 1
            return self.public + date
        if self.contains_private(identity.decode("utf-8", errors="replace")):
            raise ValueError("Private identity lacks an exact identity rule")
        return line

    def blob(self, oid, protected):
        key = (oid, protected)
        if key in self.cache:
            return self.cache[key]
        kind, data = self.read(oid)
        if kind != "blob":
            raise ValueError("Expected blob")
        encoding = "utf-8"
        if data.startswith((b"\xff\xfe", b"\xfe\xff")):
            encoding = "utf-16"
        try:
            if encoding == "utf-8" and b"\0" in data:
                raise UnicodeError()
            text = data.decode(encoding)
        except UnicodeError:
            if any(old.encode("utf-8") in data or old.encode("utf-16-le") in data
                   or old.encode("utf-16-be") in data for old, _, _ in self.pairs):
                raise ValueError("Private text detected in binary/unsupported-encoding blob " + oid)
        else:
            if protected and self.contains_private(text):
                raise ValueError("Private text detected in protected attribution/license blob " + oid)
            if not protected:
                changed = self.replace(text)
                if changed != text:
                    data = changed.encode(encoding)
                    self.stats["changed_blobs"] += 1
        result = self.write("blob", data)
        self.cache[key] = result
        return result

    def tree(self, oid, protected=False):
        key = ("tree", oid, protected)
        if key in self.cache:
            return self.cache[key]
        kind, data = self.read(oid)
        if kind != "tree":
            raise ValueError("Expected tree")
        cursor, entries = 0, []
        while cursor < len(data):
            space = data.index(b" ", cursor)
            end = data.index(b"\0", space)
            mode, name = data[cursor:space], data[space + 1:end]
            old = data[end + 1:end + 21].hex()
            if len(data[end + 1:end + 21]) != 20:
                raise ValueError("Invalid tree entry")
            display = name.decode("utf-8", errors="replace")
            if self.contains_private(display):
                raise ValueError("Private text detected in a filename; rename before export")
            protect = protected or bool(re.match(
                r"^(license|licence|copying|notice|authors|contributors)([._-].*)?$", display, re.I))
            if mode == b"40000":
                new = self.tree(old, protect)
            elif mode == b"160000":
                new = old  # A gitlink names a commit belonging to another repository.
            else:
                new = self.blob(old, protect)
            entries.append(mode + b" " + name + b"\0" + bytes.fromhex(new))
            cursor = end + 21
        result = self.write("tree", b"".join(entries))
        self.cache[key] = result
        return result

    def metadata(self, data, tag=False):
        headers, message = data.split(b"\n\n", 1)
        # Continuation lines are part of the preceding header (signatures, etc.).
        groups = []
        for line in headers.split(b"\n"):
            if line.startswith(b" ") and groups:
                groups[-1] += b"\n" + line
            else:
                groups.append(line)
        result = []
        for line in groups:
            key, value = line.split(b" ", 1)
            if key in (b"gpgsig", b"gpgsig-sha256", b"mergetag"):
                self.stats["removed_signatures"] += 1
                continue
            if key == b"tree":
                value = self.tree(value.decode()).encode()
            elif key == b"parent":
                value = self.cache[("commit", value.decode())].encode()
            elif key == b"object" and tag:
                value = self.object(value.decode()).encode()
            elif key in (b"author", b"committer", b"tagger"):
                value = self.identity(value)
            elif self.contains_private(value.decode("utf-8", errors="replace")):
                raise ValueError("Private text detected in unsupported metadata header")
            result.append(key + b" " + value)
        if tag:
            # Annotated tag signatures are invalid after rewriting their target.
            for marker in (b"-----BEGIN PGP SIGNATURE-----", b"-----BEGIN SSH SIGNATURE-----"):
                if marker in message:
                    message = message.split(marker, 1)[0]
                    self.stats["removed_signatures"] += 1
        try:
            message = self.replace(message.decode("utf-8")).encode("utf-8")
        except UnicodeError:
            raise ValueError("Non-UTF-8 commit/tag message requires manual inspection") from None
        return b"\n".join(result) + b"\n\n" + message

    def object(self, oid):
        if ("commit", oid) in self.cache:
            return self.cache[("commit", oid)]
        if ("tag", oid) in self.cache:
            return self.cache[("tag", oid)]
        kind, data = self.read(oid)
        if kind == "tag":
            result = self.write("tag", self.metadata(data, tag=True))
            self.cache[("tag", oid)] = result
            self.stats["tags"] += 1
            return result
        raise ValueError("Selected ref must resolve to a commit or annotated commit tag")

    def run(self, refs):
        if self.output.exists():
            raise ValueError("Output must not exist; no existing repository will be overwritten")
        refs = list(dict.fromkeys(refs))
        old_refs = {}
        for ref in refs:
            if not ref.startswith(("refs/heads/", "refs/tags/")):
                raise ValueError("Only fully qualified branch/tag refs are accepted")
            subprocess.run(["git", "check-ref-format", ref], check=True, capture_output=True)
            if self.contains_private(ref):
                raise ValueError("Private text detected in ref name")
            old_refs[ref] = git(self.source, "show-ref", "--verify", "--hash", ref).decode().strip()
            git(self.source, "rev-parse", "--verify", ref + "^{commit}")
        commits = git(self.source, "rev-list", "--reverse", "--topo-order", *refs, "--").decode().splitlines()
        subprocess.run(["git", "init", "--bare", str(self.output)], check=True, capture_output=True)
        self.reader = subprocess.Popen(["git", "-C", str(self.source), "cat-file", "--batch"],
                                       stdin=subprocess.PIPE, stdout=subprocess.PIPE)
        try:
            for oid in commits:
                kind, data = self.read(oid)
                if kind != "commit":
                    raise ValueError("Expected commit")
                self.cache[("commit", oid)] = self.write("commit", self.metadata(data))
                self.stats["commits"] += 1
            mapping = {ref: {"old": oid, "new": self.object(oid)} for ref, oid in old_refs.items()}
            # Atomic ref transaction: a failed rewrite leaves no publishable refs.
            commands = "start\n" + "".join("create " + ref + " " + item["new"] + "\n"
                                           for ref, item in mapping.items()) + "prepare\ncommit\n"
            subprocess.run(["git", "-C", str(self.output), "update-ref", "--stdin"],
                           input=commands.encode(), check=True, capture_output=True)
            if "refs/heads/main" in mapping:
                git(self.output, "symbolic-ref", "HEAD", "refs/heads/main")
            git(self.output, "fsck", "--full", "--no-reflogs")
            return {"refs": mapping, "statistics": self.stats}
        finally:
            self.reader.stdin.close()
            self.reader.stdout.close()
            self.reader.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--rules", required=True)
    parser.add_argument("--ref", action="append", required=True, dest="refs")
    parser.add_argument("--map", required=True, dest="mapping")
    args = parser.parse_args()
    rules = json.loads(Path(args.rules).read_text(encoding="utf-8-sig"))
    result = Sanitizer(args.source, args.output, rules).run(args.refs)
    Path(args.mapping).write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result["statistics"]))


if __name__ == "__main__":
    main()
