'use strict';
// Staging overlay: keep the package policy implementation and expose only the
// settings implemented by this exact fallback adapter. Unsupported rules cannot
// become adjustable through `all` or an administrator's generic catalog file.
const core = require('./policy-core.js');
const supported = new Set(['PushSpeed', 'PushPower', 'Braking', 'Steering']);
function compile(input) {
  const policy = core.compile(input);
  for (const key of Object.keys(core.describe(policy))) {
    if (!supported.has(key)) policy.rules[key] = { adjustable: false };
  }
  return policy;
}
function describe(policy) {
  return Object.fromEntries(Object.entries(core.describe(policy))
    .filter(([key]) => supported.has(key)));
}
module.exports = { compile, describe, apply: core.apply, copy: core.copy };
