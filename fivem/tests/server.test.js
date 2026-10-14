'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const resource = path.resolve(__dirname, '../gtaxskate_rules');
const code = fs.readFileSync(path.join(resource, 'server.js'), 'utf8');
const policyLib = require(path.join(resource, 'policy.js'));
const plain = value => JSON.parse(JSON.stringify(value));
const openPolicy = () => ({ version: 1, mode: 'all' });

function boot(initial = openPolicy()) {
  let contents = initial === null ? null : JSON.stringify(initial);
  let now = 10000;
  const net = new Map(), events = new Map(), commands = new Map(), api = new Map();
  const online = new Set(['11', '22']);
  const sent = [], emitted = [], errors = [];
  const context = {
    source: undefined,
    require: filename => {
      if (filename === 'crypto') return require('node:crypto');
      assert.equal(filename, `${resource}/policy.js`);
      return policyLib;
    },
    GetResourcePath: () => resource,
    GetCurrentResourceName: () => 'gtaxskate_rules',
    LoadResourceFile: () => contents,
    GetPlayerName: id => online.has(String(id)) ? `Player ${id}` : null,
    // Deliberately expose only the JavaScript API, not Lua's GetPlayers helper.
    getPlayers: () => [...online],
    Date: { now: () => now },
    console: { log() {}, error: message => errors.push(message) },
    onNet: (name, callback) => net.set(name, callback),
    on: (name, callback) => events.set(name, callback),
    emitNet: (name, id, payload) => sent.push({ name, id, payload: plain(payload) }),
    emit: (name, ...args) => emitted.push({ name, args: plain(args) }),
    RegisterCommand: (name, callback, restricted) => commands.set(name, { callback, restricted }),
    exports: (name, callback) => api.set(name, callback),
  };
  context.global = context;
  vm.runInNewContext(code, context, { filename: 'server.js' });
  return {
    sent, emitted, errors, commands, api, online,
    advance: (ms = 300) => { now += ms; },
    policy: value => { contents = value === null ? null : typeof value === 'string' ? value : JSON.stringify(value); },
    request: id => { context.source = id; net.get('gtaxskate:rules:request')(); },
    change: (id, revision, patch, ...generationArgs) => {
      const generation = generationArgs.length ? generationArgs[0] : api.get('GetPolicy')()?.generation;
      context.source = id; net.get('gtaxskate:rules:change')(revision, patch, generation);
    },
    drop: id => { context.source = id; events.get('playerDropped')(); },
    reload: () => commands.get('skaterules_reload').callback(0, []),
    settings: id => plain(api.get('GetEffectiveSettings')(id)),
  };
}

test('network source isolates player overrides and exports return detached copies', () => {
  const h = boot();
  h.change('11', 1, { Pop: 1.2 });
  assert.equal(h.settings(11).Pop, 1.2);
  assert.equal(h.settings(22).Pop, 1);
  assert.equal(h.sent.at(-1).id, 11);
  assert.deepEqual(h.emitted.at(-1).args.slice(0, 1), [11]);
  const exported = h.api.get('GetEffectiveSettings')(11);
  exported.Pop = 4;
  assert.equal(h.settings(11).Pop, 1.2);
  h.advance();
  h.change(11, 1, { playerId: 22, Pop: 2 });
  assert.equal(h.settings(11).Pop, 1.2);
  assert.equal(h.settings(22).Pop, 1);
  assert.equal(h.sent.at(-1).payload.error.key, 'playerId');
});

test('stale revisions reject the entire patch and return current policy', () => {
  const h = boot();
  h.change(11, 0, { Pop: 2 });
  assert.equal(h.settings(11).Pop, 1);
  assert.equal(h.sent.at(-1).payload.revision, 1);
  assert.equal(h.sent.at(-1).payload.error.reason, 'stale-policy');
  assert.equal(h.emitted.length, 0);
});

test('resource restart rejects old or missing generation despite matching revision', () => {
  const before = boot(), after = boot();
  const oldPolicy = before.api.get('GetPolicy')();
  const newPolicy = after.api.get('GetPolicy')();
  assert.equal(oldPolicy.revision, 1);
  assert.equal(newPolicy.revision, 1);
  assert.notEqual(oldPolicy.generation, newPolicy.generation);
  after.change(11, 1, { Pop: 1.2 }, oldPolicy.generation);
  assert.equal(after.settings(11).Pop, 1);
  assert.equal(after.sent.at(-1).payload.error.reason, 'stale-policy');
  assert.equal(after.sent.at(-1).payload.generation, newPolicy.generation);
  after.change(11, 1, { Pop: 1.2 }, undefined);
  assert.equal(after.settings(11).Pop, 1);
  assert.equal(after.sent.at(-1).payload.error.reason, 'stale-policy');
  after.change(11, 1, { Pop: 1.2 }, newPolicy.generation);
  assert.equal(after.settings(11).Pop, 1.2);
});

