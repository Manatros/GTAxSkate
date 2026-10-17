'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const read = file => fs.readFileSync(path.join(__dirname, file), 'utf8');
function core(extra = {}) {
  const context = vm.createContext({Uint8Array, console, ...extra});
  vm.runInContext(read('probe-core.js'), context);
  return context;
}
test('actual minimal WASM module exports 42; absent runtime is unsupported', async () => {
  const {SkateProbe: probe} = core();
  assert.equal(await probe.run(WebAssembly), 'ok');
  assert.equal(await probe.run(undefined), 'unsupported');
  assert.equal(await probe.run({}), 'unsupported');
});
test('async instantiation rejection and wrong result are errors', async () => {
  const {SkateProbe: probe} = core();
  assert.equal(await probe.run({instantiate: async () => { throw Error('disabled'); }}), 'error');
  assert.equal(await probe.run({instantiate: async () => ({instance: {exports: {answer: () => 41}}})}), 'error');
});
test('report schema rejects extra fields, oversized metadata and invalid statuses', () => {
  const probe = core().SkateProbe;
  const report = {token: 'a'.repeat(32), js: 'ok', nui: 'unsupported', metadata: probe.metadata({resource:'gtaxskate_probe', version:'0.1.0', build:3258, runtime:'FiveM'})};
  assert.equal(probe.validReport(report), true);
  assert.equal(probe.validReport({...report, secret: 'x'}), false);
  assert.equal(probe.validReport({...report, js: 'engine-ready'}), false);
  assert.equal(probe.validReport({...report, token: 'a'.repeat(100000)}), false);
  assert.equal(probe.validReport({...report, metadata:{...report.metadata, runtime:'x'.repeat(121)}}), false);
  assert.equal(probe.validReport({...report, metadata:{...report.metadata, extra:'x'}}), false);
});
test('server correlates player token, rejects forgery/replay/unbounded data, limits requests', async () => {
  const events = {}, sent = [], logs = [], timers = [];
  let source = 1;
  const context = core({require, Date, setTimeout: cb => timers.push(cb),
    console:{log: line => logs.push(line)}, onNet:(name, cb) => events[name]=cb,
    on:(name, cb) => events[name]=cb, RegisterCommand:()=>{},
    GetPlayerName: id => ['1','2'].includes(id) ? 'Player' : null,
    GetCurrentResourceName:()=> 'gtaxskate_probe', GetResourceMetadata:()=> '0.1.0',
    GetConvar:(_, fallback)=>fallback, emitNet:(...args)=>sent.push(args), WebAssembly});
  Object.defineProperty(context, 'source', {get:()=>source});
  context.global=context;
  vm.runInContext(read('server.js'), context);
  const request = events['gtaxskate:probe:request'], report = events['gtaxskate:probe:report'];
  request(); request();
  assert.equal(sent.length, 1);
  const token=sent[0][2], meta=context.SkateProbe.metadata({});
  const result={token,js:'ok',nui:'error',metadata:meta};
  source=2; report(result);
  source=1; report({...result,token:'f'.repeat(32)});
  report({...result,metadata:{...meta,build:'a'.repeat(100000)}});
  assert.equal(logs.some(line=>line.includes('player 1 reported')), false);
  report(result); report(result);
  assert.equal(logs.filter(line=>line.includes('player 1 reported')).length, 1);
  source=-1; request(); source=NaN; request();
  assert.equal(sent.length, 1);
  source=2;request();timers.at(-1)();report({...result,token:sent[1][2]});
  assert.equal(logs.filter(line=>line.includes('player 2 reported')).length, 0);
  await new Promise(resolve=>setImmediate(resolve));
  assert.equal(logs.filter(line=>line.includes('server ')).length, 2);
});
test('client rejects unsolicited/forged starts and NUI responses; async errors report safely', async () => {
  const events={}, commands={}, emitted=[], messages=[], timers=[];
  const context=core({Date, Promise, setTimeout:cb=>{timers.push(cb);return timers.length;},clearTimeout:()=>{},
    WebAssembly:{instantiate:async()=>{throw Error('disabled');}}, console:{log:()=>{}},
    RegisterNuiCallbackType:()=>{}, on:(name,cb)=>events[name]=cb,onNet:(name,cb)=>events[name]=cb,
    RegisterCommand:(name,cb)=>commands[name]=cb,emitNet:(...args)=>emitted.push(args),
    SendNuiMessage:data=>messages.push(JSON.parse(data)),GetCurrentResourceName:()=> 'gtaxskate_probe',
    GetResourceMetadata:()=> '0.1.0',GetGameBuildNumber:()=>3258});
  context.global=context;context.source=65535;
  vm.runInContext(read('client.js'), context);
  const start=events['gtaxskate:probe:start'], token='b'.repeat(32);
  await start(token); assert.equal(messages.length,0);
  commands.skateprobe();context.source=1;await start(token);assert.equal(messages.length,0);
  context.source=65535;const done=start(token);assert.equal(messages.length,1);
  events['__cfx_nui:skateprobeResult']({token:'c'.repeat(32),status:'ok'},()=>{});
  events['__cfx_nui:skateprobeResult']({token,status:'ok',extra:'ignored'},()=>{});
  events['__cfx_nui:skateprobeResult']({token,status:'unsupported'},()=>{});
  await done;
  const report=emitted.find(args=>args[0]==='gtaxskate:probe:report')[1];
  assert.equal(report.js,'error');assert.equal(report.nui,'unsupported');
  assert.equal(context.SkateProbe.validReport(report),true);
});
test('NUI uses own callback only and absent WASM returns unsupported', async () => {
  let listener;const fetched=[];
  const context=core({WebAssembly:undefined,window:{addEventListener:(_,cb)=>listener=cb},
    GetParentResourceName:()=> 'gtaxskate_probe',fetch:async(...args)=>fetched.push(args)});
  vm.runInContext(read('html/probe.js'),context);
  await listener({data:{type:'gtaxskate:probe',token:'bad'}});assert.equal(fetched.length,0);
  await listener({data:{type:'gtaxskate:probe',token:'d'.repeat(32)}});
  assert.equal(fetched[0][0],'https://gtaxskate_probe/skateprobeResult');
  assert.equal(JSON.parse(fetched[0][1].body).status,'unsupported');
});
