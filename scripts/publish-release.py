"""Publish a GitHub release of GTAxSkate from already built, checked assets.

    python scripts/publish-release.py --tag v0.7.0 --title "..." --notes notes.md \
        --manifest-json build/release/RELEASE-MANIFEST.json [--repo Manatros/GTAxSkate] [--dry-run]

Steps (each verified, nothing replaced silently):
  1. main and the annotated tag must resolve to the same commit on GitHub;
  2. a draft release is created (or an existing *draft* reused; a public one is refused);
  3. every asset listed in SHA256SUMS.txt (plus SHA256SUMS.txt itself) is uploaded and the
     server's SHA-256 digest compared with the local file;
  4. only then the draft is made public and latest, and /releases/latest is checked;
  5. a receipt build/release-published-<tag>.json is written.
The GitHub token comes from Git's credential manager (as `git push` uses it) and stays in memory.
"""
import argparse
import datetime
import hashlib
import json
import re
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def token():
    creds = subprocess.check_output(['git', 'credential', 'fill'], input=b'protocol=https\nhost=github.com\n\n', cwd=ROOT)
    fields = dict(line.split('=', 1) for line in creds.decode().splitlines() if '=' in line)
    return fields['password']


def checked_name(name):
    if not isinstance(name, str) or not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]*', name) or name in ('.', '..'):
        raise ValueError('Invalid asset filename')
    return name


def validate_local(manifest_path, tag):
    out = manifest_path.resolve().parent
    manifest = json.loads(manifest_path.read_text(encoding='utf-8-sig'))
    if manifest.get('format') != 'gtaxskate-release' or manifest.get('version') != 1 or manifest.get('release') != tag:
        raise ValueError('Manifest format/version/release differs')
    for field in ('publishedSk8vSourceCommit', 'publishedSk8vSourceTree'):
        if not re.fullmatch('[0-9a-f]{40}', manifest.get(field, '')):
            raise ValueError('Missing exact source identity: ' + field)
    expected_assets = manifest.get('assets', {})
    if not isinstance(expected_assets, dict) or not expected_assets:
        raise ValueError('Manifest assets are missing')
    sums = out / 'SHA256SUMS.txt'
    hashes = {}
    for line in sums.read_text(encoding='utf-8-sig').splitlines():
        if not line.strip():
            continue
        match = re.fullmatch(r'([0-9a-f]{64})  (.+)', line)
        if not match:
            raise ValueError('Malformed checksum entry')
        digest, name = match.groups()
        checked_name(name)
        if name.lower() in {n.lower() for n in hashes} or name == sums.name:
            raise ValueError('Duplicate or recursive checksum entry: ' + name)
        path = out / name
        if path.is_symlink() or path.resolve().parent != out or not path.is_file() or sha(path) != digest:
            raise ValueError(name + ': local file differs from SHA256SUMS.txt')
        hashes[name] = digest
    for name, digest in expected_assets.items():
        checked_name(name)
        if not re.fullmatch('[0-9a-f]{64}', str(digest)) or hashes.get(name) != digest:
            raise ValueError('Manifest asset differs: ' + name)
    if set(hashes) - set(expected_assets) - {manifest_path.name}:
        raise ValueError('Checksum file contains assets absent from manifest')
    if manifest_path.name in hashes and hashes[manifest_path.name] != sha(manifest_path):
        raise ValueError('Manifest checksum differs')
    pair = manifest.get('acceptedBinarySha256', {})
    for name in ('SkateVLegacy.asi', 'SkateVRuntime.dll'):
        if not re.fullmatch('[0-9a-f]{64}', pair.get(name, '')):
            raise ValueError('Exact candidate binary pair is missing')
    pair_archives = 0
    for name in expected_assets:
        if not name.lower().endswith('.zip'):
            continue
        with zipfile.ZipFile(out / name) as archive:
            found = {}
            for entry in archive.infolist():
                leaf = entry.filename.replace('\\', '/').rsplit('/', 1)[-1]
                if leaf in pair:
                    found[leaf] = found.get(leaf, 0) + 1
                    if hashlib.sha256(archive.read(entry)).hexdigest() != pair[leaf]:
                        raise ValueError('Candidate binary differs in ' + name + ': ' + leaf)
            if found:
                if any(found.get(binary) != 1 for binary in pair):
                    raise ValueError('Missing or duplicate candidate pair in ' + name)
                pair_archives += 1
    if not pair_archives:
        raise ValueError('No release archive contains the candidate binary pair')
    return manifest, out, list(hashes) + [sums.name]