test('invalid reload retains revision and existing player overrides', () => {
  const h = boot();
  h.change(11, 1, { Pop: 1.4 });
  const previousSent = h.sent.length;
  h.policy({ version: 1, mode: 'ranges', defaults: { Pop: Infinity } });
  h.reload();
  assert.equal(h.settings(11).Pop, 1.4);
  assert.equal(h.api.get('GetPolicy')().revision, 1);
  assert.equal(h.sent.length, previousSent);
  assert.match(h.errors.at(-1), /reload rejected/);
});

test('successful reload clears overrides and pushes all connected clients and adapters', () => {
  const h = boot();
  assert.equal(h.commands.get('skaterules_reload').restricted, true);
  h.change(11, 1, { Pop: 1.4 });
  h.sent.length = 0;
  h.emitted.length = 0;
  h.policy({ version: 1, mode: 'locked', defaults: { Pop: 0.9 } });
  h.reload();
  assert.equal(h.settings(11).Pop, 0.9);
  assert.equal(h.settings(22).Pop, 0.9);
  assert.deepEqual(h.sent.map(message => message.id), [11, 22]);
  assert.ok(h.sent.every(message => message.payload.revision === 2 && message.payload.mode === 'locked'));
  assert.deepEqual(h.emitted.map(message => [message.args[0], message.args[2]]), [[11, 2], [22, 2]]);
  assert.equal(h.errors.length, 0);
});

test('disconnect removes overrides and rate state before an ID is reused', () => {
  const h = boot();
  h.change(11, 1, { Pop: 1.4 });
  for (let i = 0; i < 7; i++) h.request(11); // Exhaust this connection's burst budget.
  h.drop('11');
  h.online.delete('11');
  assert.equal(h.api.get('GetEffectiveSettings')(11), null);
  h.online.add('11');
  h.change(11, 1, { Pop: 1.1 }); // Same clock: only cleanup lets this request through.
  assert.equal(h.settings(11).Pop, 1.1);
});

test('locked, out-of-range, unknown and non-finite changes do not partially apply', () => {
  const h = boot({ version: 1, mode: 'ranges', rules: { Pop: { adjustable: true, min: 0.8, max: 1.2 } } });
  for (const patch of [{ Pop: 1.1, BoardAim: true }, { Pop: 1.3 }, { Pop: NaN }, { Pop: Infinity }, { unknown: true }]) {
    h.change(11, 1, patch);
    assert.equal(h.settings(11).Pop, 1);
    assert.equal(h.settings(11).BoardAim, false);
    assert.ok(h.sent.at(-1).payload.error);
    h.advance();
  }
  assert.equal(h.emitted.length, 0);
});

test('rate limit is per player and invalid identities cannot create state', () => {
  const h = boot();
  for (let i = 0; i < 8; i++) h.request(11);
  h.change(11, 1, { Pop: 1.2 });
  assert.equal(h.settings(11).Pop, 1);
  h.change(22, 1, { Pop: 1.2 });
  assert.equal(h.settings(22).Pop, 1.2);
  const count = h.sent.length;
  for (const id of [0, -1, 1.5, 999, 'invalid']) h.change(id, 1, { Pop: 2 });
  assert.equal(h.sent.length, count);
  h.advance(250);
  h.change(11, 1, { Pop: 1.2 });
  assert.equal(h.settings(11).Pop, 1.2);
});

test('missing or malformed startup policy disables exports and requests until a valid reload', () => {
  for (const initial of [null, { version: 99, mode: 'all' }]) {
    const h = boot(initial);
    assert.equal(h.api.get('GetPolicy')(), null);
    assert.equal(h.api.get('GetEffectiveSettings')(11), null);
    h.request(11);
    h.change(11, 1, { Pop: 2 });
    assert.equal(h.sent.length, 0);
    assert.match(h.errors[0], /disabled/);
    h.policy(openPolicy());
    h.reload();
    assert.equal(h.api.get('GetPolicy')().revision, 1);
    assert.equal(h.settings(11).Pop, 1);
  }
});
