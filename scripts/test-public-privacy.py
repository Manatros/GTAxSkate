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
        slash = chr(92)
        spaced_unicode = "C:" + slash + "Users" + slash + "Rider Müller" + slash + "Videos" + slash + "capture.mp4"
        self.assertIn("profile_path", scanner.scan_bytes(spaced_unicode.encode("utf-16-le"), self.rules))

    def test_utf16_users_literal_does_not_join_unrelated_metadata(self):
        # The real profile component starts with a letter/number/underscore;
        # a leading combining mark is metadata, not a plausible username.
        decoy = b"!" + ("/users/\u0301abcdef\x00").encode("utf-16-le")
        self.assertNotIn("profile_path", scanner.scan_bytes(decoy, self.rules))

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

    def test_openai_token_shape_does_not_match_locale_table(self):
        locale_table = "af-NA af-ZA am-ET ar-AE as-IN az-AZ be-BY bg-BG bn-BD ca-ES cs-CZ da-DK de-DE el-GR en-US es-ES et-EE eu-ES fa-IR fi-FI fil-PH fr-FR ga-IE gl-ES gu-IN he-IL hi-IN hr-HR hu-HU hy-AM id-ID is-IS it-IT ja-JP kk-KZ km-KH kn-IN ko-KR lo-LA lt-LT lv-LV mk-MK ml-IN mn-MN mr-IN ms-MY mt-MT my-MM nb-NO ne-NP nl-NL nn-NO pa-IN pl-PL pt-BR ro-RO ru-RU sk-SK sl-SI sq-AL sr-RS sv-SE sw-KE ta-IN te-IN th-TH tr-TR uk-UA ur-PK uz-UZ vi-VN zh-CN"
        self.assertNotIn("access_token", scanner.scan_bytes(locale_table.encode(), self.rules))
        self.assertIn("access_token", scanner.scan_bytes(("sk-proj-" + "A_" * 24).encode(), self.rules))

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

    def test_binary_pk_signature_false_positive_still_scans_raw_bytes(self):
        # A managed DLL may contain PK signatures without being an extractable
        # nested archive. Keep raw-byte privacy checks while rejecting bad .zip files.
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "_test") as directory:
            payload = io.BytesIO()
            with zipfile.ZipFile(payload, "w") as archive:
                archive.writestr("payload.txt", "safe")
            malformed = bytearray(payload.getvalue())
            malformed[-6:-2] = (0x7FFFFFFF).to_bytes(4, "little")
            self.assertTrue(zipfile.is_zipfile(io.BytesIO(malformed)))
            dll = Path(directory) / "runtime.dll"
            dll.write_bytes(malformed + sample_profile_path().encode())
            findings = scanner.scan_path(dll, self.rules)
            self.assertEqual(findings[0]["classes"], {"unreadable_archive_candidate": 1, "profile_path": 1})

            bad_zip = Path(directory) / "broken.zip"
            bad_zip.write_bytes(malformed)
            with redirect_stdout(io.StringIO()):
                self.assertEqual(scanner.main(["--path", str(bad_zip)]), 2)

            folder_zip = Path(directory) / "folder-broken.zip"
            folder_zip.write_bytes(b"not a zip")
            with redirect_stdout(io.StringIO()):
                self.assertEqual(scanner.main(["--path", str(folder_zip)]), 2)

            unsupported = bytearray(payload.getvalue())
            local_header = unsupported.find(b"PK\x03\x04")
            central_header = unsupported.find(b"PK\x01\x02")
            self.assertGreaterEqual(local_header, 0)
            self.assertGreaterEqual(central_header, 0)
            unsupported[local_header + 8:local_header + 10] = (99).to_bytes(2, "little")
            unsupported[central_header + 10:central_header + 12] = (99).to_bytes(2, "little")
            unsupported_zip = Path(directory) / "unsupported-method.zip"
            unsupported_zip.write_bytes(unsupported)
            with redirect_stdout(io.StringIO()) as output:
                self.assertEqual(scanner.main(["--path", str(unsupported_zip)]), 2)
            self.assertNotIn("Traceback", output.getvalue())

    def test_nested_archives_and_archive_named_dll(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "_test") as directory:
            root = Path(directory)
            inner = io.BytesIO()
            with zipfile.ZipFile(inner, "w") as archive:
                # Nested payloads are not treated as scanned just because their
                # compressed outer bytes were inspected.
                archive.writestr("private.txt", sample_profile_path())
            nested = root / "outer.zip"
            with zipfile.ZipFile(nested, "w") as archive:
                archive.writestr("nested.dll", inner.getvalue())
            findings = scanner.scan_path(nested, self.rules)
            self.assertIn("nested_archive_requires_separate_scan", {kind for item in findings for kind in item["classes"]})

            malformed = bytearray(inner.getvalue())
            malformed[-6:-2] = (0x7FFFFFFF).to_bytes(4, "little")
            bad_nested = root / "bad-nested.zip"
            with zipfile.ZipFile(bad_nested, "w") as archive:
                archive.writestr("inner.zip", malformed)
            with redirect_stdout(io.StringIO()):
                self.assertEqual(scanner.main(["--path", str(bad_nested)]), 2)

            dll = root / "false-positive.zip"
            with zipfile.ZipFile(dll, "w") as archive:
                archive.writestr("runtime.dll", malformed + sample_profile_path().encode())
            findings = scanner.scan_path(dll, self.rules)
            self.assertIn("profile_path", {kind for item in findings for kind in item["classes"]})

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

