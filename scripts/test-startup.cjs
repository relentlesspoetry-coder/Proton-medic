const {test}=require('node:test');const assert=require('node:assert/strict');const vm=require('node:vm');const fs=require('node:fs');
const code=fs.readFileSync('public/startup.js','utf8');
function fixture(mounted=false){const logs=[],events={},banner={hidden:false,textContent:''};let timer;
const window={__TAURI_INTERNALS__:{invoke:async(_,args)=>{logs.push(args.message)}},addEventListener:(name,fn)=>events[name]=fn};
vm.runInNewContext(code,{window,console:{error:()=>{}},document:{getElementById:()=>banner,querySelector:()=>mounted?{}:null,addEventListener:(name,fn)=>events[name]=fn},setTimeout:fn=>timer=fn});return {logs,events,banner,runTimer:()=>timer()};}
test('records startup and reports a missing mounted application',()=>{const f=fixture();f.events.DOMContentLoaded();f.runTimer();assert.deepEqual(f.logs,['HTML startup script executed','DOM loaded','App not mounted after 15 seconds']);});
test('reports DOM presence without claiming successful rendering',()=>{const f=fixture(true);f.runTimer();assert.match(f.logs.at(-1),/Startup complete/);});
test('captures module resource failures and reveals an error banner',()=>{const f=fixture();f.events.error({target:{src:'/assets/app.js'}});assert.match(f.banner.textContent,/Resource failed/);assert.equal(f.banner.hidden,false);});
test('bounds rejected-promise diagnostics',()=>{const f=fixture();f.events.unhandledrejection({reason:'x'.repeat(9000)});assert.equal(f.logs.at(-1).length,4000);});
