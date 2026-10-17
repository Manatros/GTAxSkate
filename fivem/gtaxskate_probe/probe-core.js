'use strict';
// One tiny module: exported answer() returns i32 42. No imports, data or IO.
globalThis.SkateProbe = (() => {
  const bytes = [0,97,115,109,1,0,0,0,1,5,1,96,0,1,127,3,2,1,0,7,10,1,6,97,110,115,119,101,114,0,0,10,6,1,4,0,65,42,11];
  const statuses = new Set(['ok', 'unsupported', 'error', 'timeout']);
  const tokenOK = value => typeof value === 'string' && /^[a-f0-9]{32}$/.test(value);
  const safeText = value => typeof value === 'string' && value.length <= 120 && /^[\w .:+/()\-]*$/.test(value);
  async function run(api) {
    if (!api || typeof api.instantiate !== 'function') return 'unsupported';
    try {
      const result = await api.instantiate(new Uint8Array(bytes));
      return result && result.instance && typeof result.instance.exports.answer === 'function'
        && result.instance.exports.answer() === 42 ? 'ok' : 'error';
    } catch (_) { return 'error'; }
  }
  function metadata(values) {
    const out = {};
    for (const key of ['resource', 'version', 'build', 'runtime']) {
      const value = String(values[key] ?? 'unknown');
      out[key] = safeText(value) ? value : 'unknown';
    }
    return out;
  }
  function validReport(report) {
    if (!report || typeof report !== 'object' || Array.isArray(report)) return false;
    if (Object.keys(report).sort().join(',') !== 'js,metadata,nui,token') return false;
    if (!tokenOK(report.token) || !statuses.has(report.js) || !statuses.has(report.nui)) return false;
    const meta = report.metadata;
    return meta && typeof meta === 'object' && !Array.isArray(meta)
      && Object.keys(meta).sort().join(',') === 'build,resource,runtime,version'
      && Object.values(meta).every(safeText);
  }
  return { run, metadata, validReport, tokenOK, statuses };
})();
