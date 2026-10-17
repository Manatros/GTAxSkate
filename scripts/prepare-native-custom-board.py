"""Stage one selected custom board through SK8V's original native drawable path.

Offline candidate only: does not install, alter source assets, or implement live
shop/model switching. Skeleton, skin bindings and source geometry stay intact.
"""
import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
import re
import shutil
import subprocess
import xml.etree.ElementTree as ET


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def same_text(a, b):
    a, b = (a or '').split(), (b or '').split()
    if len(a) != len(b):
        return False
    for x, y in zip(a, b):
        if x == y:
            continue
        try:
            if not math.isclose(float(x), float(y), rel_tol=1e-6, abs_tol=1e-7):
                return False
        except ValueError:
            return False
    return True


def same_tree(a, b):
    if a is None or b is None:
        return a is None and b is None
    if a.tag == b.tag == 'TextureDictionary':
        # RAGE writes the dictionary in name-hash order, independently of
        # material indices. Compare every named texture, not insertion order.
        left = {e.findtext('Name'): e for e in a}
        right = {e.findtext('Name'): e for e in b}
        return (len(left) == len(a) == len(b) == len(right) and left.keys() == right.keys()
                and all(same_tree(left[k], right[k]) for k in left))
    return (a.tag == b.tag and a.attrib.keys() == b.attrib.keys()
            and all(same_text(a.attrib[k], b.attrib[k]) for k in a.attrib)
            and same_text(a.text, b.text) and len(a) == len(b)
            and all(same_tree(x, y) for x, y in zip(a, b)))


def filename(text):
    if not text or not re.fullmatch(r'[A-Za-z0-9_.-]+\.dds', text) or text in ('.dds', '..dds'):
        raise ValueError('Unsafe texture filename')
    return text


def compose(native, parts, texture_dirs, textures_out):
    """Compose unmodified source geometries, remapping only shader indices."""
    if not 1 <= len(parts) <= 8 or len(parts) != len(texture_dirs):
        raise ValueError('Expected one to eight parts and exactly one texture directory per part')
    result = copy.deepcopy(native)
    models = result.findall('./DrawableModelsHigh/Item')
    if len(models) != 1:
        raise ValueError('Expected one native High model')
    model = models[0]
    if int(model.find('HasSkin').get('value')) != 1 or not int(model.find('Flags').get('value')) & 1:
        raise ValueError('Native model lacks skin binding/render flag')
    for level in ('Med', 'Low', 'Vlow'):
        if result.findall('./DrawableModels' + level + '/Item'):
            raise ValueError('Additional native LODs require explicit support')
    geometries = model.find('Geometries')
    geometries.clear()
    group = result.find('ShaderGroup')
    dictionary, shaders = group.find('TextureDictionary'), group.find('Shaders')
    dictionary.clear()
    shaders.clear()
    texture_records, positions = {}, []
    for part, directory in zip(parts, texture_dirs):
        if not part.findall('./DrawableModelsHigh/Item/Geometries/Item'):
            raise ValueError('Part has no High geometries')
        if not same_tree(native.find('Skeleton'), part.find('Skeleton')):
            raise ValueError('Part skeleton differs from native skeleton')
        if not same_tree(native.find('Joints'), part.find('Joints')):
            raise ValueError('Part joints differ from native joints')
        for level in ('Med', 'Low', 'Vlow'):
            if part.findall('./DrawableModels' + level + '/Item'):
                raise ValueError('Additional part LODs require explicit support')
        for texture in part.findall('./ShaderGroup/TextureDictionary/Item'):
            name = texture.findtext('Name')
            if not name or not re.fullmatch(r'[A-Za-z0-9_.-]+', name):
                raise ValueError('Unsafe texture name')
            file = filename(texture.findtext('FileName'))
            source = directory / file
            digest = sha(source)
            key = name.lower()
            if key in texture_records:
                prior, prior_hash = texture_records[key]
                if not same_tree(prior, texture) or prior_hash != digest:
                    raise ValueError('Texture name collision with different metadata/pixels: ' + name)
                continue
            target = textures_out / file
            if target.exists() and sha(target) != digest:
                raise ValueError('Texture filename collision: ' + file)
            shutil.copyfile(source, target)
            texture_records[key] = (copy.deepcopy(texture), digest)
            dictionary.append(copy.deepcopy(texture))
        part_shaders = part.findall('./ShaderGroup/Shaders/Item')
        offset = len(shaders)
        for shader in part_shaders:
            shaders.append(copy.deepcopy(shader))
        for source_model in part.findall('./DrawableModelsHigh/Item'):
            for field in ('RenderMask', 'Flags', 'HasSkin', 'BoneIndex', 'Unknown1'):
                if field == 'Flags' and source_model.find(field) is not None:
                    # Older generated parts omitted the skinned render bit.
                    # Normalize that known exporter inconsistency only in the
                    # derived model; the original files remain byte-identical.
                    if int(source_model.find(field).get('value')) | 1 == int(model.find(field).get('value')):
                        continue
                if not same_tree(model.find(field), source_model.find(field)):
                    raise ValueError('Part model binding/render metadata differs: ' + field)
            for source_geometry in source_model.findall('./Geometries/Item'):
                geometry = copy.deepcopy(source_geometry)
                shader = geometry.find('ShaderIndex')
                index = int(shader.get('value'))
                if not 0 <= index < len(part_shaders):
                    raise ValueError('Geometry shader index outside source shader list')
                shader.set('value', str(offset + index))
                geometries.append(geometry)
                layout = [e.tag for e in geometry.findall('./VertexBuffer/Layout/*')]
                if layout[:1] != ['Position']:
                    raise ValueError('Unsupported vertex layout')
                positions.extend([list(map(float, line.split()[:3])) for line in
                                  geometry.findtext('VertexBuffer/Data').splitlines() if line.strip()])
    if len(geometries) == 0 or not positions:
        raise ValueError('No source geometries')
    if any(len(p) != 3 or not all(math.isfinite(v) for v in p) for p in positions):
        raise ValueError('Invalid or nonfinite source vertex position')
    # Derive only aggregate culling bounds; never move or rescale vertices.
    minimum = [min(v[i] for v in positions) for i in range(3)]
    maximum = [max(v[i] for v in positions) for i in range(3)]
    center = [(a + b) / 2 for a, b in zip(minimum, maximum)]
    for tag, value in [('BoundingBoxMin', minimum), ('BoundingBoxMax', maximum), ('BoundingSphereCenter', center)]:
        result.find(tag).attrib.update(dict(zip('xyz', map(str, value))))
    radius = max(math.dist(p, center) for p in positions)
    result.find('BoundingSphereRadius').set('value', str(radius))
    return result, {filename(t.findtext('FileName')): digest for t, digest in texture_records.values()}


