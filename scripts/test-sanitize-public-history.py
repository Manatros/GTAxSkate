#!/usr/bin/env python3
"""Synthetic local-only regression tests for the public history sanitizer."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("sanitizer", Path(__file__).with_name("sanitize-public-history.py"))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class HistoryTests(unittest.TestCase):
    def setUp(self):
        test_root = Path(__file__).resolve().parents[1] / "_test"
        test_root.mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix="privacy-fixture-", dir=test_root)
        self.root = Path(self.temp.name)
        self.repo, self.output = self.root / "source", self.root / "public.git"
        self.repo.mkdir()
        self.call("init", "-b", "main")
        self.call("config", "user.name", "Synthetic Private")
        self.call("config", "user.email", "private@example.invalid")
        self.call("config", "commit.gpgsign", "false")
        self.call("config", "tag.gpgsign", "false")
        self.rules = {"replacements": [{"from": "Synthetic Private", "to": "PublicHandle"},
                                        {"from": "private@example.invalid", "to": "public@example.invalid"}],
                      "identities": ["Synthetic Private <private@example.invalid>"],
                      "public_identity": "PublicHandle <public@example.invalid>"}

    def tearDown(self):
        self.temp.cleanup()

    def call(self, *args, data=None, repo=None, env=None):
        return subprocess.check_output(["git", "-C", str(repo or self.repo), *args], input=data,
                                       env=env, stderr=subprocess.PIPE)

    def commit(self, message):
        self.call("add", "-A")
        self.call("commit", "-m", message)
        return self.call("rev-parse", "HEAD").strip().decode()

    def sanitize(self, *refs):
        return MODULE.Sanitizer(self.repo, self.output, self.rules).run(list(refs or ["refs/heads/main"]))

    def test_merge_history_tags_encoding_modes_and_other_authors(self):
        (self.repo / "README.md").write_text("by Synthetic Private\n", encoding="utf-8")
        (self.repo / "LICENSE").write_text("Copyright Third Party, Apache-2.0\n")
        (self.repo / "binary.dat").write_bytes(b"\0\xff\x80\0")
        (self.repo / "unicode.txt").write_bytes("Synthetic Private\r\n".encode("utf-16"))
        self.commit("initial Synthetic Private")
        self.call("checkout", "-b", "feature")
        (self.repo / "feature.txt").write_text("Synthetic Private\n")
        self.commit("feature")
        self.call("checkout", "main")
        (self.repo / "other.txt").write_text("Other contributor\n")
        self.call("add", "-A")
        env = os.environ.copy()
        env.update(GIT_AUTHOR_NAME="Other Contributor", GIT_AUTHOR_EMAIL="other@example.invalid",
                   GIT_COMMITTER_NAME="Other Contributor", GIT_COMMITTER_EMAIL="other@example.invalid")
        self.call("commit", "-m", "third-party contribution", env=env)
        self.call("merge", "--no-ff", "feature", "-m", "merge by Synthetic Private")
        self.call("update-index", "--chmod=+x", "feature.txt")
        self.call("commit", "-m", "executable")
        self.call("tag", "-a", "v1", "-m", "release Synthetic Private")
        self.call("tag", "-a", "nested", "v1", "-m", "nested private@example.invalid")
        self.call("branch", "private-unpublished")
        before = self.call("show-ref")
        report = self.sanitize("refs/heads/main", "refs/tags/v1", "refs/tags/nested")
        self.assertEqual(before, self.call("show-ref"))
        self.assertEqual(set(report["refs"]), {"refs/heads/main", "refs/tags/v1", "refs/tags/nested"})
        refs = self.call("for-each-ref", "--format=%(refname)", repo=self.output).decode()
        self.assertNotIn("feature", refs)
        self.assertNotIn("private-unpublished", refs)
        history = self.call("log", "--all", "--format=%an <%ae> %cn <%ce> %B", repo=self.output).decode()
        self.assertNotIn("Synthetic Private", history)
        self.assertNotIn("private@example.invalid", history)
        self.assertIn("Other Contributor <other@example.invalid>", history)
        self.assertEqual(self.call("show", "main:README.md", repo=self.output), b"by PublicHandle\n")
        self.assertEqual(self.call("show", "main:LICENSE", repo=self.output),
                         self.call("show", "main:LICENSE"))
        self.assertEqual(self.call("show", "main:binary.dat", repo=self.output), b"\0\xff\x80\0")
        self.assertEqual(self.call("show", "main:unicode.txt", repo=self.output).decode("utf-16"),
                         "PublicHandle\r\n")
        self.assertEqual(self.call("ls-tree", "main", "feature.txt", repo=self.output).split()[0], b"100755")
        self.assertEqual(self.call("rev-list", "--count", "--merges", "main", repo=self.output).strip(), b"1")
        self.assertIn(b"release PublicHandle", self.call("cat-file", "tag", "v1", repo=self.output))
        self.assertNotIn(b"private@example.invalid", self.call("cat-file", "tag", "nested", repo=self.output))
        self.assertEqual(self.call("rev-list", "--all", "--count", repo=self.output),
                         self.call("rev-list", "--all", "--count"))
        self.assertFalse((self.output / "objects" / "info" / "alternates").exists())

    def test_private_data_in_ancestor_is_sanitized_even_if_deleted(self):
        (self.repo / "old.txt").write_text("Synthetic Private")
        self.commit("old")
        old_blob = self.call("rev-parse", "HEAD:old.txt").decode().strip()
        (self.repo / "old.txt").unlink()
        (self.repo / "now.txt").write_text("public")
        self.commit("now")
        self.sanitize()
        self.assertEqual(self.call("show", "main~1:old.txt", repo=self.output), b"PublicHandle")
        self.assertFalse((self.output / "objects" / old_blob[:2] / old_blob[2:]).exists())

    def test_binary_private_data_fails_without_refs(self):
        (self.repo / "data.bin").write_bytes(b"\0Synthetic Private\0")
        self.commit("binary")
        with self.assertRaisesRegex(ValueError, "binary"):
            self.sanitize()
        self.assertEqual(self.call("for-each-ref", repo=self.output), b"")

    def test_protected_attribution_refuses_change(self):
        (self.repo / "LICENSE").write_text("Copyright Synthetic Private")
        self.commit("license")
        with self.assertRaisesRegex(ValueError, "protected"):
            self.sanitize()
        self.assertEqual(self.call("for-each-ref", repo=self.output), b"")

    def test_refuse_existing_destination(self):
        (self.repo / "file").write_text("public")
        self.commit("one")
        self.output.mkdir()
        (self.output / "keep").write_text("keep")
        with self.assertRaisesRegex(ValueError, "must not exist"):
            self.sanitize()
        self.assertEqual((self.output / "keep").read_text(), "keep")

    def test_unknown_private_identity_requires_exact_rule(self):
        (self.repo / "file").write_text("public")
        self.call("config", "user.email", "another@example.invalid")
        self.commit("one")
        with self.assertRaisesRegex(ValueError, "exact identity"):
            self.sanitize()

    def test_signatures_removed_gitlinks_and_lightweight_tags_preserved(self):
        (self.repo / "file").write_text("public")
        first = self.commit("base")
        self.call("update-index", "--add", "--cacheinfo", "160000," + first + ",dependency")
        self.call("commit", "-m", "gitlink")
        raw = self.call("cat-file", "commit", "HEAD")
        headers, body = raw.split(b"\n\n", 1)
        signed = headers + b"\ngpgsig -----BEGIN PGP SIGNATURE-----\n fake\n -----END PGP SIGNATURE-----\n\n" + body
        signed_id = self.call("hash-object", "-t", "commit", "-w", "--stdin", data=signed).strip()
        self.call("update-ref", "refs/heads/main", signed_id.decode())
        self.call("tag", "light")
        self.call("tag", "-a", "signed", "-m", "Synthetic Private release")
        raw_tag = self.call("cat-file", "tag", "signed")
        raw_tag += b"-----BEGIN PGP SIGNATURE-----\nfake\n-----END PGP SIGNATURE-----\n"
        tag_id = self.call("hash-object", "-t", "tag", "-w", "--stdin", data=raw_tag).strip()
        self.call("update-ref", "refs/tags/signed", tag_id.decode())
        report = self.sanitize("refs/heads/main", "refs/tags/light", "refs/tags/signed")
        self.assertEqual(report["statistics"]["removed_signatures"], 2)
        self.assertNotIn(b"gpgsig", self.call("cat-file", "commit", "main", repo=self.output))
        self.assertNotIn(b"SIGNATURE", self.call("cat-file", "tag", "signed", repo=self.output))
        self.assertEqual(self.call("rev-parse", "main", repo=self.output),
                         self.call("rev-parse", "light", repo=self.output))
        self.assertEqual(self.call("ls-tree", "main", "dependency", repo=self.output).split()[2], first.encode())

    def test_optional_whole_word_literal_replacement(self):
        self.rules["replacements"].append({"from": "Alex", "to": "PublicHandle", "whole_word": True})
        self.rules["replacements"].append({"from": "A.B", "to": r"literal\1", "whole_word": True})
        (self.repo / "names.txt").write_text("Alex Alexander XAlex Alex_ alex (Alex) A.B A0B XA.B", encoding="utf-8")
        self.commit("Alex met Alexander")
        self.sanitize()
        self.assertEqual(self.call("show", "main:names.txt", repo=self.output).decode(),
                         r"PublicHandle Alexander XAlex Alex_ alex (PublicHandle) literal\1 A0B XA.B")
        self.assertEqual(self.call("log", "-1", "--format=%s", repo=self.output).strip(),
                         b"PublicHandle met Alexander")


if __name__ == "__main__":
    unittest.main()
