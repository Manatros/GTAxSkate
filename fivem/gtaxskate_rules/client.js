'use strict';
let state = null;
onNet('gtaxskate:rules:state', incoming => {
  if (Number(global.source) !== 65535) return; // Accept state only from the server.
  if (!incoming || incoming.version !== 1) return;
  state = incoming;
  if (state.error) console.log(`[GTAxSkate] ${state.error.reason}: ${state.error.key ?? ''}`);
  emit('gtaxskate:rules:updated', JSON.parse(JSON.stringify(state)));
});
on('onClientResourceStart', resource => {
  if (resource === GetCurrentResourceName()) emitNet('gtaxskate:rules:request');
});
RegisterCommand('skaterules', () => {
  if (!state) return emitNet('gtaxskate:rules:request');
  console.log(`[GTAxSkate] ${state.mode}, revision ${state.revision}. Policy prototype; skating adapter not included.`);
  for (const [key, rule] of Object.entries(state.constraints)) {
    const range = rule.values ? rule.values.join('|') : rule.min !== undefined ? `${rule.min}..${rule.max}` : 'boolean';
    console.log(`${key} = ${state.settings[key]} (${rule.adjustable ? range : 'server locked'})`);
  }
}, false);
RegisterCommand('skateset', (_, args) => {
  if (!state) return emitNet('gtaxskate:rules:request');
  const [key, text] = args;
  if (!key || text === undefined || !Object.prototype.hasOwnProperty.call(state.constraints, key)) {
    return console.log('Usage: skateset <setting> <value>. Use skaterules to list settings.');
  }
  const rule = state.constraints[key];
  const value = rule.values ? text : text === 'true' ? true : text === 'false' ? false : Number(text);
  emitNet('gtaxskate:rules:change', state.revision, { [key]: value }, state.generation);
}, false);
// Future FiveM menu submits every change/preset through this export. The
// server validates it atomically; this client cache is only for presentation.
exports('RequestSettings', patch => {
  if (state) emitNet('gtaxskate:rules:change', state.revision, patch, state.generation);
});
exports('GetRules', () => state ? JSON.parse(JSON.stringify(state)) : null);
