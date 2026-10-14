'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const code = fs.readFileSync(path.resolve(__dirname, '../gtaxskate_rules/client.js'), 'utf8');
const plain = value => JSON.parse(JSON.stringify(value));
const state = () => ({
  version: 1, generation: 'server-session-a', revision: 3, mode: 'ranges',
  settings: { Pop: 1, BoardAim: false, Difficulty: 'Normal' },
  constraints: {
    Pop: { adjustable: true, min: 0.8, max: 1.2 },
    BoardAim: { adjustable: false },
    Difficulty: { adjustable: true, values: ['Normal', 'Hardcore'] },
  }, error: null,
});

function boot() {
  const net = new Map(), local = new Map(), commands = new Map(), api = new Map();
  const sent = [], emitted = [];
  const context = {
    source: undefined,
    GetCurrentResourceName: () => 'gtaxskate_rules',
    onNet: (name, fn) => net.set(name, fn),
    on: (name, fn) => local.set(name, fn),
    RegisterCommand: (name, fn) => commands.set(name, fn),
    exports: (name, fn) => api.set(name, fn),
    emitNet: (name, ...args) => sent.push({ name, args: plain(args) }),
    emit: (name, payload) => emitted.push({ name, payload }),
    console: { log() {} },
  };
  context.global = context;
  vm.runInNewContext(code, context, { filename: 'client.js' });
  return {
    api, sent, emitted,
    receive: (source, value) => { context.source = source; net.get('gtaxskate:rules:state')(value); },
    start: name => local.get('onClientResourceStart')(name),
    command: (name, args = []) => commands.get(name)(0, args),
  };
}

test('server-origin guard accepts numeric and string server sources only', () => {
  for (const source of [65535, '65535']) {
    const h = boot();
    h.receive(source, state());
    assert.equal(h.api.get('GetRules')().generation, 'server-session-a');
    assert.equal(h.emitted.length, 1);
  }
  for (const source of [undefined, null, 0, '0', 11, '11', 'invalid', NaN]) {
    const h = boot();
    h.receive(source, state());
    assert.equal(h.api.get('GetRules')(), null);
    assert.equal(h.emitted.length, 0);
  }
});

test('unsynced client cannot submit settings and startup requests only its resource', () => {
  const h = boot();
  h.api.get('RequestSettings')({ Pop: 1.1 });
  assert.equal(h.sent.length, 0);
  h.start('another_resource');
  assert.equal(h.sent.length, 0);
  h.start('gtaxskate_rules');
  h.command('skateset', ['Pop', '1.1']);
  h.command('skaterules');
  assert.equal(h.sent.length, 3);
  assert.ok(h.sent.every(message => message.name === 'gtaxskate:rules:request'));
});

test('commands and menu exports attach current revision and generation', () => {
  const h = boot();
  h.receive(65535, state());
  h.command('skateset', ['Pop', '1.1']);
  assert.deepEqual(h.sent.at(-1), {
    name: 'gtaxskate:rules:change', args: [3, { Pop: 1.1 }, 'server-session-a'],
  });
  h.command('skateset', ['BoardAim', 'true']);
  assert.deepEqual(h.sent.at(-1).args[1], { BoardAim: true });
  h.command('skateset', ['Difficulty', 'Hardcore']);
  assert.deepEqual(h.sent.at(-1).args[1], { Difficulty: 'Hardcore' });
  const updated = state();
  updated.revision = 1;
  updated.generation = 'server-session-b';
  h.receive('65535', updated);
  h.api.get('RequestSettings')({ Pop: 1.2 });
  assert.deepEqual(h.sent.at(-1).args, [1, { Pop: 1.2 }, 'server-session-b']);
});

test('unknown command keys and invalid protocol versions do not submit settings', () => {
  const h = boot();
  h.receive(65535, { ...state(), version: 99 });
  assert.equal(h.api.get('GetRules')(), null);
  h.receive(65535, state());
  h.command('skateset', ['unknown', '1']);
  h.command('skateset', ['Pop']);
  assert.equal(h.sent.length, 0);
});

test('rule exports and emitted update snapshots are detached from cached state', () => {
  const h = boot();
  h.receive(65535, state());
  const first = h.api.get('GetRules')();
  first.settings.Pop = 9;
  first.constraints.Pop.min = 9;
  h.emitted[0].payload.settings.Pop = 8;
  h.emitted[0].payload.constraints.Pop.max = 8;
  const next = h.api.get('GetRules')();
  assert.equal(next.settings.Pop, 1);
  assert.equal(next.constraints.Pop.min, 0.8);
  assert.equal(next.constraints.Pop.max, 1.2);
});
