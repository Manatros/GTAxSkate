"""Offline publication gate regression tests; no credentials or network access."""
import datetime
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock
import zipfile

spec = importlib.util.spec_from_file_location('publisher', Path(__file__).with_name('publish-release.py'))
publisher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publisher)


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        qa = Path(__file__).resolve().parents[1] / 'build' / 'publish-release-tests'
        qa.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=qa)
        self.addCleanup(self.temp.cleanup)
        self.out = Path(self.temp.name)
        self.path = self.out / 'RELEASE-MANIFEST.json'
        self.pair = {'SkateVLegacy.asi': b'host', 'SkateVRuntime.dll': b'runtime'}
        self.manifest = dict(format='gtaxskate-release', version=1, release='v0.7.0',
                             publishedSk8vSourceCommit='a'*40, publishedSk8vSourceTree='b'*40,
                             acceptedBinarySha256={k: hashlib.sha256(v).hexdigest() for k,v in self.pair.items()})
        self.write()

    def write(self):
        with zipfile.ZipFile(self.out / 'candidate.zip', 'w') as archive:
            for name, data in self.pair.items():
                archive.writestr(name, data)
        self.manifest['assets'] = {'candidate.zip': publisher.sha(self.out / 'candidate.zip')}
        self.path.write_text(json.dumps(self.manifest), encoding='utf-8')
        (self.out / 'SHA256SUMS.txt').write_text(self.manifest['assets']['candidate.zip'] + '  candidate.zip\n', encoding='utf-8')

    def test_valid_candidate_without_approval(self):
        manifest, out, names = publisher.validate_local(self.path, 'v0.7.0')
        self.assertEqual(out, self.out)
        self.assertEqual(names, ['candidate.zip', 'SHA256SUMS.txt'])
        self.assertGreaterEqual(len(publisher.publication_blocks(manifest, datetime.datetime(2026,10,21,tzinfo=datetime.timezone.utc))), 3)

    def test_tamper_rejected(self):
        (self.out / 'candidate.zip').write_bytes(b'tampered')
        with self.assertRaisesRegex(ValueError, 'differs'):
            publisher.validate_local(self.path, 'v0.7.0')

    def test_traversal_and_duplicate_rejected(self):
        sums = self.out / 'SHA256SUMS.txt'
        original = sums.read_text()
        for text in (original.replace('candidate.zip', '../candidate.zip'), original + original):
            sums.write_text(text)
            with self.assertRaises(ValueError):
                publisher.validate_local(self.path, 'v0.7.0')

    def test_wrong_binary_pair_rejected_even_with_updated_asset_sum(self):
        self.pair['SkateVRuntime.dll'] = b'wrong-runtime'
        self.write()
        with self.assertRaisesRegex(ValueError, 'Candidate binary differs'):
            publisher.validate_local(self.path, 'v0.7.0')

    def test_explicit_acceptance_and_date(self):
        self.manifest.update(extensionSourceCommit='c'*40, acceptedTestId='candidate-123',
                             gameplayApproval=dict(approved=True, approvedBy='Manatros', approvedAt='2026-10-21T10:00:00Z'))
        early = datetime.datetime(2026,10,21,12,tzinfo=datetime.timezone.utc)
        ready = datetime.datetime(2026,10,24,12,tzinfo=datetime.timezone.utc)
        self.assertTrue(any('date gate' in b for b in publisher.publication_blocks(self.manifest, early)))
        self.assertEqual(publisher.publication_blocks(self.manifest, ready), [])
        self.manifest['gameplayApproval']['approved'] = 'true'
        self.assertTrue(publisher.publication_blocks(self.manifest, ready))

    def test_weekly_gate(self):
        latest = {'tag_name': 'v0.7.0', 'published_at': '2026-10-17T13:30:00Z'}
        with self.assertRaisesRegex(ValueError, 'Weekly release gate'):
            publisher.check_weekly_gate(latest, datetime.datetime(2026,10,24,13,29,tzinfo=datetime.timezone.utc))
        with self.assertRaisesRegex(ValueError, 'Weekly release gate'):
            publisher.check_weekly_gate(latest, datetime.datetime(2026,10,24,13,29,tzinfo=datetime.timezone.utc), 'v0.8.0')
        publisher.check_weekly_gate(latest, datetime.datetime(2026,10,18,12,0,tzinfo=datetime.timezone.utc), 'v0.7.1')
        publisher.check_weekly_gate(latest, datetime.datetime(2026,10,24,13,30,tzinfo=datetime.timezone.utc))

    def authorized_manifest(self):
        self.manifest.update(extensionSourceCommit='c'*40, acceptedTestId='candidate-123',
                             gameplayApproval=dict(approved=False),
                             releaseAuthorization=dict(authorized=True, authorizedBy='Manatros',
                                                       authorizedAt='2026-10-26T10:00:00Z', testId='candidate-123'))
        return datetime.datetime(2026,10,26,12,tzinfo=datetime.timezone.utc)

    def test_owner_release_authorization_without_gameplay_claim(self):
        now = self.authorized_manifest()
        self.assertEqual(publisher.publication_blocks(self.manifest, now), [])
        self.assertIs(self.manifest['gameplayApproval']['approved'], False)

    def test_invalid_release_authorization_blocked(self):
        for field, value in [('authorized', False), ('authorized', 'true'), ('authorizedBy', 'Other'),
                             ('testId', 'wrong-candidate'), ('authorizedAt', '2026-10-27T00:00:00Z'),
                             ('authorizedAt', '2026-10-26T10:00:00'), ('authorizedAt', 'invalid'),
                             ('authorizedAt', None)]:
            with self.subTest(field=field, value=value):
                now = self.authorized_manifest()
                self.manifest['releaseAuthorization'][field] = value
                self.assertTrue(publisher.publication_blocks(self.manifest, now))

    def test_release_authorization_preserves_source_and_date_gates(self):
        now = self.authorized_manifest()
        self.manifest['extensionSourceCommit'] = ''
        self.assertTrue(any('source commit' in b for b in publisher.publication_blocks(self.manifest, now)))
        self.authorized_manifest()
        self.manifest['notBefore'] = '2026-10-27'
        self.assertTrue(any('date gate' in b for b in publisher.publication_blocks(self.manifest, now)))

    def test_dry_run_never_requests_credentials_or_network(self):
        notes = self.out / 'notes.md'
        notes.write_text('Candidate release notes', encoding='utf-8')
        argv = ['publish-release.py', '--tag', 'v0.7.0', '--title', 'Candidate', '--notes', str(notes), '--manifest-json', str(self.path), '--dry-run']
        with mock.patch.object(publisher.sys, 'argv', argv), mock.patch.object(publisher, 'token', side_effect=AssertionError('Credentials requested')), mock.patch.object(publisher.urllib.request, 'urlopen', side_effect=AssertionError('Network requested')), mock.patch('builtins.print'):
            publisher.main()

    def test_missing_approval_blocks_before_credentials(self):
        notes = self.out / 'notes.md'
        notes.write_text('Candidate release notes', encoding='utf-8')
        argv = ['publish-release.py', '--tag', 'v0.7.0', '--title', 'Candidate', '--notes', str(notes), '--manifest-json', str(self.path)]
        with mock.patch.object(publisher.sys, 'argv', argv), mock.patch.object(publisher, 'token', side_effect=AssertionError('Credentials requested')), mock.patch('builtins.print'):
            with self.assertRaisesRegex(SystemExit, 'Publication blocked'):
                publisher.main()


if __name__ == '__main__':
    unittest.main()
