'use strict';
const probe = globalThis.SkateProbe;
let requestedUntil = 0, active = null;
const prefix = '[GTAxSkate probe]';
function metadata() {
  const name = GetCurrentResourceName();
  return probe.metadata({resource: name, version: GetResourceMetadata(name, 'version', 0),
    build: typeof GetGameBuildNumber === 'function' ? GetGameBuildNumber() : 'unknown',
    runtime: 'FiveM client JavaScript'});
}
RegisterNuiCallbackType('skateprobeResult');
on('__cfx_nui:skateprobeResult', (data, cb) => {
  cb({ok: true});
  if (!active || !data || typeof data !== 'object' || Array.isArray(data)
    || Object.keys(data).sort().join(',') !== 'status,token'
    || data.token !== active.token || !probe.statuses.has(data.status)) return;
  active.finish(data.status);
});
RegisterCommand('skateprobe', () => {
  if (active || Date.now() < requestedUntil) return;
  requestedUntil = Date.now() + 10000;
  emitNet('gtaxskate:probe:request');
  console.log(`${prefix} requested; testing answer42 only, not Skate-engine support.`);
}, false);
onNet('gtaxskate:probe:start', async token => {
  if (Number(global.source) !== 65535 || active || Date.now() >= requestedUntil || !probe.tokenOK(token)) return;
  // Reserve the token before any async work; duplicate server messages cannot start extra probes.
  let finish;
  const nui = new Promise(resolve => { finish = resolve; });
  active = {token, finish};
  const timer = setTimeout(() => finish('timeout'), 5000);
  SendNuiMessage(JSON.stringify({type: 'gtaxskate:probe', token}));
  const js = await Promise.race([probe.run(globalThis.WebAssembly), new Promise(resolve => setTimeout(() => resolve('timeout'), 5000))]);
  const report = {token, js, nui: await nui, metadata: metadata()};
  clearTimeout(timer);
  active = null;
  console.log(`${prefix} ${JSON.stringify(report)}; answer42 only. Client results are untrusted diagnostics.`);
  emitNet('gtaxskate:probe:report', report);
});
