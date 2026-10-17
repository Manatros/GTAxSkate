import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('worker', Path(__file__).with_name('board-composition-worker.py'))
worker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(worker)
QA = Path(__file__).resolve().parents[1] / 'build' / 'board-composition-tests'
QA.mkdir(parents=True, exist_ok=True)


class RequestTests(unittest.TestCase):
    def request(self, folder, models):
        path = folder / ('skatev_c_1234_' + worker.model_key(models) + '.request')
        path.write_text(''.join(m + '\n' for m in models), encoding='ascii')
        return path

    def test_full_split_and_partial_vectors_and_order(self):
        for models in (['skatev_w_default'], ['skatev_d_default', 'skatev_g_black'],
                       ['skatev_d_default', 'skatev_tb_red', 'skatev_th_black', 'skatev_w_default', 'skatev_g_black']):
            with tempfile.TemporaryDirectory(dir=QA) as tmp:
                path = self.request(Path(tmp), models)
                self.assertEqual(worker.parse_request(path)[1], models)
                if len(models) > 1:
                    self.assertNotEqual(worker.model_key(models), worker.model_key(models[::-1]))

    def test_invalid_requests_and_key_collisions_are_refused(self):
        vectors = [[], ['../escape', 'skatev_w_default'],
                   ['skatev_d_default', 'skatev_d_default'], ['skatev_x_unknown', 'skatev_w_default'],
                   ['skatev_d_' + 'x' * 129, 'skatev_w_default']]
        for models in vectors:
            with self.subTest(models=models), tempfile.TemporaryDirectory(dir=QA) as tmp:
                path = self.request(Path(tmp), models)
                with self.assertRaises(ValueError):
                    worker.parse_request(path)
        with tempfile.TemporaryDirectory(dir=QA) as tmp:
            path = self.request(Path(tmp), ['skatev_d_default', 'skatev_w_default'])
            path.write_text('skatev_d_changed\nskatev_w_default\n', encoding='ascii')
            with self.assertRaisesRegex(ValueError, 'key mismatch'):
                worker.parse_request(path)

    def test_sources_cannot_escape_folder(self):
        with tempfile.TemporaryDirectory(dir=QA) as tmp:
            root = Path(tmp)
            parts = root / 'parts'
            parts.mkdir()
            (root / 'outside.ydr').write_bytes(b'outside')
            with self.assertRaises(ValueError):
                worker.source_at(parts, '../outside.ydr')

    def test_no_publication_for_missing_input_or_prior_partial_output(self):
        with tempfile.TemporaryDirectory(dir=QA) as tmp:
            root = Path(tmp)
            path = self.request(root, ['skatev_d_default', 'skatev_w_default'])
            with self.assertRaises(FileNotFoundError):
                worker.build(path, root / 'missing.ydr', root, root / 'rage.exe')
            self.assertFalse(path.with_suffix('.ready').exists())
            path.with_suffix('.ydr').write_bytes(b'partial')
            with self.assertRaisesRegex(ValueError, 'Incomplete prior'):
                worker.build(path, root / 'missing.ydr', root, root / 'rage.exe')

    def test_existing_publication_never_overwrites_registered_files(self):
        with tempfile.TemporaryDirectory(dir=QA) as tmp:
            root = Path(tmp)
            models = ['skatev_d_default', 'skatev_w_default']
            path = self.request(root, models)
            path.with_suffix('.ready').write_text(path.read_text(), encoding='ascii')
            with self.assertRaises(FileNotFoundError):
                worker.build(path, root / 'missing.ydr', root, root / 'rage.exe')
            for ext in ('.ydr', '.ytyp'):
                path.with_suffix(ext).write_bytes(b'verified test resource')
            manifest = {'model': path.stem, 'models': models,
                        'ydrSha256': worker.composer.sha(path.with_suffix('.ydr')),
                        'ytypSha256': worker.composer.sha(path.with_suffix('.ytyp'))}
            path.with_suffix('.json').write_text(json.dumps(manifest), encoding='utf-8')
            self.assertFalse(worker.build(path, root / 'missing.ydr', root, root / 'rage.exe'))
            path.with_suffix('.ydr').write_bytes(b'corrupted')
            with self.assertRaisesRegex(ValueError, 'hash/identity'):
                worker.build(path, root / 'missing.ydr', root, root / 'rage.exe')
            path.with_suffix('.ready').write_text('wrong vector', encoding='ascii')
            with self.assertRaisesRegex(ValueError, 'collision'):
                worker.build(path, root / 'missing.ydr', root, root / 'rage.exe')


if __name__ == '__main__':
    unittest.main()
