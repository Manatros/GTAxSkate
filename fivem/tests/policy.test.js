'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { compile, apply, describe } = require('../gtaxskate_rules/policy');
const config = (mode, rules = {}, defaults = {}) => compile({ version: 1, mode, defaults, rules });

test('locked denies every player edit, including an explicitly allowed row', () => {
  const p = config('locked', { Pop: { adjustable: true } });
  assert.equal(apply(p, p.defaults, { Pop: 1.1 }).reason, 'server-locked');
  assert.equal(describe(p).Pop.adjustable, false);
});
test('all allows catalog settings but respects individual locks and engine-independent bounds', () => {
  const p = config('all', { BoardAim: { adjustable: false } });
  assert.equal(apply(p, p.defaults, { Pop: 1.1 }).settings.Pop, 1.1);
  assert.equal(apply(p, p.defaults, { BoardAim: true }).ok, false);
  assert.equal(apply(p, p.defaults, { Runtime: true }).ok, false);
  assert.equal(apply(p, p.defaults, { Pop: 999 }).ok, false);
});
test('selected is deny-by-default, with exact boolean and choice types', () => {
  const p = config('selected', { BoardAim: { adjustable: true }, Difficulty: { adjustable: true, values: ['Normal', 'Hardcore'] } });
  assert.equal(apply(p, p.defaults, { Pop: 1.1 }).ok, false);
  assert.equal(apply(p, p.defaults, { BoardAim: true }).ok, true);
  assert.equal(apply(p, p.defaults, { BoardAim: 'true' }).ok, false);
  assert.equal(apply(p, p.defaults, { Difficulty: 'Easy' }).ok, false);
});
test('ranges includes endpoints, rejects out-of-range edits and requires explicit numeric ranges', () => {
  const p = config('ranges', { Pop: { adjustable: true, min: 0.8, max: 1.2 } });
  for (const v of [0.8, 1, 1.2]) assert.equal(apply(p, p.defaults, { Pop: v }).ok, true);
  for (const v of [0.799, 1.201]) assert.equal(apply(p, p.defaults, { Pop: v }).ok, false);
  assert.throws(() => config('ranges', { Pop: { adjustable: true } }));
});
test('malformed numbers, coercion, arrays, prototype keys and unknown settings fail closed', () => {
  const p = config('all');
  for (const value of [NaN, Infinity, -Infinity, '1.2', null, {}, [], true]) {
    assert.equal(apply(p, p.defaults, { Pop: value }).ok, false);
  }
  for (const patch of [null, [], {}, 'Pop=1', JSON.parse('{"__proto__":{"polluted":true}}'), { constructor: 1 }]) {
    assert.equal(apply(p, p.defaults, patch).ok, false);
  }
  assert.equal({}.polluted, undefined);
});
test('presets apply atomically and never mutate current state on partial failure', () => {
  const p = config('selected', { Pop: { adjustable: true } });
  const before = structuredClone(p.defaults);
  assert.equal(apply(p, p.defaults, { Pop: 1.2, BoardAim: true }).ok, false);
  assert.deepEqual(p.defaults, before);
  const result = apply(p, p.defaults, { Pop: 1.2 });
  assert.equal(result.ok, true);
  assert.deepEqual(p.defaults, before);
  assert.equal(result.settings.BoardAim, before.BoardAim);
});
test('invalid admin policies are rejected rather than silently widened', () => {
  const invalid = [
    { version: 2, mode: 'all' }, { version: 1, mode: 'typo' },
    { version: 1, mode: 'all', unexpected: true },
    { version: 1, mode: 'all', defaults: { Unknown: 1 } },
    { version: 1, mode: 'all', defaults: { Pop: '1' } },
    { version: 1, mode: 'ranges', rules: { Pop: { adjustable: true, min: 2, max: 1 } } },
    { version: 1, mode: 'ranges', rules: { Pop: { adjustable: true, min: 1.1, max: 1.2 } } },
    { version: 1, mode: 'all', rules: { BoardAim: { adjustable: true, min: 0, max: 1 } } },
    { version: 1, mode: 'all', rules: { Difficulty: { adjustable: true, values: ['fake'] } } },
    { version: 1, mode: 'all', rules: { Pop: { adjustable: 'yes' } } },
  ];
  for (const input of invalid) assert.throws(() => compile(input), JSON.stringify(input));
});
test('compiled state and public constraints do not leak mutable references', () => {
  const input = { version: 1, mode: 'selected', defaults: {}, rules: { Difficulty: { adjustable: true, values: ['Normal', 'Hardcore'] } } };
  const p = compile(input);
  input.rules.Difficulty.values.push('Easy');
  const view = describe(p);
  view.Difficulty.values.push('Easy');
  assert.equal(apply(p, p.defaults, { Difficulty: 'Easy' }).ok, false);
});
