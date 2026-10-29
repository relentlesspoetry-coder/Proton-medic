const modules=process.env.PM_UI_MODULES || process.cwd()+'/node_modules';
const {chromium}=require(modules+'/playwright');
const {default:AxeBuilder}=require(modules+'/@axe-core/playwright');
const fs=require('fs');
const path=require('path');
const http=require('http');
const root=path.resolve('dist');
const output=process.env.PM_UI_OUTPUT || '/tmp/proton-medic-ui';
fs.mkdirSync(output,{recursive:true});
const server=http.createServer((request,response)=>{
 const file=path.resolve(root,'.'+new URL(request.url,'http://localhost').pathname);
 if(!file.startsWith(root+path.sep)&&file!==root){response.writeHead(403).end();return;}
 const target=file===root||file.endsWith(path.sep)?path.join(root,'index.html'):file;
 const types={'.html':'text/html','.js':'text/javascript','.css':'text/css','.png':'image/png','.webp':'image/webp'};
 try{response.setHeader('Content-Type',types[path.extname(target)]||'application/octet-stream');response.end(fs.readFileSync(target));}catch{response.writeHead(404).end();}
});
let browser;
(async()=>{
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
 const address='http://127.0.0.1:'+server.address().port;
 browser=await chromium.launch({headless:true,args:['--no-sandbox']});
 const context=await browser.newContext();
 const page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
 const fixture=()=>{
  const names=['Black Myth: Wukong','Hollow Knight','No Man’s Sky','Baldur’s Gate 3','Stardew Valley','ELDEN RING'];
  const games=names.map((name,i)=>({name,appId:String(100+i),libraryPath:'/home/player/Games/SteamLibrary',installPath:'/home/player/Games/'+name,installPathExists:i!==2,manifestPath:'/home/player/Games/steamapps/appmanifest_'+i+'.acf',prefixPath:'/home/player/Games/compatdata/'+i,prefixExists:true,configuredTool:'GE-Proton10-34',buildId:'123456',lastPlayedAt:1700000000000+i*1000}));
  const findings=[{code:'wine-exception',severity:'warning',title:'A Windows exception was recorded',description:'The runtime log contains an exception. Review the game and runtime settings before trying a repair.',recommendation:'Try a different installed Proton version after backing up your configuration.',evidence:['Unhandled exception in game process']}];
  const scan={games,installations:[{kind:'Native',path:'/home/player/.steam/steam'}],libraries:[{path:'/home/player/Games/SteamLibrary',steamAppsPath:'/home/player/Games/SteamLibrary/steamapps',installKind:'Native'}],compatibilityTools:[{name:'GE-Proton10-34',path:'/home/player/.steam/compatibilitytools.d/GE-Proton10-34',source:'User-installed'},{name:'Proton Experimental',path:'/Steam/Proton Experimental',source:'Steam'}],warnings:[]};
  const reports=games.map((g,i)=>({appId:g.appId,libraryPath:g.libraryPath,logPath:i===0?'/home/player/steam-100.log':null,logSize:32156,modifiedUnixSeconds:1700000000,truncated:false,status:i===0?'review':'no-log',findings:i===0?findings:[],notes:[]}));
  const windows={runtimes:[{id:'proton:fixture',name:'GE-Proton10-34',kind:'proton'}],games:[{id:'123-456',name:'Black Myth: Wukong',runtimeId:'proton:fixture',gameExe:'/home/player/Games/Black Myth Wukong/b1.exe',state:'game-exited',exitCode:0,message:'Game process ended.',directory:'/home/player/.local/share/proton-medic/windows-games/123-456',windowsDrive:'/home/player/Games/Black Myth Wukong/pfx/drive_c',prefixDirectory:'/home/player/Games/Black Myth Wukong/pfx',logDirectory:'/home/player/.local/share/proton-medic/windows-games/123-456/logs/123'}],removedGames:[],warnings:[]};
  windows.games[0].lastLaunchedAt=1700000060000;
  windows.games.push({...windows.games[0],id:'789-123',name:'Game Setup',gameExe:null,state:'installer-exited'});
  window.__PM_FIXTURE_WINDOWS=windows;
  window.__TAURI_INTERNALS__={invoke:async(cmd,args)=>{
   if(cmd==='prepare_support_report')return {token:1,content:JSON.stringify({appVersion:'0.6.19',pathsRedacted:args.options.redactPaths,system:{os:'Zorin OS',architecture:'x86_64'},steam:{games:[{name:'Black Myth: Wukong',installPath:'[redacted path]',status:'review'}]},windows:{games:[{name:'Black Myth: Wukong',runtime:'GE-Proton10-34',exitCode:7}]}},null,2)+'\n'};
   if(cmd==='save_support_report')return '/home/player/Documents/proton-medic-report.json';
   if(cmd==='list_proton_versions')return {installations:scan.installations,tools:scan.compatibilityTools,status:window.__PM_PROTON_STATUS??null,umuAvailable:false};
   if(cmd==='fetch_proton_releases')return [{tag:'GE-Proton11-11',name:'GE-Proton11-11-x86_64.tar.gz',size:564789552},{tag:'GE-Proton11-10',name:'GE-Proton11-10-x86_64.tar.gz',size:566144788}];
   if(cmd==='install_proton_version'){window.__PM_PROTON_STATUS={phase:'downloading',tag:args.tag,downloaded:200000000,total:564789552,message:'Downloading from the official GE-Proton release.'};return;}
   if(cmd==='cancel_proton_download'){window.__PM_PROTON_STATUS={phase:'canceled',tag:'GE-Proton11-11',downloaded:0,total:0,message:'Download canceled. Installed versions are unchanged.'};return;}
   if(cmd==='scan_steam')return scan;
   if(cmd==='analyze_diagnostics')return {games:reports,system:{os:'Zorin OS',kernel:'Linux 6.14',arch:'x86_64',gpuDevices:['NVIDIA GeForce RTX 3060'],vulkaninfoPath:'/usr/bin/vulkaninfo',icdManifests:['nvidia_icd.json'],notes:[]},warnings:[]};
   if(cmd==='recommend_proton')return {games:games.map(g=>({appId:g.appId,libraryPath:g.libraryPath,gameName:g.name,status:'try-version',suggestedTool:'Proton Experimental',configuredTool:'GE-Proton10-34',reason:'A different installed version may help with the recorded exception.',nextStep:'Back up your selection, then test the game.',confidence:'tentative',alternatives:['GE-Proton10-34'],evidence:['wine-exception']})),note:''};
   if(cmd==='list_windows_games')return windows;
   if(cmd==='get_windows_launch_history')return {entries:[{id:'1-1700000000',kind:'game',runtime:'GE-Proton10-34',startedAt:1700000000000,endedAt:1700000065000,state:'process-failed',exitCode:7,message:'Game process ended.',summaries:['A Windows exception was recorded'],partialLogs:true},{id:'1-1600000000',kind:'installer',runtime:'Wine',startedAt:1600000000000,endedAt:1600000065000,state:'installer-exited',exitCode:0,message:'Setup process ended.',summaries:[],partialLogs:false}],warnings:[]};
   if(cmd==='monitor_game_logs')return [];
   if(cmd==='sample_windows_resources')return {status:'live',sample:{runId:args.runId,capturedAt:Date.now(),cpuPercent:145.2,memoryBytes:1536*1024*1024,peakMemoryBytes:1700*1024*1024,processCount:6,partial:false},message:''};
   if(cmd==='stop_windows_resource_monitor')return;
   if(cmd.includes('game_icon')||cmd==='get_windows_executable_icon')return null;
   if(cmd==='review_windows_installation')return {candidates:[{path:'/home/player/Games/A New Game/Binaries/Win64/game.exe',suggestedName:'A New Game'}],truncated:false};
   if(cmd==='get_game_logging')return null;
   if(cmd==='list_repair_backups')return [];
   if(cmd==='plugin:updater|check')return null;
   if(cmd==='startup_diagnostic')return;
   throw new Error('Unexpected fixture command: '+cmd);
  }};
 };
 await page.addInitScript(fixture);
 const results=[];
 for(const [width,height] of [[1440,900],[1200,800],[850,620],[1280,800],[390,844]]){
  await page.setViewportSize({width,height});await page.goto(address);await page.getByText('6',{exact:true}).waitFor();
  for(const name of ['Home','My Games','Windows Games','Proton versions','Fixes','System Check','Settings']){
   await page.getByRole('navigation',{name:'Main navigation'}).getByRole('button',{name,exact:true}).click();await page.waitForTimeout(100);
   const overflow=await page.evaluate(()=>({page:document.documentElement.scrollWidth>innerWidth+1,elements:[...document.querySelectorAll('main button,main input,main select,main summary')].filter(el=>{const r=el.getBoundingClientRect();return r.width&&r.height&&(r.right>innerWidth+1||r.left<0)}).map(el=>el.textContent.slice(0,90))}));
   if(overflow.page||overflow.elements.length)throw Error(JSON.stringify({width,name,overflow}));
   if(width===1200||width===850)await page.screenshot({path:path.join(output,`${width}-${name.replaceAll(' ','-')}.jpg`),fullPage:true,type:'jpeg',quality:80});
   if(width===1200){const a=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze();results.push({width,name,violations:a.violations.map(v=>({id:v.id,impact:v.impact,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))}))});}
   else results.push({width,name,layout:'passed'});
   if(width===1200&&name==='Home'){
    await page.getByRole('button',{name:/^Scan your library/}).hover();
    const hover=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze();
    results.push({width,name:'Home primary action hover',violations:hover.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))}))});
   }
  }
 }
 await page.getByRole('navigation',{name:'Main navigation'}).getByRole('button',{name:'Proton versions',exact:true}).click();
 await page.getByRole('button',{name:'Load latest releases',exact:true}).click();
 await page.getByLabel('I approve downloading and installing this GE-Proton release.',{exact:true}).check();
 await page.getByRole('button',{name:'Download and install',exact:true}).click();
 await page.getByRole('progressbar',{name:'Proton download progress'}).waitFor();
 for(const [width,height] of [[1280,800],[390,844]]){
  await page.setViewportSize({width,height});
  if(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1))throw Error('Proton download overflow: '+width);
  const check=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze();
  results.push({width,name:'Proton manager download',violations:check.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))}))});
  if(width===1280)await page.screenshot({path:path.join(output,'proton-manager.jpg'),fullPage:true,type:'jpeg',quality:80});
 }
 await page.getByRole('button',{name:'Cancel download',exact:true}).click();
 await page.getByRole('navigation',{name:'Main navigation'}).getByRole('button',{name:'Settings',exact:true}).click();
 await page.getByRole('button',{name:'Create report',exact:true}).click();
 await page.getByLabel('Recent Windows launch history',{exact:true}).check();
 await page.getByRole('button',{name:'Generate preview',exact:true}).click();
 await page.getByLabel('Exact file contents',{exact:true}).waitFor();
 if(await page.getByRole('button',{name:'Save report',exact:true}).isEnabled())throw Error('Report save enabled before review');
 for(const [width,height] of [[1280,800],[850,620],[390,844]]){
  await page.setViewportSize({width,height});
  if(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1))throw Error('Report preview overflow: '+width);
  const check=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze();
  results.push({width,name:'Troubleshooting report preview',violations:check.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))}))});
  if(width===1280)await page.screenshot({path:path.join(output,'support-report.jpg'),fullPage:true,type:'jpeg',quality:80});
 }
 await page.getByLabel('I reviewed this report and approve saving it.',{exact:true}).check();
 await page.getByRole('button',{name:'Save report',exact:true}).click();
 await page.getByText('Report saved to /home/player/Documents/proton-medic-report.json. Nothing was uploaded.',{exact:true}).waitFor();
 await page.getByRole('navigation',{name:'Main navigation'}).getByRole('button',{name:'My Games',exact:true}).click();
 await page.getByRole('button',{name:'Open Black Myth: Wukong (Windows)',exact:true}).waitFor();
 await page.getByRole('button',{name:'Add Black Myth: Wukong (Windows) to favorites',exact:true}).click();
 await page.getByRole('combobox',{name:'Filter games'}).selectOption('favorites');
 await page.getByRole('combobox',{name:'Sort games'}).selectOption('recent');
 for(const [width,height] of [[1280,800],[390,844]]){
  await page.setViewportSize({width,height});
  if(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1))throw Error('Unified library overflow: '+width);
  const check=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze();
  results.push({width,name:'Unified favorites and recent sorting',violations:check.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))}))});
 }
 await page.getByRole('button',{name:'Open Black Myth: Wukong (Windows)',exact:true}).click();
 await page.getByRole('button',{name:'Launch game',exact:true}).waitFor();
 if(await page.getByRole('region',{name:'Add Windows game'}).count())throw Error('Focused Windows detail contains the installation form');
 await page.getByRole('button',{name:'Back to My Games',exact:true}).click();
 await page.getByRole('combobox',{name:'Filter games'}).selectOption('all');
 await page.getByRole('combobox',{name:'Sort games'}).selectOption('name');
 await page.setViewportSize({width:1280,height:800});
 await page.screenshot({path:path.join(output,'unified-library.jpg'),fullPage:true,type:'jpeg',quality:80});
 await page.setViewportSize({width:1200,height:800});await page.getByRole('navigation',{name:'Main navigation'}).getByRole('button',{name:'My Games',exact:true}).click();await page.getByRole('button',{name:'Open Black Myth: Wukong (Steam)',exact:true}).click();await page.screenshot({path:path.join(output,'game-detail.jpg'),fullPage:true,type:'jpeg',quality:80});
 for(const name of ['Diagnostics','Proton','Launch & repair','Backups & details']) {
  await page.getByRole('tab',{name,exact:true}).click();
  if(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1))throw Error('Game panel overflow: '+name);
  const detail=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze();
  results.push({name:'Game detail: '+name,violations:detail.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))}))});
  await page.screenshot({path:path.join(output,'detail-'+name.replaceAll(/[^a-zA-Z]+/g,'-')+'.jpg'),fullPage:true,type:'jpeg',quality:80});
 }
 await page.getByRole('navigation',{name:'Main navigation'}).getByRole('button',{name:'Windows Games',exact:true}).click();
 await page.getByRole('combobox',{name:'Installed executable for Game Setup'}).selectOption('/home/player/Games/A New Game/Binaries/Win64/game.exe');
 await page.getByLabel('I trust the selected executable and approve saving this game.').check();
 await page.getByText('Game settings and logs',{exact:true}).first().click();
 await page.getByRole('button',{name:'Edit launch settings',exact:true}).click();
 await page.getByRole('textbox',{name:'Launch arguments for Black Myth: Wukong'}).fill('-dx12\ntwo words\n$HOME');
 for(const [width,height] of [[1440,900],[1200,800],[850,620],[1280,800],[390,844]]){
  await page.setViewportSize({width,height});
  if(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1))throw Error('Windows settings or installation assistant overflow: '+width);
  const check=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze();
  results.push({width,name:'Installation review and launch settings',violations:check.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))}))});
  if(width===1200)await page.screenshot({path:path.join(output,'windows-options.jpg'),fullPage:true,type:'jpeg',quality:80});
 }
 await page.getByText('Launch history',{exact:true}).first().click();
 await page.getByText('Game · Runtime exit 7',{exact:true}).waitFor();
 for(const [width,height] of [[1280,800],[390,844]]){
  await page.setViewportSize({width,height});
  if(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1))throw Error('Launch history overflow: '+width);
  const check=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze();
  results.push({width,name:'Launch history',violations:check.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))}))});
  if(width===1280)await page.screenshot({path:path.join(output,'launch-history.jpg'),fullPage:true,type:'jpeg',quality:80});
 }
 await page.evaluate(()=>{window.__PM_FIXTURE_WINDOWS.games[0].state='game-running';});
 await page.getByRole('button',{name:'Refresh',exact:true}).click();
 await page.getByRole('button',{name:'Monitor resources',exact:true}).first().click();
 await page.getByText('145.2%',{exact:true}).waitFor();
 await page.getByRole('img',{name:'Recent RAM usage, zero-based scale'}).waitFor();
 for(const [width,height] of [[1280,800],[390,844]]){
  await page.setViewportSize({width,height});
  if(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1))throw Error('Resource monitor overflow: '+width);
  const check=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze();
  results.push({width,name:'Resource monitor',violations:check.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))}))});
  if(width===1280)await page.screenshot({path:path.join(output,'resource-monitor.jpg'),fullPage:true,type:'jpeg',quality:80});
 }
 await page.getByRole('button',{name:'Stop monitoring',exact:true}).click();
 const touchContext=await browser.newContext({hasTouch:true,viewport:{width:1280,height:800}});
 const touch=await touchContext.newPage();touch.on('pageerror',e=>errors.push(e.message));
 await touch.addInitScript(fixture);await touch.goto(address);await touch.getByText('6',{exact:true}).waitFor();
 for(const name of ['Home','My Games','Windows Games','Settings']){
  await touch.getByRole('navigation',{name:'Main navigation'}).getByRole('button',{name,exact:true}).tap();
  const small=await touch.evaluate(()=>[...document.querySelectorAll('button,summary')].filter(el=>{const r=el.getBoundingClientRect();return r.width>0&&r.height>0&&(r.width<43.5||r.height<43.5)}).map(el=>el.textContent));
  if(small.length)throw Error('Small Deck touch targets on '+name+': '+JSON.stringify({small,media:await touch.evaluate(()=>({coarse:matchMedia('(any-pointer: coarse)').matches,size:getComputedStyle(document.documentElement).getPropertyValue('--pm-control-size')}))}));
  if(await touch.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1))throw Error('Deck touch layout overflow: '+name);
  results.push({name:'Deck touch: '+name,layout:'passed'});
  if(name==='My Games'){
   await touch.getByRole('textbox',{name:'Search games'}).focus();
   const outline=await touch.locator('.pm-search').evaluate(el=>getComputedStyle(el).outlineStyle);
   if(outline==='none')throw Error('Library search has no visible focus outline');
  }
  if(name==='Home')await touch.screenshot({path:path.join(output,'deck-touch-home.jpg'),fullPage:true,type:'jpeg',quality:80});
 }
 await touchContext.close();
 fs.writeFileSync(path.join(output,'results.json'),JSON.stringify({results,errors},null,2));console.log(JSON.stringify({layouts:results.length,errors,violations:results.filter(r=>r.violations?.length)},null,2));
 for(const file of ['1200-Home.jpg','1200-My-Games.jpg','1200-Windows-Games.jpg','1200-Settings.jpg','850-Home.jpg','game-detail.jpg','detail-Proton.jpg','detail-Launch-repair.jpg','detail-Backups-details.jpg','windows-options.jpg','deck-touch-home.jpg','launch-history.jpg','resource-monitor.jpg','unified-library.jpg','support-report.jpg','proton-manager.jpg']) {
  console.log('UI_RENDER '+file+' '+fs.readFileSync(path.join(output,file)).toString('base64'));
 }
 await browser.close();server.close();
 if(errors.length||results.some(r=>r.violations?.length))process.exitCode=1;
})().catch(async e=>{
 console.error(e);
 for(const file of fs.readdirSync(output).filter(file=>file.endsWith('.jpg')))console.log('UI_RENDER '+file+' '+fs.readFileSync(path.join(output,file)).toString('base64'));
 if(browser)await browser.close();server.close();process.exitCode=1;
});
