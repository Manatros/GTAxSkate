'use strict';
const policyLib = require(`${GetResourcePath(GetCurrentResourceName())}/policy.js`);
const generation = require('crypto').randomBytes(16).toString('hex');
let policy = null, revision = 0, constraints = null;
const players = new Map(), rates = new Map();
function load() {
  // Invalid reloads leave the last good policy intact; invalid startup stays disabled.
  const candidate = policyLib.compile(JSON.parse(LoadResourceFile(GetCurrentResourceName(), 'policy.json')));
  policy = candidate;
  constraints = policyLib.describe(policy);
  revision++;
  players.clear(); // Tightening policy discards previous player overrides.
  console.log(`[GTAxSkate] rules revision ${revision}, mode ${policy.mode}; skating adapter NOT included`);
}
function settings(id) {
  return policyLib.copy(players.get(id) ?? policy.defaults);
}
function send(id, error) {
  if (!policy) return;
  emitNet('gtaxskate:rules:state', id, {
    version: 1, generation, revision, mode: policy.mode, settings: settings(id),
    constraints: policyLib.copy(constraints), error: error ?? null,
  });
}
function permitted(id) {
  if (!Number.isInteger(id) || id <= 0 || !GetPlayerName(String(id))) return false;
  // Small burst for sync + menu interaction, sustained budget of four/s.
  const now = Date.now(), bucket = rates.get(id) ?? { tokens: 8, at: now };
  bucket.tokens = Math.min(8, bucket.tokens + Math.max(0, now - bucket.at) / 250);
  bucket.at = now;
  rates.set(id, bucket);
  if (bucket.tokens < 1) return false;
  bucket.tokens--;
  return true;
}
onNet('gtaxskate:rules:request', () => {
  const id = Number(global.source);
  if (policy && permitted(id)) send(id);
});
onNet('gtaxskate:rules:change', (expectedRevision, patch, expectedGeneration) => {
  // Identity comes from FiveM's event source; no client-supplied player/admin ID.
  const id = Number(global.source);
  if (!policy || !permitted(id)) return;
  if (expectedGeneration !== generation || expectedRevision !== revision) return send(id, { reason: 'stale-policy' });
  const result = policyLib.apply(policy, settings(id), patch);
  if (!result.ok) return send(id, { reason: result.reason, key: result.key });
  players.set(id, result.settings);
  send(id);
  // Server-local only. A future server skating adapter reads these effective values.
  emit('gtaxskate:rules:changed', id, settings(id), revision, generation);
});
on('playerDropped', () => {
  const id = Number(global.source);
  players.delete(id); rates.delete(id);
});
RegisterCommand('skaterules_reload', () => {
  try {
    load();
    for (const id of getPlayers()) {
      send(Number(id));
      emit('gtaxskate:rules:changed', Number(id), settings(Number(id)), revision, generation);
    }
  } catch (error) { console.error(`[GTAxSkate] reload rejected: ${error.message}`); }
}, true); // ACE command.skaterules_reload; no network event can reload policy.
exports('GetEffectiveSettings', playerId => {
  const id = Number(playerId);
  return policy && Number.isInteger(id) && id > 0 && GetPlayerName(String(id)) ? settings(id) : null;
});
exports('GetPolicy', () => policy ? {
  version: 1, catalogVersion: 1, runtimeConnected: false, generation,
  revision, mode: policy.mode, constraints: policyLib.copy(constraints),
} : null);
try { load(); } catch (error) { console.error(`[GTAxSkate] disabled: ${error.message}`); }
