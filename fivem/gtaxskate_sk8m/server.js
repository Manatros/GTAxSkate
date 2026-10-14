'use strict';
const RULES = 'gtaxskate_rules';
const capabilities = Object.freeze(['PushSpeed', 'PushPower', 'Braking', 'Steering']);
const rates = new Map();
function snapshot(id) {
  if (GetResourceState(RULES) !== 'started' || !GetPlayerName(String(id))) return null;
  try {
    const policy = exports[RULES].GetPolicy();
    const effective = exports[RULES].GetEffectiveSettings(id);
    if (!policy || policy.version !== 1 || policy.catalogVersion !== 1 || !effective ||
        typeof policy.generation !== 'string' || !Number.isInteger(policy.revision)) return null;
    // Refuse a generic, unfiltered rules deployment: it could advertise editable
    // options this adapter cannot implement. The staged wrapper is mandatory.
    if (!policy.constraints || Object.keys(policy.constraints).length !== capabilities.length ||
        capabilities.some(key => !Object.prototype.hasOwnProperty.call(policy.constraints, key))) return null;
    const settings = {};
    for (const key of capabilities) {
      const value = effective[key];
      if (!Number.isFinite(value) || value < 0.5 || value > 3) return null;
      settings[key] = value;
    }
    return { version: 1, generation: policy.generation, revision: policy.revision,
      settings, capabilities: [...capabilities], physics: 'client-owned-lua-fallback' };
  } catch (_) { return null; }
}
function send(id) { emitNet('gtaxskate:sk8m:state', id, snapshot(id)); }
function all() { for (const id of getPlayers()) send(Number(id)); }
onNet('gtaxskate:sk8m:sync', () => {
  const id = Number(global.source), now = Date.now();
  if (!Number.isInteger(id) || id <= 0 || !GetPlayerName(String(id))) return;
  if (now - (rates.get(id) ?? 0) < 500) return;
  rates.set(id, now); send(id);
});
on('gtaxskate:rules:changed', id => send(Number(id)));
on('playerDropped', () => rates.delete(Number(global.source)));
on('onResourceStart', resource => { if (resource === RULES || resource === GetCurrentResourceName()) all(); });
on('onResourceStop', resource => {
  if (resource === RULES || resource === GetCurrentResourceName()) {
    // Explicitly invalidate before stop completes; do not rely on resource-state timing.
    for (const id of getPlayers()) emitNet('gtaxskate:sk8m:state', Number(id), null);
  }
});
exports('GetSnapshot', id => {
  const player = Number(id);
  return Number.isInteger(player) && player > 0 ? snapshot(player) : null;
});
