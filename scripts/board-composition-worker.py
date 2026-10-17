"""Local shop drawable preparation. No game/animation access or original writes.

The host queues exact model vectors; this bounded worker builds a uniquely named
drawable and archetype, verifies them, then publishes a readiness marker last.
Run separately for the experimental composed-board host; no network/dependencies.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time
import xml.etree.ElementTree as ET

spec = importlib.util.spec_from_file_location('native_composer', Path(__file__).with_name('prepare-native-custom-board.py'))
composer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(composer)


def model_key(models):
    value = 14695981039346656037
    for byte in (''.join(m + '\n' for m in models)).encode('ascii'):
        value = ((value ^ byte) * 1099511628211) & 0xffffffffffffffff
    return f'{value:016x}'


def parse_request(path):
    match = re.fullmatch(r'(skatev_c_[0-9a-f]{1,16}_[0-9a-f]{16})\.request', path.name)
    if not match or path.stat().st_size > 2048:
        raise ValueError('Invalid request filename/size')
    payload = path.read_text(encoding='ascii')
    models = payload.splitlines()
    if not 1 <= len(models) <= 8 or any(not re.fullmatch(r'skatev_(?:d|t|tb|th|w|g)_[a-z0-9_]{1,128}', m) for m in models):
        raise ValueError('Invalid shop model vector')
    if len(set(models)) != len(models) or not match[1].endswith('_' + model_key(models)):
        raise ValueError('Duplicate models or request key mismatch')
    return match[1], models, ''.join(m + '\n' for m in models)


def source_at(folder, name):
    path = (folder / name).resolve(strict=True)
    if path.parent != folder.resolve() or not path.is_file():
        raise ValueError('Source must be a regular file directly within its asset folder')
    return path


def rage_run(rage, *args):
    result = subprocess.run([str(rage), '--no-update-check', *map(str, args)],
                            capture_output=True, text=True, timeout=90,
                            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
    if result.returncode:
        raise RuntimeError(result.stderr[-2000:] or result.stdout[-2000:])


def build(request, native, parts, rage):
    stem, models, payload = parse_request(request)
    root = request.parent
    ready = root / (stem + '.ready')
    # Immutable registration names: never replace anything that GTA could load.
    if ready.exists():
        if ready.read_text(encoding='ascii') != payload:
            raise ValueError('Published key collision')
        receipt = json.loads((root / (stem + '.json')).read_text(encoding='utf-8'))
        if receipt.get('model') != stem or receipt.get('models') != models or any(
                composer.sha(root / (stem + ext)) != receipt.get(field)
                for ext, field in (('.ydr', 'ydrSha256'), ('.ytyp', 'ytypSha256'))):
            raise ValueError('Published resource hash/identity mismatch')
        return False
    outputs = [root / (stem + ext) for ext in ('.ydr', '.ytyp', '.json')]
    if any(p.exists() for p in outputs):
        raise ValueError('Incomplete prior publication; start a new host session')
    sources = [source_at(native.parent, native.name)] + [source_at(parts, m + '.ydr') for m in models]
    hashes = [composer.sha(p) for p in sources]
    tool_hash = composer.sha(rage)
    with tempfile.TemporaryDirectory(prefix='compose-', dir=root) as tmp:
        work = Path(tmp)
        roots, directories = [], []
        for i, source in enumerate(sources):
            folder = work / str(i)
            folder.mkdir()
            xml = folder / 'source.xml'
            rage_run(rage, 'resource', 'dump', source, '-o', xml)
            roots.append(ET.parse(xml).getroot())
            directories.append(folder)
        combined = work / 'combined'
        combined.mkdir()
        drawable, textures = composer.compose(roots[0], roots[1:], directories[1:], combined)
        drawable.find('Name').text = stem
        xml = combined / (stem + '.ydr.xml')
        ET.ElementTree(drawable).write(xml, encoding='utf-8', xml_declaration=True)
        ydr, ytyp = work / (stem + '.ydr'), work / (stem + '.ytyp')
        rage_run(rage, 'resource', 'build', xml, '-o', ydr, '--strict')
        checked = work / 'checked'
        checked.mkdir()
        dumped = checked / 'composed.xml'
        rage_run(rage, 'resource', 'dump', ydr, '-o', dumped)
        if not composer.same_tree(drawable, ET.parse(dumped).getroot()):
            raise ValueError('Composed drawable changed in strict roundtrip')
        if any(composer.sha(checked / file) != digest for file, digest in textures.items()):
            raise ValueError('Embedded texture changed in roundtrip')
        rage_run(rage, 'ytyp', 'from-drawables', ydr, '-o', ytyp, '--flags', '131104')
        archetype_xml = checked / 'archetype.xml'
        rage_run(rage, 'resource', 'dump', ytyp, '-o', archetype_xml)
        archetypes = ET.parse(archetype_xml).getroot().findall('./archetypes/Item')
        if len(archetypes) != 1:
            raise ValueError('Expected exactly one derived archetype')
        archetype = archetypes[0]
        if any(archetype.findtext(tag) != stem for tag in ('name', 'assetName')):
            raise ValueError('Derived archetype does not reference composed model')
        if int(archetype.find('flags').get('value')) != 0x20020 or archetype.findtext('assetType') != 'ASSET_TYPE_DRAWABLE' or archetype.findtext('physicsDictionary'):
            raise ValueError('Derived archetype creation/physics metadata differs from native board')
        for tag, native_tag in (('bbMin', 'BoundingBoxMin'), ('bbMax', 'BoundingBoxMax'), ('bsCentre', 'BoundingSphereCenter'), ('bsRadius', 'BoundingSphereRadius')):
            a, b = archetype.find(tag), drawable.find(native_tag)
            if a is None or b is None or a.attrib.keys() != b.attrib.keys() or any(not composer.same_text(a.get(k), b.get(k)) for k in a.attrib):
                raise ValueError('Derived archetype bounds differ: ' + tag)
        if hashes != [composer.sha(p) for p in sources] or tool_hash != composer.sha(rage):
            raise ValueError('Source/tool changed during preparation')
        manifest = {'format': 'gtaxskate-shop-composition', 'version': 1, 'model': stem,
                    'models': models, 'sources': [{'file': p.name, 'sha256': h} for p, h in zip(sources, hashes)],
                    'rageSha256': tool_hash, 'ydrSha256': composer.sha(ydr), 'ytypSha256': composer.sha(ytyp),
                    'textures': textures, 'geometryCount': len(drawable.findall('./DrawableModelsHigh/Item/Geometries/Item')),
                    'verification': 'Full XML and exact DDS roundtrip; unchanged inputs; unique matching archetype'}
        receipt = work / (stem + '.json')
        receipt.write_text(json.dumps(manifest, indent=2), encoding='utf-8')
        for source, target in zip((ydr, ytyp, receipt), outputs):
            os.rename(source, target)
        marker = work / 'ready'
        marker.write_text(payload, encoding='ascii', newline='\n')
        os.rename(marker, ready)
    return True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('root', 'native', 'parts', 'rage'):
        parser.add_argument('--' + name, required=True, type=Path)
    parser.add_argument('--watch', action='store_true')
    parser.add_argument('--idle-exit', type=float, default=600)
    args = parser.parse_args()
    root = args.root.resolve(strict=True)
    log = (root / 'worker.log').open('a', encoding='utf-8', buffering=1)
    sys.stdout = log
    sys.stderr = log
    lock = root / 'worker.lock'
    # An OS lock survives neither crash nor shutdown. The small lock file may
    # remain, but never prevents the next host session from preparing boards.
    import msvcrt
    handle = lock.open('a+b')
    if not lock.stat().st_size:
        handle.write(b'0')
        handle.flush()
    handle.seek(0)
    try:
        msvcrt.locking(handle.fileno(), msvcrt.LK_NBLCK, 1)
    except OSError:
        handle.close()
        raise SystemExit('A composition worker already owns this cache')
    last_work = time.monotonic()
    failed = False
    try:
        with handle:
            while True:
                processed = False
                active_path = root / 'active-session'
                session = active_path.read_text(encoding='ascii') if active_path.is_file() else ''
                if not re.fullmatch(r'[0-9a-f]{1,16}', session):
                    raise ValueError('Missing/invalid active-session marker from host')
                for request in sorted(root.glob('skatev_c_' + session + '_*.request'), key=lambda p: p.stat().st_mtime, reverse=True):
                    if request.with_suffix('.ready').exists() or request.with_suffix('.error').exists():
                        continue
                    try:
                        built = build(request, args.native.resolve(), args.parts.resolve(), args.rage.resolve())
                        if built:
                            print('Prepared ' + request.stem, flush=True)
                    except Exception as error:
                        failed = True
                        request.with_suffix('.error').write_text(str(error), encoding='utf-8')
                        print('Failed ' + request.stem + ': ' + str(error), flush=True)
                    last_work = time.monotonic()
                    processed = True
                    break  # Re-read active session and newest selection after each build.
                if not args.watch:
                    if processed:
                        continue
                    break
                if time.monotonic() - last_work > args.idle_exit:
                    break
                time.sleep(0.5)
    finally:
        handle.close()
    if failed:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
