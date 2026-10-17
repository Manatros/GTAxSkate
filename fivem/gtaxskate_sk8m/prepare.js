'use strict';
// Deterministic source overlay, fresh staging destination only. Nothing is
// installed into a running/previous server; no downloads or asset conversion.
const fs = require('node:fs');
const path = require('node:path');
const cp = require('node:child_process');
const crypto = require('node:crypto');
const PIN = '50b923097691d7c73d6e8eb719cffc5cab40d138';
const BASE_FILES = ['config.lua', 'flick.lua', 'tuning.lua', 'surface.lua', 's3.lua',
  'dev.lua', 'deck.lua', 'client.lua', 'shop.lua', 'server.lua', 'deck_server.lua',
  'html/sound.html', 'html/sound.js', 'html/designer.html', 'html/designer.js', 'html/designer.css'];
const RULE_FILES = ['fxmanifest.lua', 'server.js', 'client.js', 'policy.json'];
const sha = content => crypto.createHash('sha256').update(content).digest('hex');
function replaceOnce(text, from, to, label) {
  if (text.split(from).length !== 2) throw Error(`Pinned overlay anchor mismatch: ${label}`);
  return text.replace(from, to);
}
function overlayClient(original) {
  let text = original.replace(/\r\n/g, '\n');
  const start = text.indexOf('local reply\nRegisterNetEvent(\'skateboard:boardReply\'');
  const end = text.indexOf('\n-- ------------------------------------------------------------------ trick lines', start);
  if (start < 0 || end < 0) throw Error('Pinned ownsBoard overlay anchor mismatch');
  text = text.slice(0, start) +
    '-- Preliminary eligibility only; every mount is server-authorized below.\n' +
    'local function ownsBoard() return Friends.canRide() end\n' + text.slice(end);
  text = replaceOnce(text, 'local function mount(from)\n    local ped = PlayerPedId()',
    'local function mount(from)\n    if not Friends.authorize() then return end\n    local ped = PlayerPedId()', 'mount authorization');
  text = replaceOnce(text, '    riding = true\n    sk.rider = ped',
    '    if not Friends.granted() then Friends.stop() return end\n    riding = true\n    sk.rider = ped', 'post-load authorization');
  text = replaceOnce(text,
    "            notify('~y~No skateboard model found.~s~ Add one to Config.BoardModels. Riding without a board.')",
    "            notify('~r~Board model unavailable.~s~ Riding was not started. Ask the server owner to check the board resource.')\n            return",
    'missing board must abort mounting');
  text = replaceOnce(text,
    '            board = CreateObject(model, here.x, here.y, here.z - 1.0, Config.NetworkBoard, true, false)\n            SetModelAsNoLongerNeeded(model)',
    "            board = CreateObject(model, here.x, here.y, here.z - 1.0, Config.NetworkBoard, true, false)\n            SetModelAsNoLongerNeeded(model)\n            if not board or board == 0 then\n                board = nil\n                notify('~r~Board creation failed.~s~ Riding was not started. Check the server entity policy.')\n                return\n            end",
    'failed entity creation must abort mounting');
  for (const name of ['skatecam', 'skatestance', 'skatemode', 'skatevolume', 'skatesound', 'skatehelmet']) {
    const marker = new RegExp(`RegisterCommand\\('${name}', function\\([^\\n]*\\)\\n`, 'g');
    const matches = [...text.matchAll(marker)];
    if (matches.length !== 1) throw Error(`Pinned command overlay mismatch: ${name}`);
    text = text.replace(marker, match => match +
      "    do notify('Unavailable in this narrow friends-test adapter. Use skaterules / skateset for supported settings.') return end\n");
  }
  return text + '\n-- GTAxSkate pinned lifecycle seam: use SK8M cleanup, never duplicate riding physics.\n' +
    'Friends.stop = function()\n' +
    '    if riding then getOff() end\n' +
    '    if off then putAway(false) end\n' +
    '    releaseCam()\n    deleteBoard()\n' +
    '    local ped = PlayerPedId()\n    FreezeEntityPosition(ped, false)\n' +
    'end\n';
}
function overlayServer(original) {
  let text = original.replace(/\r\n/g, '\n');
  const start = text.indexOf("RegisterNetEvent('skateboard:requestBoard', function()");
  const end = text.indexOf('\nlocal function ace', start);
  if (start < 0 || end < 0) throw Error('Pinned server authorization overlay anchor mismatch');
  text = text.slice(0, start) + `RegisterNetEvent('skateboard:requestBoard', function(token, generation, revision)
    local src = source
    if type(token) ~= 'string' or #token < 1 or #token > 100 or not token:match('^[%d:]+$') then return end
    local now, c = GetGameTimer(), cache[src]
    if c and now - c.asked < 100 then return end
    local ok, state = pcall(function()
        if GetResourceState('gtaxskate_sk8m') ~= 'started' then return nil end
        return exports.gtaxskate_sk8m:GetSnapshot(src)
    end)
    local allowed = ok and type(state) == 'table' and state.version == 1 and
        state.generation == generation and state.revision == revision
    -- Preserve the existing inventory check. Do not cache a grant across revisions.
    if allowed then
        if not c or now - c.time >= 750 or c.generation ~= generation or c.revision ~= revision then
            local checked, owned = pcall(ownsBoard, src)
            c = { time = now, asked = now, result = checked and owned == true,
                generation = generation, revision = revision }
        end
        allowed = c.result
    end
    if not c then c = { time = now, result = false } end
    c.asked = now
    cache[src] = c
    TriggerClientEvent('skateboard:boardReply', src, allowed == true, token,
        ok and state and state.generation or nil, ok and state and state.revision or nil)
end)
` + text.slice(end);
  return text;
}
function prepare(root, output) {
  root = path.resolve(root);
  const allowed = path.join(root, 'build', 'fivem-friends');
  output = path.resolve(output);
  const relative = path.relative(allowed, output);
  if (!relative || relative.startsWith('..') || path.isAbsolute(relative) || relative.includes(path.sep)) {
    throw Error('Destination must be one fresh direct child of build/fivem-friends');
  }
  if (fs.existsSync(output)) throw Error('Destination exists; preparation never overwrites previous packages');
  for (let check = path.dirname(output); check !== path.dirname(check); check = path.dirname(check)) {
    if (fs.existsSync(check) && fs.lstatSync(check).isSymbolicLink()) throw Error('Linked staging ancestors are unsupported');
  }
  const repo = path.join(root, 'external', 'sk8m');
  const git = args => cp.execFileSync('git', ['-C', repo, ...args], { encoding: 'utf8', windowsHide: true });
  if (git(['rev-parse', 'HEAD']).trim() !== PIN) throw Error(`Expected SK8M ${PIN}; review another revision before staging`);
  const sources = {}, overlays = {}, outputs = new Map();
  const read = file => {
    const bytes = fs.readFileSync(path.join(root, file)); overlays[file] = sha(bytes); return bytes;
  };
  for (const name of BASE_FILES) {
    const file = `fivem/skateboard/${name}`;
    const original = git(['show', `${PIN}:${file}`]);
    sources[file] = sha(original);
    const value = name === 'client.lua' ? overlayClient(original) : name === 'server.lua' ? overlayServer(original) : original;
    outputs.set(`resources/skateboard/${name}`, value);
  }
  for (const name of ['LICENSE', 'NOTICE']) {
    const text = git(['show', `${PIN}:${name}`]); sources[name] = sha(text);
    outputs.set(`resources/skateboard/${name}`, text);
  }
  for (const name of ['bridge.lua', 'fallback-config.lua']) {
    outputs.set(`resources/skateboard/${name}`, read(`fivem/gtaxskate_sk8m/${name}`));
  }
  const boardFiles = ['fxmanifest.lua', 'README.md', 'LICENSE', 'provenance.json', 'build-checks.json',
    'stream/gtaxskate_test_board.ydr', 'stream/gtaxskate_test_board.ytd', 'stream/gtaxskate_board.ytyp'];
  const provenance = JSON.parse(read('fivem/gtaxskate_board/provenance.json').toString('utf8').replace(/^\uFEFF/, ''));
  if (provenance.generatorSha256 !== sha(read('fivem/gtaxskate_board/generate.py'))) {
    throw Error('Original board generator changed; rebuild and review its assets before packaging');
  }
  const checks = JSON.parse(read('fivem/gtaxskate_board/build-checks.json').toString('utf8').replace(/^\uFEFF/, ''));
  if (!Array.isArray(checks) || checks.length !== 3) throw Error('Original board build checks missing');
  for (const file of boardFiles) {
    const bytes = read(`fivem/gtaxskate_board/${file}`);
    if (file.startsWith('stream/')) {
      const check = checks.find(item => item.File === path.basename(file));
      if (!check || check.BinaryReadback !== true || check.Sha256 !== sha(bytes) || check.Bytes !== bytes.length) {
        throw Error(`Original board artifact differs from reviewed build: ${file}`);
      }
    }
    outputs.set(`resources/gtaxskate_board/${file}`, bytes);
  }
  outputs.set('resources/skateboard/fxmanifest.lua', `fx_version 'cerulean'
game 'gta5'
version '0.4.0-gtaxskate-friends.1'
dependency 'gtaxskate_rules'
dependency 'gtaxskate_sk8m'
dependency 'gtaxskate_board'
shared_scripts { 'config.lua', 'fallback-config.lua' }
client_scripts { 'flick.lua', 'tuning.lua', 'surface.lua', 's3.lua', 'dev.lua', 'deck.lua', 'bridge.lua', 'client.lua', 'shop.lua' }
server_scripts { 'server.lua', 'deck_server.lua' }
ui_page 'html/sound.html'
files { 'html/sound.html', 'html/sound.js', 'html/designer.html', 'html/designer.js', 'html/designer.css' }
`);
  for (const name of RULE_FILES) {
    let value = read(`fivem/gtaxskate_rules/${name}`);
    if (name === 'server.js') value = replaceOnce(value.toString(), 'skating adapter NOT included',
      'narrow SK8M fallback catalog; real multiplayer test pending', 'rules server diagnostic');
    if (name === 'client.js') value = replaceOnce(value.toString(), 'Policy prototype; skating adapter not included.',
      'SK8M Lua fallback: four settings, client-owned physics; multiplayer test pending.', 'rules client diagnostic');
    outputs.set(`resources/gtaxskate_rules/${name}`, value);
  }
  outputs.set('policy-presets/developer.json', read('fivem/gtaxskate_sk8m/policy-developer.json'));
  outputs.set('resources/gtaxskate_rules/policy-core.js', read('fivem/gtaxskate_rules/policy.js'));
  outputs.set('resources/gtaxskate_rules/policy.js', read('fivem/gtaxskate_sk8m/rules-policy.js'));
  for (const name of ['server.js', 'fxmanifest.lua']) outputs.set(`resources/gtaxskate_sk8m/${name}`, read(`fivem/gtaxskate_sk8m/${name}`));
  outputs.set('server-local.example.cfg', `# REVIEW EXAMPLE ONLY: not a complete runnable server configuration.
# Use your normal txAdmin-created config and private license key; never send the key in chat.
# Replace existing endpoint lines for an isolated same-PC test (do not append duplicate bindings).
endpoint_add_tcp "127.0.0.1:30120"
endpoint_add_udp "127.0.0.1:30120"
sv_lan 1
sv_scriptHookAllowed 0
ensure gtaxskate_rules
ensure gtaxskate_sk8m
ensure gtaxskate_board
ensure skateboard
# No account login, license, firewall/router change or server start is performed by preparation.
`);
  const guide = fs.existsSync(path.join(root, 'docs/FIVEM-FRIENDS-TEST.md')) ? 'docs/FIVEM-FRIENDS-TEST.md' : 'FIVEM-FRIENDS-TEST.md';
  outputs.set('FRIENDS-TEST.md', read(guide));
  for (const name of ['LICENSE', 'NOTICE']) {
    const file = fs.existsSync(path.join(root, 'public', name)) ? `public/${name}` : name;
    outputs.set(`${name}.txt`, read(file));
  }
  const hashes = Object.fromEntries([...outputs].map(([name, value]) => [name, sha(value)]));
  const manifest = { format: 1, target: 'SK8M Lua fallback friends test', sourceCommit: PIN,
    authoritativePhysics: false, supportedSettings: ['PushSpeed', 'PushPower', 'Braking', 'Steering'],
    sourceHashes: sources, overlayHashes: overlays, outputHashes: hashes };
  // Collect and verify every source before creating the fresh staging folder.
  fs.mkdirSync(output, { recursive: true });
  for (const [name, value] of outputs) {
    const destination = path.join(output, name);
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    fs.writeFileSync(destination, value, { flag: 'wx' });
  }
  fs.writeFileSync(path.join(output, 'package-manifest.json'), JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' });
  return manifest;
}
module.exports = { PIN, BASE_FILES, overlayClient, overlayServer, prepare };
if (require.main === module) {
  const root = path.resolve(__dirname, '../..');
  const output = process.argv[2] || path.join(root, 'build', 'fivem-friends', crypto.randomUUID());
  prepare(root, output);
  console.log(`Staged experimental fallback: ${output}\nNo server files changed and no server started.`);
}
