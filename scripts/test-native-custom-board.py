import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest
import xml.etree.ElementTree as ET

spec = importlib.util.spec_from_file_location('compose', Path(__file__).with_name('prepare-native-custom-board.py'))
tool = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tool)
QA = Path(__file__).resolve().parents[1] / 'build' / 'native-custom-board-unit-tests'
QA.mkdir(parents=True, exist_ok=True)


def drawable(texture='shared'):
    return ET.fromstring(f'''<Drawable><Name>skatev_board</Name>
    <BoundingBoxMin x="0" y="0" z="0"/><BoundingBoxMax x="1" y="1" z="1"/>
    <BoundingSphereCenter x="0" y="0" z="0"/><BoundingSphereRadius value="1"/>
    <ShaderGroup><TextureDictionary><Item><Name>{texture}</Name><FileName>{texture}.dds</FileName></Item></TextureDictionary>
    <Shaders><Item><Name>normal_spec</Name></Item></Shaders></ShaderGroup><Skeleton id="same"/>
    <DrawableModelsHigh><Item><RenderMask value="255"/><Flags value="1"/><HasSkin value="1"/>
    <BoneIndex value="0"/><Unknown1 value="0"/><Geometries><Item><ShaderIndex value="0"/>
    <VertexBuffer><Layout><Position/></Layout><Data>0 0 0\n1 1 1</Data></VertexBuffer></Item></Geometries></Item></DrawableModelsHigh></Drawable>''')


class ComposeTests(unittest.TestCase):
    def fixture(self, folder, count=3):
        dirs = []
        for i in range(count):
            d = folder / str(i)
            d.mkdir()
            (d / 'shared.dds').write_bytes(b'same exact texture bytes')
            dirs.append(d)
        out = folder / 'out'
        out.mkdir()
        return dirs, out

    def test_shader_remapping_exact_texture_dedup_and_source_preservation(self):
        with tempfile.TemporaryDirectory(dir=QA) as temp:
            dirs, out = self.fixture(Path(temp))
            parts = [drawable() for _ in dirs]
            before = [ET.tostring(p) for p in parts]
            result, textures = tool.compose(drawable(), parts, dirs, out)
            self.assertEqual([int(g.find('ShaderIndex').get('value')) for g in result.findall('./DrawableModelsHigh/Item/Geometries/Item')], [0, 1, 2])
            self.assertEqual(len(result.findall('./ShaderGroup/TextureDictionary/Item')), 1)
            self.assertEqual(len(textures), 1)
            self.assertEqual(before, [ET.tostring(p) for p in parts])
            for original, geometry in zip(parts, result.findall('./DrawableModelsHigh/Item/Geometries/Item')):
                geometry = copy.deepcopy(geometry)
                geometry.find('ShaderIndex').set('value', '0')
                self.assertEqual(ET.tostring(geometry), ET.tostring(original.find('./DrawableModelsHigh/Item/Geometries/Item')))

    def test_same_name_with_different_pixels_is_refused(self):
        with tempfile.TemporaryDirectory(dir=QA) as temp:
            dirs, out = self.fixture(Path(temp))
            (dirs[1] / 'shared.dds').write_bytes(b'different pixels')
            with self.assertRaisesRegex(ValueError, 'collision'):
                tool.compose(drawable(), [drawable() for _ in dirs], dirs, out)

    def test_skeleton_mismatch_and_invalid_shader_index_are_refused(self):
        for failure in ('skeleton', 'shader'):
            with tempfile.TemporaryDirectory(dir=QA) as temp:
                dirs, out = self.fixture(Path(temp))
                part = drawable()
                if failure == 'skeleton':
                    part.find('Skeleton').set('id', 'different')
                else:
                    part.find('./DrawableModelsHigh/Item/Geometries/Item/ShaderIndex').set('value', '2')
                with self.assertRaises(ValueError):
                    tool.compose(drawable(), [part, drawable(), drawable()], dirs, out)

    def test_input_count_and_nonfinite_vertex_are_refused(self):
        with tempfile.TemporaryDirectory(dir=QA) as temp:
            dirs, out = self.fixture(Path(temp))
            with self.assertRaisesRegex(ValueError, 'exactly'):
                tool.compose(drawable(), [drawable()] * 3, dirs[:2], out)
            part = drawable()
            part.find('./DrawableModelsHigh/Item/Geometries/Item/VertexBuffer/Data').text = '0 0 nan'
            with self.assertRaisesRegex(ValueError, 'nonfinite'):
                tool.compose(drawable(), [part, drawable(), drawable()], dirs, out)

    def test_partial_split_trucks_grip_and_bound_are_supported(self):
        for count in (1, 3, 4, 5, 8):
            with self.subTest(count=count), tempfile.TemporaryDirectory(dir=QA) as temp:
                dirs, out = self.fixture(Path(temp), count)
                result, _ = tool.compose(drawable(), [drawable() for _ in dirs], dirs, out)
                self.assertEqual(len(result.findall('./DrawableModelsHigh/Item/Geometries/Item')), count)
        with tempfile.TemporaryDirectory(dir=QA) as temp:
            dirs, out = self.fixture(Path(temp), 9)
            with self.assertRaises(ValueError):
                tool.compose(drawable(), [drawable() for _ in dirs], dirs, out)

    def test_empty_part_is_not_silently_dropped(self):
        with tempfile.TemporaryDirectory(dir=QA) as temp:
            dirs, out = self.fixture(Path(temp))
            empty = drawable()
            empty.find('./DrawableModelsHigh/Item/Geometries').clear()
            with self.assertRaisesRegex(ValueError, 'no High geometries'):
                tool.compose(drawable(), [empty, drawable(), drawable()], dirs, out)

    def test_older_skin_render_flag_is_corrected_only_in_derived_drawable(self):
        with tempfile.TemporaryDirectory(dir=QA) as temp:
            dirs, out = self.fixture(Path(temp), 1)
            old = drawable()
            old.find('./DrawableModelsHigh/Item/Flags').set('value', '0')
            before = ET.tostring(old)
            result, _ = tool.compose(drawable(), [old], dirs, out)
            self.assertEqual(result.find('./DrawableModelsHigh/Item/Flags').get('value'), '1')
            self.assertEqual(ET.tostring(old), before)
            old.find('./DrawableModelsHigh/Item/Flags').set('value', '4')
            with self.assertRaisesRegex(ValueError, 'metadata differs'):
                tool.compose(drawable(), [old], dirs, out)


if __name__ == '__main__':
    unittest.main()