def run(rage, *args):
    completed = subprocess.run([str(rage), '--no-update-check', 'resource', *map(str, args)],
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if completed.returncode:
        raise RuntimeError(completed.stderr[-2000:] or completed.stdout[-2000:])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('native', 'deck', 'trucks', 'wheels', 'rage', 'out'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    out, rage = args.out.resolve(), args.rage.resolve()
    sources = [getattr(args, n).resolve() for n in ('native', 'deck', 'trucks', 'wheels')]
    if out.exists() or any(out == p.parent or p.parent in out.parents for p in sources):
        raise ValueError('Use a new output folder outside source asset directories')
    if any(not p.is_file() or p.suffix.lower() != '.ydr' for p in sources):
        raise ValueError('Expected existing native/deck/truck/wheel YDRs')
    # The payload replaces only the drawable; retain the original matching
    # native archetype in the game. No new registration/model name is invented.
    archetype = sources[0].with_suffix('.ytyp')
    if not archetype.is_file():
        raise ValueError('Matching native archetype is required beside native YDR')
    archetype_hash = sha(archetype)
    before = [sha(p) for p in sources]
    out.mkdir(parents=True)
    roots, dirs = [], []
    for label, source in zip(('native', 'deck', 'trucks', 'wheels'), sources):
        folder = out / 'sources' / label
        folder.mkdir(parents=True)
        xml = folder / 'source.xml'
        run(rage, 'dump', source, '-o', xml)
        roots.append(ET.parse(xml).getroot())
        dirs.append(folder)
    work = out / 'combined'
    work.mkdir()
    result, textures = compose(roots[0], roots[1:], dirs[1:], work)
    xml = work / 'skatev_board.ydr.xml'
    ET.ElementTree(result).write(xml, encoding='utf-8', xml_declaration=True)
    relative = 'private/board-native/skatev_board.ydr'
    target = out / 'payload' / relative
    target.parent.mkdir(parents=True)
    run(rage, 'build', xml, '-o', target, '--strict')
    check = out / 'verified'
    check.mkdir()
    checked = check / 'skatev_board.xml'
    run(rage, 'dump', target, '-o', checked)
    if not same_tree(result, ET.parse(checked).getroot()):
        raise ValueError('Composed drawable changed during binary roundtrip')
    for file, digest in textures.items():
        if sha(check / file) != digest:
            raise ValueError('Embedded texture changed during binary roundtrip: ' + file)
    if before != [sha(p) for p in sources] or sha(archetype) != archetype_hash:
        raise ValueError('A source changed during staging')
    manifest = {'format': 'gtaxskate-native-custom-board', 'version': 1,
                'candidateOnly': True, 'rageSha256': sha(rage),
                'sources': [{'name': str(p), 'sha256': h} for p, h in zip(sources, before)],
                'matchingNativeArchetype': {'name': str(archetype), 'sha256': archetype_hash, 'action': 'retain unchanged; required by this YDR-only candidate'},
                'files': [{'name': relative, 'originalSha256': before[0], 'sha256': sha(target)}],
                'geometryCount': len(result.findall('./DrawableModelsHigh/Item/Geometries/Item')),
                'shaderCount': len(result.findall('./ShaderGroup/Shaders/Item')),
                'textures': textures, 'verification': 'Full composed XML roundtrip, exact DDS hashes, unchanged source hashes',
                'limits': 'Selected deck/trucks/wheels only; no live shop/paint/wear switching; existing griptape/scuff/sticker overlays excluded'}
    (out / 'manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
    print(json.dumps({'output': str(out), 'geometryCount': manifest['geometryCount'], 'shaderCount': manifest['shaderCount']}))


if __name__ == '__main__':
    main()