def publication_blocks(manifest, now):
    blocks = []
    approval = manifest.get('gameplayApproval', {})
    if not isinstance(approval, dict):
        approval = {}
    gameplay_valid = approval.get('approved') is True and approval.get('approvedBy') == 'Manatros' and bool(manifest.get('acceptedTestId'))
    try:
        approved = datetime.datetime.fromisoformat(approval.get('approvedAt', '').replace('Z', '+00:00'))
        if approved.tzinfo is None or approved > now:
            raise ValueError()
    except (ValueError, TypeError, AttributeError):
        gameplay_valid = False
    authorization = manifest.get('releaseAuthorization', {})
    if not isinstance(authorization, dict):
        authorization = {}
    authorized = (authorization.get('authorized') is True and authorization.get('authorizedBy') == 'Manatros'
                  and bool(manifest.get('acceptedTestId')) and authorization.get('testId') == manifest.get('acceptedTestId'))
    try:
        authorized_at = datetime.datetime.fromisoformat(authorization.get('authorizedAt', '').replace('Z', '+00:00'))
        if authorized_at.tzinfo is None or authorized_at > now:
            raise ValueError()
    except (ValueError, TypeError, AttributeError):
        authorized = False
    if not gameplay_valid and not authorized:
        blocks.append('explicit maintainer gameplay acceptance or valid release authorization of this exact candidate is pending')
        blocks.append('valid gameplay approval or release authorization timestamp is required')
    if not re.fullmatch('[0-9a-f]{40}', manifest.get('extensionSourceCommit', '')):
        blocks.append('exact extension source commit is pending')
    earliest = datetime.date(2026, 10, 24) if manifest['release'] == 'v0.7.0' else datetime.date.min
    if manifest.get('notBefore'):
        earliest = max(earliest, datetime.date.fromisoformat(manifest['notBefore']))
    # UTC is conservative at the Berlin date boundary and cannot publish early.
    if now.date() < earliest:
        blocks.append('release date gate: not before ' + earliest.isoformat())
    return blocks


