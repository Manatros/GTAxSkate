'use strict';
const probe = globalThis.SkateProbe;
const crypto = require('crypto');
const pending = new Map(), cooldown = new Map();
const prefix = '[GTAxSkate probe]';
function playerSource() {
  const id = Number(global.source);
  return Number.isInteger(id) && id > 0 && id <= 65535 && GetPlayerName(String(id)) ? id : null;
}
function metadata() {
  const name = GetCurrentResourceName();
  return probe.metadata({resource: name, version: GetResourceMetadata(name, 'version', 0),
    build: GetConvar('sv_enforceGameBuild', 'unknown'), runtime: GetConvar('version', 'unknown')});
}
async function serverResult() {
  const status = await probe.run(globalThis.WebAssembly);
  console.log(`${prefix} server ${JSON.stringify({status, metadata: metadata()})}; answer42 only, not Skate-engine validation.`);
}
onNet('gtaxskate:probe:request', () => {
  const id = playerSource(), now = Date.now();
  if (!id || now < (cooldown.get(id) || 0)) return;
  cooldown.set(id, now + 10000);
  const token = crypto.randomBytes(16).toString('hex');
  pending.set(id, {token, until: now + 10000});
  emitNet('gtaxskate:probe:start', id, token);
  void serverResult();
  setTimeout(() => {
    if (pending.get(id)?.token !== token) return;
    pending.delete(id);
    console.log(`${prefix} player ${id}: report timed out.`);
  }, 10000);
});
onNet('gtaxskate:probe:report', report => {
  const id = playerSource(), request = pending.get(id);
  if (!id || !request || Date.now() > request.until || !probe.validReport(report) || report.token !== request.token) return;
  pending.delete(id);
  // Client observations are diagnostic claims, never permissions or trusted engine attestations.
  console.log(`${prefix} player ${id} reported ${JSON.stringify(report)}; client claims, answer42 only.`);
});
on('playerDropped', () => { const id = Number(global.source); pending.delete(id); cooldown.delete(id); });
RegisterCommand('skateprobe', source => {
  if (Number(source) === 0) void serverResult();
}, false);
