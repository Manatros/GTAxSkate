'use strict';

// Pure server policy: adapters must consume effective server values, never raw
// client settings. Bounds are package validation limits, not verified FiveM
// engine support; a future adapter must declare which settings it implements.
const schema = Object.freeze({
  Difficulty: { values: ['Easy', 'Normal', 'Hardcore'], default: 'Normal' },
  CameraView: { values: ['Skate', 'Near', 'Middle', 'Far', 'FirstPerson', 'Head'], default: 'Near' },
  Pop: { min: 0.5, max: 5, default: 1 },
  GrindPop: { min: 0.5, max: 5, default: 1 },
  PushSpeed: { min: 0.5, max: 3, default: 1 },
  PushPower: { min: 0.5, max: 3, default: 1 },
  Braking: { min: 0.5, max: 3, default: 1 },
  Steering: { min: 0.5, max: 3, default: 1 },
  Wobble: { min: 0.1, max: 3, default: 1 },
  OffboardJump: { min: 0.5, max: 3, default: 1 },
  Grip: { min: 0.5, max: 3, default: 1 },
  TurnPower: { min: 0.5, max: 3, default: 1 },
  ManualDrag: { min: 0.1, max: 3, default: 1 },
  TruckTightness: { min: 0, max: 1, default: 0.6 },
  WheelHardness: { min: 0, max: 1, default: 0.6 },
  HippieJump: { min: 0.8, max: 2.5, default: 1 },
  VehiclePush: { min: 0, max: 1, default: 0 },
  BoardAim: { default: false }, PedsSurvive: { default: true },
  PedLaunch: { default: false }, VehicleDamage: { default: false },
  Injuries: { default: true }, Stamina: { default: true },
  HallOfMeat: { default: true }, TrickAnalyzer: { default: false },
  KeyboardHints: { default: true }, HoldFakie: { default: false },
});

const own = (o, k) => Object.prototype.hasOwnProperty.call(o, k);
const record = o => o !== null && typeof o === 'object' && !Array.isArray(o);
const copy = o => JSON.parse(JSON.stringify(o));
function validValue(key, value) {
  const s = schema[key];
  if (!own(schema, key)) return false;
  if (s.values) return s.values.includes(value);
  if (typeof s.default === 'boolean') return typeof value === 'boolean';
  return typeof value === 'number' && Number.isFinite(value) && value >= s.min && value <= s.max;
}
function fields(object, allowed, name) {
  if (!record(object) || Object.keys(object).some(k => !allowed.includes(k))) throw Error(`Invalid ${name}`);
}
function compile(input) {
  fields(input, ['version', 'mode', 'defaults', 'rules'], 'policy fields');
  if (input.version !== 1 || !['all', 'selected', 'ranges', 'locked'].includes(input.mode)) throw Error('Invalid version/mode');
  const defaults = {}, rules = {};
  for (const [k, s] of Object.entries(schema)) defaults[k] = s.default;
  fields(input.defaults ?? {}, Object.keys(schema), 'defaults');
  for (const [k, value] of Object.entries(input.defaults ?? {})) {
    if (!validValue(k, value)) throw Error(`Invalid default: ${k}`);
    defaults[k] = value;
  }
  fields(input.rules ?? {}, Object.keys(schema), 'rules');
  for (const [k, r] of Object.entries(input.rules ?? {})) {
    fields(r, ['adjustable', 'min', 'max', 'values'], `rule ${k}`);
    if (typeof r.adjustable !== 'boolean') throw Error(`Missing adjustable: ${k}`);
    const s = schema[k];
    if (own(r, 'min') || own(r, 'max')) {
      if (!own(s, 'min') || !Number.isFinite(r.min) || !Number.isFinite(r.max) ||
          r.min > r.max || r.min < s.min || r.max > s.max) throw Error(`Invalid range: ${k}`);
    }
    if (own(r, 'values') && (!Array.isArray(r.values) || !r.values.length ||
        !s.values || r.values.some(v => !validValue(k, v)))) throw Error(`Invalid choices: ${k}`);
    if (input.mode === 'ranges' && r.adjustable && own(s, 'min') && !own(r, 'min')) {
      throw Error(`Explicit range required: ${k}`);
    }
    if ((own(r, 'min') && (defaults[k] < r.min || defaults[k] > r.max)) ||
        (r.values && !r.values.includes(defaults[k]))) throw Error(`Default outside rule: ${k}`);
    rules[k] = copy(r);
  }
  // Compiled state is detached from the admin's mutable input.
  return { version: 1, mode: input.mode, defaults, rules };
}
function constraint(policy, key) {
  if (!own(schema, key)) return null;
  const rule = policy.rules[key] ?? {};
  const editable = policy.mode === 'all' ? rule.adjustable !== false :
    policy.mode !== 'locked' && rule.adjustable === true;
  return copy({ ...schema[key], ...rule, adjustable: editable });
}
function apply(policy, current, patch) {
  if (!record(patch) || !Object.keys(patch).length || Object.keys(patch).length > Object.keys(schema).length) {
    return { ok: false, reason: 'invalid-patch' };
  }
  // Reject the entire request; a preset may not sneak a forbidden field through.
  for (const [key, value] of Object.entries(patch)) {
    const rule = constraint(policy, key);
    if (!rule || !rule.adjustable) return { ok: false, reason: 'server-locked', key };
    if (!validValue(key, value) || (own(rule, 'min') && (value < rule.min || value > rule.max)) ||
        (rule.values && !rule.values.includes(value))) return { ok: false, reason: 'outside-range', key };
  }
  return { ok: true, settings: { ...current, ...patch } };
}
function describe(policy) {
  return Object.fromEntries(Object.keys(schema).map(k => [k, constraint(policy, k)]));
}
module.exports = { compile, apply, describe, copy };
