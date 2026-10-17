"""Real RAGE validation with installed player-owned assets, staged only."""
import argparse
import importlib.util
import json
import subprocess
import sys
from pathlib import Path

spec = importlib.util.spec_from_file_location('worker', Path(__file__).with_name('board-composition-worker.py'))
worker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(worker)

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--assets', required=True, type=Path)
parser.add_argument('--rage', required=True, type=Path)
parser.add_argument('--out', required=True, type=Path)
args = parser.parse_args()
args.out.mkdir(parents=True, exist_ok=False)
parts = args.assets / 'private' / 'board-parts'
native = args.assets / 'private' / 'board-native' / 'skatev_board.ydr'
selected = ['skatev_d_2c7f381700081a5b', 'skatev_t_2c7f3817001716c1', 'skatev_w_2c7f381700171ab4']
setups = [selected, [selected[0], 'skatev_tb_blackg', 'skatev_th_blackm', selected[2], 'skatev_g_a841_black'],
          [selected[2]], [selected[0], 'skatev_g_a841_black']]
results = []
for models in setups:
    stem = 'skatev_c_16_' + worker.model_key(models)
    request = args.out / (stem + '.request')
    request.write_text(''.join(m + '\n' for m in models), encoding='ascii')
    assert worker.build(request, native, parts, args.rage)
    before = [worker.composer.sha(request.with_suffix(ext)) for ext in ('.ydr', '.ytyp', '.json')]
    assert not worker.build(request, native, parts, args.rage)  # Return to an already built selection.
    assert before == [worker.composer.sha(request.with_suffix(ext)) for ext in ('.ydr', '.ytyp', '.json')]
    result = json.loads(request.with_suffix('.json').read_text(encoding='utf-8'))
    results.append({'model': stem, 'parts': len(models), 'geometries': result['geometryCount'], 'reusedWithoutWrites': True})
(args.out / 'results.json').write_text(json.dumps(results, indent=2), encoding='utf-8')
cli = args.out / 'worker-cli'
cli.mkdir()
(cli / 'active-session').write_text('16', encoding='ascii')
for request in args.out.glob('*.request'):
    (cli / request.name).write_bytes(request.read_bytes())
command = [sys.executable, str(Path(__file__).with_name('board-composition-worker.py')), '--root', str(cli),
           '--native', str(native), '--parts', str(parts), '--rage', str(args.rage)]
subprocess.run(command, check=True, timeout=120)
assert len(list(cli.glob('*.ready'))) == len(setups), 'One-shot worker did not drain active requests'
assert not list(cli.glob('*.error')), 'Worker CLI reported failure'
subprocess.run(command, check=True, timeout=30)  # Stale lock file must not block a new process.
print(json.dumps(results))
