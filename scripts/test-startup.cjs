const {test}=require('node:test');const assert=require('node:assert/strict');const vm=require('node:vm');const fs=require('node:fs');
const code=fs.readFileSync('public/startup.js','utf8');
function fixture(mounted=false){const logs=[],events={},banner={hidden:false,textContent:''};let timer;
const window={__TAURI_INTERNALS__:{invoke:async(_,args)=>{logs.push(args.message)}},addEventListener:(name,fn)=>events[name]=fn};
vm.runInNewContext(code,{window,console:{error:()=>{}},document:{getElementById:()=>banner,querySelector:()=>mounted?{}:null,addEventListener:(name,fn)=>events[name]=fn},setTimeout:fn=>timer=fn});return {logs,events,banner,runTimer:()=>timer()};}
test('records startup and reports a missing mounted application',()=>{const f=fixture();f.events.DOMContentLoaded();f.runTimer();assert.deepEqual(f.logs,['HTML startup script executed','DOM loaded','App not mounted after 15 seconds']);});
test('reports DOM presence without claiming successful rendering',()=>{const f=fixture(true);f.runTimer();assert.match(f.logs.at(-1),/Startup complete/);});
test('captures module resource failures and reveals an error banner',()=>{const f=fixture();f.events.error({target:{src:'/assets/app.js'}});assert.match(f.banner.textContent,/Resource failed/);assert.equal(f.banner.hidden,false);});
test('bounds rejected-promise diagnostics',()=>{const f=fixture();f.events.unhandledrejection({reason:'x'.repeat(9000)});assert.equal(f.logs.at(-1).length,4000);});
test('image failures retain the loading banner without claiming startup failed',()=>{const f=fixture();f.banner.textContent='Loading interface';f.events.error({target:{tagName:'IMG',src:'/cover.png'}});assert.equal(f.banner.textContent,'Loading interface');assert.match(f.logs.at(-1),/Resource failed: \/cover.png/);});
test('mounted image failures keep the startup banner hidden and allow image fallback',()=>{
 const {JSDOM}=require('jsdom');const dom=new JSDOM('<div id="startup-status" hidden>Loading</div><div id="root"><main class="pm-app"><img id="cover" src="https://example.invalid/cover.jpg"></main></div>',{runScripts:'outside-only'});
 try{const {window}=dom;const logs=[];window.console.error=message=>logs.push(message);window.setTimeout=()=>0;window.eval(code);
 let fallback=false;const image=window.document.getElementById('cover');image.addEventListener('error',()=>{fallback=true;image.remove();});image.dispatchEvent(new window.Event('error'));
 assert.equal(fallback,true);assert.equal(window.document.getElementById('startup-status').hidden,true);assert.ok(window.document.querySelector('#root .pm-app'));assert.match(logs.at(-1),/Resource failed/);
 }finally{dom.window.close();}
});
test('stylesheet and JavaScript errors still reveal the startup banner',()=>{const f=fixture();f.banner.hidden=true;f.events.error({target:{tagName:'LINK',href:'/assets/app.css'}});assert.equal(f.banner.hidden,false);assert.match(f.banner.textContent,/app.css/);f.banner.hidden=true;f.events.error({message:'Startup crashed'});assert.equal(f.banner.hidden,false);assert.match(f.banner.textContent,/Startup crashed/);});