def check_weekly_gate(latest, now):
    published = datetime.datetime.fromisoformat(latest['published_at'].replace('Z', '+00:00'))
    if published.tzinfo is None or now < published + datetime.timedelta(days=7):
        raise ValueError('Weekly release gate: seven days since the latest publication have not elapsed')


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--tag', required=True)
    ap.add_argument('--title', required=True)
    ap.add_argument('--notes', required=True, type=Path)
    ap.add_argument('--manifest-json', required=True, type=Path)
    ap.add_argument('--repo', default='Manatros/GTAxSkate')
    ap.add_argument('--dry-run', action='store_true')
    args = ap.parse_args()

    manifest, out, names = validate_local(args.manifest_json, args.tag)
    expected_hashes = {name: sha(out / name) for name in names}
    for name, digest in manifest['assets'].items():
        if expected_hashes[name] != digest:
            sys.exit('Asset changed after validation: ' + name)
    now = datetime.datetime.now(datetime.timezone.utc)
    blocks = publication_blocks(manifest, now)
    body = args.notes.read_text(encoding='utf-8')
    print(f'{len(names)} assets checked locally: {", ".join(names)}')
    if args.dry_run:
        for block in blocks:
            print('pending: ' + block)
        print('dry run: nothing sent to GitHub')
        return
    if blocks:
        sys.exit('Publication blocked: ' + '; '.join(blocks))

    tok = token()

    def api(path, data=None, method=None, content_type='application/json'):
        url = path if path.startswith('https://') else f'https://api.github.com/repos/{args.repo}{path}'
        payload = json.dumps(data).encode() if isinstance(data, dict) else data
        req = urllib.request.Request(url, payload, method=method, headers={
            'Authorization': 'Bearer ' + tok, 'Accept': 'application/vnd.github+json',
            'User-Agent': 'GTAxSkate-release', 'Content-Type': content_type})
        with urllib.request.urlopen(req, timeout=600) as response:
            return json.load(response)

    check_weekly_gate(api('/releases/latest'), now)
    main_sha = api('/git/ref/heads/main')['object']['sha']
    ref = api(f'/git/ref/tags/{args.tag}')['object']
    tag_sha = api('/git/tags/' + ref['sha'])['object']['sha'] if ref['type'] == 'tag' else ref['sha']
    if main_sha != tag_sha:
        sys.exit(f'main ({main_sha}) and {args.tag} ({tag_sha}) differ on GitHub')
    if main_sha != manifest['extensionSourceCommit']:
        sys.exit('GitHub main/tag differ from the manifest source commit')
    fork_source = api('https://api.github.com/repos/Manatros/SK8V/git/commits/' + manifest['publishedSk8vSourceCommit'])
    if fork_source['tree']['sha'] != manifest['publishedSk8vSourceTree']:
        sys.exit('Published SK8V source tree differs from manifest')
    try:
        release = api(f'/releases/tags/{args.tag}')
        if not release['draft']:
            sys.exit(f'{args.tag} is already public; refusing to replace its assets')
    except urllib.error.HTTPError as e:
        if e.code != 404:
            raise
        release = api('/releases', {'tag_name': args.tag, 'target_commitish': 'main', 'name': args.title,
                                    'body': body, 'draft': True, 'prerelease': False}, 'POST')
    existing = {a['name']: a for a in api(f"/releases/{release['id']}/assets")}
    if set(existing) - set(names):
        sys.exit('Draft contains unexpected assets; review and remove them before publishing')
    for name in names:
        data = (out / name).read_bytes()
        expected = 'sha256:' + hashlib.sha256(data).hexdigest()
        if expected != 'sha256:' + expected_hashes[name]:
            sys.exit('Asset changed after validation: ' + name)
        if name in existing:
            if existing[name].get('digest') != expected:
                sys.exit(f'draft asset {name} differs from the local file; delete it on GitHub first')
            continue
        url = (f"https://uploads.github.com/repos/{args.repo}/releases/{release['id']}/assets?name="
               + urllib.parse.quote(name))
        asset = api(url, data, 'POST', 'application/octet-stream')
        if asset['size'] != len(data) or asset.get('digest') != expected:
            sys.exit(f'upload of {name} did not verify')
        print(f'uploaded and verified: {name}', flush=True)
    remote = {a['name']: a for a in api(f"/releases/{release['id']}/assets")}
    for name in names:
        if remote[name].get('digest') != 'sha256:' + expected_hashes[name]:
            sys.exit(f'{name}: server digest differs')
    check_weekly_gate(api('/releases/latest'), datetime.datetime.now(datetime.timezone.utc))
    release = api(f"/releases/{release['id']}", {'name': args.title, 'body': body, 'draft': False,
                                                  'prerelease': False, 'make_latest': 'true'}, 'PATCH')
    latest = api('/releases/latest')
    if latest['tag_name'] != args.tag or latest['draft']:
        sys.exit('the release is not GitHub latest')
    receipt = {'releaseUrl': release['html_url'], 'releaseId': release['id'], 'tag': args.tag, 'mainCommit': main_sha,
               'assets': {n: remote[n]['digest'] for n in names},
               'verifiedUtc': datetime.datetime.now(datetime.timezone.utc).isoformat()}
    (ROOT / 'build' / f'release-published-{args.tag}.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print('published and verified: ' + release['html_url'])


if __name__ == '__main__':
    main()
