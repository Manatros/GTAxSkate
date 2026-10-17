'use strict';
let busy = false;
window.addEventListener('message', async event => {
  const data = event.data, probe = globalThis.SkateProbe;
  if (busy || !data || typeof data !== 'object' || Array.isArray(data)
    || Object.keys(data).sort().join(',') !== 'token,type'
    || data.type !== 'gtaxskate:probe' || !probe.tokenOK(data.token)) return;
  busy = true;
  try {
    const status = await probe.run(globalThis.WebAssembly);
    await fetch(`https://${GetParentResourceName()}/skateprobeResult`, {
      method: 'POST', headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({token: data.token, status})
    });
  } catch (_) { /* Client timeout records failed callback delivery; never change browser flags. */ }
  finally { busy = false; }
});
