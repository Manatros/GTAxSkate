#!/usr/bin/env python3
"""Synthetic, local tests for the publication privacy gate."""
import importlib.util
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import zipfile
from contextlib import redirect_stdout

(Path(__file__).resolve().parents[1] / "_test").mkdir(exist_ok=True)

spec = importlib.util.spec_from_file_location("public_privacy", Path(__file__).with_name("public-privacy.py"))
scanner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scanner)


def sample_profile_path():
    return "\\".join(("C:", "Users", "SyntheticPerson", ".cargo", "source.rs"))


class PrivacyTests(unittest.TestCase):
    def setUp(self):
        self.rules = scanner.compile_rules()

    def test_combined_private_rules_preserve_flags_and_backreferences(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "_test") as directory:
            path = Path(directory) / "patterns.txt"
            path.write_text("SyntheticPerson\n(?m)^PRIVATE_START\n(Synthetic)\\1\n", encoding="utf-8")
            rules = scanner.compile_rules(path)
            for value in (b"syntheticperson", b"prefix\nPRIVATE_START", b"SyntheticSynthetic"):
                self.assertIn("forbidden_identity", scanner.scan_bytes(value, rules))
            self.assertNotIn("forbidden_identity", scanner.scan_bytes(b"safe public content", rules))

    def test_ascii_and_utf16_binary_paths(self):
        value = sample_profile_path()
        for encoding in ("utf-8", "utf-16-le", "utf-16-be"):
            for prefix in (b"", b"!"):
                result = scanner.scan_bytes(prefix + value.encode(encoding) + b"\x00\x01", self.rules)
                self.assertIn("profile_path", result)

    def test_reviewed_exception_binds_path_hash_and_class(self):
        data = sample_profile_path().encode()
        item = {"path": "vendor/example.rs", "sha256": hashlib.sha256(data).hexdigest(), "classes": ["profile_path"], "reason": "Unchanged upstream synthetic example path.", "upstream": "https://example.org/source"}
        exceptions = {item["path"]: item}
        applied = []
        self.assertEqual(scanner.reviewed_content(data, item["path"], self.rules, exceptions, applied), {})
        self.assertEqual(len(applied), 1)
        self.assertIn("profile_path", scanner.reviewed_content(data + b"modified", item["path"], self.rules, exceptions, []))
        self.assertIn("profile_path", scanner.reviewed_content(data, "other/example.rs", self.rules, exceptions, []))
        private_rules = self.rules + [("forbidden_identity", scanner.re.compile("SyntheticPerson"))]
        self.assertIn("forbidden_identity", scanner.reviewed_content(data, item["path"], private_rules, exceptions, []))

    def test_reviewed_exception_config_rejects_unsafe_fields(self):
        item = {"path": "vendor/example.rs", "sha256": "a" * 64, "classes": ["profile_path"], "reason": "Unchanged upstream synthetic example path.", "upstream": "https://example.org/source"}
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "_test") as directory:
            path = Path(directory) / "exceptions.json"
            for change in ({"classes": [kind]} for kind in ("forbidden_identity", "access_token", "private_key", "retail_game_payload", "archive_symlink")):
                path.write_text(json.dumps({"format": "reviewed-upstream-paths", "version": 1, "exceptions": [item | change]}))
                with self.assertRaises(ValueError):
                    scanner.load_reviewed_exceptions(path)
            for change in ({"path": "../escape"}, {"path": "vendor/./example.rs"}, {"sha256": "invalid"}, {"upstream": ""}, {"reason": ""}, {"unknown": True}):
                path.write_text(json.dumps({"format": "reviewed-upstream-paths", "version": 1, "exceptions": [item | change]}))
                with self.assertRaises(ValueError):
                    scanner.load_reviewed_exceptions(path)

    def test_reviewed_zip_exception_preserves_symlink_detection(self):
        data = sample_profile_path().encode()
        item = {"path": "vendor/example.rs", "sha256": hashlib.sha256(data).hexdigest(), "classes": ["profile_path"]}
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "_test") as directory:
            path = Path(directory) / "source.zip"
            info = zipfile.ZipInfo(item["path"])
            info.create_system = 3
            info.external_attr = (0o120777 << 16)
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr(info, data)
            findings = scanner.scan_path(path, self.rules, {item["path"]: item}, [])
            self.assertEqual(findings[0]["classes"], {"archive_symlink": 1})

    def test_registry_keys_are_not_local_install_paths(self):
        registry = "HKLM:" + "\\SOFTWARE\\Rockstar Games\\GTAV"
        self.assertNotIn("game_install_path", scanner.scan_bytes(registry.encode(), self.rules))
        path = "D:" + "\\Program Files\\Rockstar Games\\GTAV"
        self.assertIn("game_install_path", scanner.scan_bytes(path.encode(), self.rules))

    def test_tokens_and_private_key(self):
        key_header = "-----BEGIN " + "OPENSSH " + "PRIVATE KEY-----"
        result = scanner.scan_bytes(("ghp_" + "a" * 36 + "\n" + key_header).encode(), self.rules)
        self.assertEqual(set(result), {"access_token", "private_key"})

    def test_generic_game_path(self):
        value = "\\".join(("D:", "Games", "Example")).encode()
        self.assertIn("game_install_path", scanner.scan_bytes(value, self.rules))

    def test_noreply_and_third_party_license_allowed(self):
        text = b"Copyright Third Party Author <author@example.org>\nMaintainer <handle@users.noreply.github.com>"
        self.assertEqual(scanner.scan_bytes(text, self.rules), {})

    def test_private_rule_and_redacted_location(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "_test") as directory:
            rules_file = Path(directory) / "rules.txt"
            rules_file.write_text("# local rules\nSyntheticPerson\n", encoding="utf-8")
            rules = scanner.compile_rules(rules_file)
            self.assertIn("forbidden_identity", scanner.scan_bytes(b"SyntheticPerson", rules))
            self.assertNotIn("SyntheticPerson", scanner.safe_location("SyntheticPerson.txt", rules))

    def test_zip_binary_unsafe_paths_symbols_retail(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "_test") as directory:
            archive = Path(directory) / "payload.zip"
            with zipfile.ZipFile(archive, "w") as z:
                z.writestr("runtime.dll", sample_profile_path().encode("utf-16-le"))
                z.writestr("../escape.txt", "safe")
                z.writestr("debug.pdb", "safe")
                z.writestr("update.rpf", "safe")
            findings = scanner.scan_path(archive, self.rules)
            classes = {kind for item in findings for kind in item["classes"]}
            self.assertEqual(classes, {"profile_path", "unsafe_archive_path", "debug_symbols", "retail_game_payload"})
            self.assertNotIn("SyntheticPerson", json.dumps(findings))

    def test_directory_and_exit_status(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "_test") as directory:
            root = Path(directory)
            (root / "LICENSE").write_text("Copyright Third Party Author", encoding="utf-8")
            with redirect_stdout(io.StringIO()):
                self.assertEqual(scanner.main(["--path", directory]), 0)
            (root / "bad.txt").write_text(sample_profile_path(), encoding="utf-8")
            output = io.StringIO()
            with redirect_stdout(output):
                self.assertEqual(scanner.main(["--path", directory]), 1)
            self.assertNotIn("SyntheticPerson", output.getvalue())

    def test_git_own_private_email_only(self):
        records = ("a" * 40 + "\0TestHandle\0handle@users.noreply.github.com\0TestHandle\0handle@users.noreply.github.com\0ok\0\x1e"
                   + "b" * 40 + "\0Third Party\0author@example.org\0Third Party\0author@example.org\0ok\0\x1e"
                   + "c" * 40 + "\0TestHandle\0handle@example.org\0TestHandle\0handle@example.org\0ok\0\x1e")
        with patch.object(subprocess, "run", return_value=subprocess.CompletedProcess([], 0, records.encode(), b"")):
            findings = scanner.scan_git(".", ["HEAD"], self.rules, "TestHandle")
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["classes"], {"own_private_email": 2})
        self.assertNotIn("example.org", json.dumps(findings))

    def test_error_never_prints_private_pattern(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "_test") as directory:
            rules = Path(directory) / "rules.txt"
            rules.write_text("SyntheticPerson(", encoding="utf-8")
            output = io.StringIO()
            with redirect_stdout(output):
                self.assertEqual(scanner.main(["--path", directory, "--forbidden-patterns-file", str(rules)]), 2)
            self.assertNotIn("SyntheticPerson", output.getvalue())

    def test_scanner_sources_are_clean_in_isolation(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "_test") as directory:
            for name in ("public-privacy.py", "test-public-privacy.py"):
                (Path(directory) / name).write_bytes(Path(__file__).with_name(name).read_bytes())
            self.assertEqual(scanner.scan_path(directory, self.rules), [])


if __name__ == "__main__":
    unittest.main()

