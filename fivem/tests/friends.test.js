'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const crypto = require('node:crypto');
const prepareLib = require('../gtaxskate_sk8m/prepare.js');
const core = require('../gtaxskate_rules/policy.js');
const root = path.resolve(__dirname, '../..');
const plain = value => JSON.parse(JSON.stringify(value));
function adapter() {
  const events = new Map(), net = new Map(), api = new Map(), sent = [];
  const resourceStates = { gtaxskate_rules: 'started' };
  let policy = { version: 1, catalogVersion: 1, generation: 'session-a', revision: 1,
    constraints: { PushSpeed: {}, PushPower: {}, Braking: {}, Steering: {} } };
  let settings = { PushSpeed: 1, PushPower: 1, Braking: 1, Steering: 1, Pop: 4 };
  const exportsMock = (name, callback) => api.set(name, callback);
  exportsMock.gtaxskate_rules = { GetPolicy: () => policy, GetEffectiveSettings: () => settings };
  const context = {
    source: undefined,
    GetResourceState: name => resourceStates[name] ?? 'missing',
    GetPlayerName: id => ['11', '22'].includes(id) ? `Player ${id}` : null,
    GetCurrentResourceName: () => 'gtaxskate_sk8m',
    getPlayers: () => ['11', '22'],
    exports: exportsMock,
    on: (name, fn) => events.set(name, fn),
    onNet: (name, fn) => net.set(name, fn),
    emitNet: (name, id, value) => sent.push({ name, id, value: plain(value) }),
    Date: { now: () => 10000 },
  };
  context.global = context;
  vm.runInNewContext(fs.readFileSync(path.join(root, 'fivem/gtaxskate_sk8m/server.js'), 'utf8'), context);
  return {
    api, sent, resourceStates,
    policy: value => { policy = value; },
    settings: value => { settings = value; },
    event: (name, ...args) => events.get(name)(...args),
    sync: id => { context.source = id; net.get('gtaxskate:sk8m:sync')(); },
  };
}
function narrowPolicy() {
  const module = { exports: {} };
  vm.runInNewContext(fs.readFileSync(path.join(root, 'fivem/gtaxskate_sk8m/rules-policy.js'), 'utf8'), {
    module, require: name => { assert.equal(name, './policy-core.js'); return core; },
  });
  return module.exports;
}

test('fallback transport exposes exactly four validated settings and fails closed without rules', () => {
  const h = adapter();
  const result = h.api.get('GetSnapshot')(11);
  assert.deepEqual(Object.keys(result.settings).sort(), ['Braking', 'PushPower', 'PushSpeed', 'Steering']);
  assert.equal(result.physics, 'client-owned-lua-fallback');
  assert.equal(result.generation, 'session-a');
  result.settings.PushSpeed = 2;
  assert.equal(h.api.get('GetSnapshot')(11).settings.PushSpeed, 1);
  assert.equal(h.api.get('GetSnapshot')(999), null);
  h.resourceStates.gtaxskate_rules = 'stopped';
  assert.equal(h.api.get('GetSnapshot')(11), null);
  h.resourceStates.gtaxskate_rules = 'started';
  h.policy(null);
  assert.equal(h.api.get('GetSnapshot')(11), null);
  h.policy({ version: 1, catalogVersion: 2, generation: 'session-a', revision: 1 });
  assert.equal(h.api.get('GetSnapshot')(11), null);
  h.policy({ version: 1, catalogVersion: 1, generation: 'session-a', revision: 1,
    constraints: { PushSpeed: {}, PushPower: {}, Braking: {}, Steering: {}, Pop: {} } });
  assert.equal(h.api.get('GetSnapshot')(11), null);
});

test('transport rejects malformed supported values instead of partially applying settings', () => {
  const h = adapter();
  for (const value of [NaN, Infinity, '1', 0.49, 3.01, undefined]) {
    h.settings({ PushSpeed: value, PushPower: 1, Braking: 1, Steering: 1 });
    assert.equal(h.api.get('GetSnapshot')(11), null);
  }
});

test('transport uses event source, limits sync traffic, pushes edits and invalidates lifecycle', () => {
  const h = adapter();
  h.sync(11);
  h.sync(11);
  h.sync(22);
  h.sync(999);
  assert.deepEqual(h.sent.map(message => message.id), [11, 22]);
  h.event('gtaxskate:rules:changed', 11);
  assert.equal(h.sent.at(-1).id, 11);
  h.event('onResourceStop', 'gtaxskate_rules');
  assert.deepEqual(h.sent.slice(-2).map(message => [message.id, message.value]), [[11, null], [22, null]]);
});

test('narrow policy hides unsupported catalog and prevents all mode from unlocking it', () => {
  const lib = narrowPolicy();
  const policy = lib.compile({ version: 1, mode: 'all', rules: { Pop: { adjustable: true } } });
  assert.deepEqual(Object.keys(lib.describe(policy)).sort(), ['Braking', 'PushPower', 'PushSpeed', 'Steering']);
  assert.equal(lib.apply(policy, policy.defaults, { PushPower: 1.2 }).ok, true);
  assert.equal(lib.apply(policy, policy.defaults, { Pop: 2 }).ok, false);
  assert.equal(lib.apply(policy, policy.defaults, { PushPower: 1.2, Pop: 2 }).ok, false);
});

test('packager pins and hashes a fresh workspace-only fallback without engine or private assets', { skip: !fs.existsSync(path.join(root, 'external/sk8m/.git')) && 'Optional pinned SK8M checkout absent; transport and policy tests still run' }, () => {
  const destination = path.join(root, 'build/fivem-friends', `automated-${crypto.randomUUID()}`);
  const manifest = prepareLib.prepare(root, destination);
  assert.equal(manifest.sourceCommit, prepareLib.PIN);
  assert.equal(manifest.authoritativePhysics, false);
  assert.equal(Object.keys(manifest.outputHashes).some(name => /engine_server|engine\.lua|skate3_assets|\.wasm/.test(name)), false);
  for (const [name, hash] of Object.entries(manifest.outputHashes)) {
    const actual = crypto.createHash('sha256').update(fs.readFileSync(path.join(destination, name))).digest('hex');
    assert.equal(actual, hash, name);
  }
  const client = fs.readFileSync(path.join(destination, 'resources/skateboard/client.lua'), 'utf8');
  assert.match(client, /local function mount\(from\)\n    if not Friends\.authorize\(\) then return end/);
  assert.match(client, /if not Friends\.granted\(\) then Friends\.stop\(\) return end\n    riding = true/);
  assert.match(client, /Friends\.stop = function\(\)/);
  assert.doesNotMatch(client, /RegisterNetEvent\('skateboard:boardReply'/);
  const server = fs.readFileSync(path.join(destination, 'resources/skateboard/server.lua'), 'utf8');
  assert.match(server, /state\.generation == generation and state\.revision == revision/);
  assert.match(server, /pcall\(ownsBoard, src\)/);
  assert.match(server, /type\(token\) ~= 'string'/);
  assert.match(server, /#token > 100/);
  const policy = require(path.join(destination, 'resources/gtaxskate_rules/policy.js'));
  const compiled = policy.compile({ version: 1, mode: 'all' });
  assert.equal(policy.apply(compiled, compiled.defaults, { Pop: 2 }).ok, false);
  assert.throws(() => prepareLib.prepare(root, destination), /never overwrites/);
  assert.throws(() => prepareLib.prepare(root, path.join(root, 'external/sk8m')), /Destination/);
});

test('overlay refuses changed mount/command anchors rather than guessing compatibility', () => {
  assert.throws(() => prepareLib.overlayClient('unrecognized runtime'), /anchor mismatch/);
  assert.throws(() => prepareLib.overlayServer('unrecognized runtime'), /anchor mismatch/);
});

test('developer server preset unlocks only implemented multipliers and rejects unsupported edits', () => {
  const lib = narrowPolicy();
  const config = JSON.parse(fs.readFileSync(path.join(root, 'fivem/gtaxskate_sk8m/policy-developer.json'), 'utf8'));
  const policy = lib.compile(config);
  for (const key of ['PushSpeed', 'PushPower', 'Braking', 'Steering']) {
    assert.equal(lib.apply(policy, policy.defaults, { [key]: 1.1 }).ok, true);
  }
  assert.equal(lib.apply(policy, policy.defaults, { PushSpeed: 99 }).ok, false);
  assert.equal(lib.apply(policy, policy.defaults, { Pop: 2 }).ok, false);
});
