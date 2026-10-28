import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { Download, Play, RefreshCw } from 'lucide-react';
import type { Finding } from './types';
interface LogReport { findings:Finding[]; logs:{name:string;status:string;truncated:boolean}[] }
interface Runtime { id:string; name:string; kind:string }
interface Game { id:string; name:string; runtimeId:string; gameExe:string|null; state:string; exitCode:number|null; message:string; directory:string; windowsDrive:string; logDirectory:string|null }
interface Library { runtimes:Runtime[]; games:Game[]; warnings:string[] }
const empty:Library={runtimes:[],games:[],warnings:[]};
export function WindowsGames(){
  const [library,setLibrary]=useState<Library>(empty);
  const [name,setName]=useState(''); const [installer,setInstaller]=useState(''); const [runtime,setRuntime]=useState('');
  const [approved,setApproved]=useState(false);const [busy,setBusy]=useState(false);const [error,setError]=useState('');
  const [findings,setFindings]=useState<Record<string,LogReport>>({});
  const refresh=useCallback(async()=>{
    const next=await invoke<Library>('list_windows_games');setLibrary(next);
    setRuntime(current=>next.runtimes.some(r=>r.id===current)?current:next.runtimes[0]?.id??'');
  },[]);
  useEffect(()=>{
    let stopped=false;let timer:ReturnType<typeof setTimeout>;
    const poll=async()=>{try{const next=await invoke<Library>('list_windows_games');if(!stopped){setLibrary(next);setRuntime(current=>next.runtimes.some(r=>r.id===current)?current:next.runtimes[0]?.id??'');}}catch(e){if(!stopped)setError(String(e));}finally{if(!stopped)timer=setTimeout(()=>void poll(),3000);}};
    void poll();return()=>{stopped=true;clearTimeout(timer);};
  },[]);
  const run=async(task:()=>Promise<void>)=>{setBusy(true);setError('');try{await task();}catch(e){setError(String(e));}finally{setBusy(false);}};
  const pickInstaller=()=>run(async()=>{
    const file=await open({multiple:false,directory:false,filters:[{name:'Windows installer',extensions:['exe']}]});
    if(typeof file!=='string')return;
    setInstaller(file);setName(file.split('/').pop()?.replace(/\.exe$/i,'')??'Windows game');setApproved(false);
  });
  const install=()=>run(async()=>{
    if(!approved || !installer || !name.trim() || !runtime)return;
    await invoke<Game>('install_windows_game',{name,installer,runtimeId:runtime,confirmed:true});
    setApproved(false);await refresh();
  });
  const chooseGame=(game:Game)=>run(async()=>{
    const file=await open({multiple:false,directory:false,defaultPath:game.windowsDrive,filters:[{name:'Installed Windows game',extensions:['exe']}]});
    if(typeof file!=='string')return;
    await invoke('select_windows_game_executable',{id:game.id,path:file,confirmed:true});await refresh();
  });
  return <>
    <div className="pm-page-title"><div><h1>Windows Games</h1><p>Install a Windows game into its own environment, then choose its executable and play.</p></div><button disabled={busy} className="pm-button pm-button-secondary" onClick={()=>void run(refresh)}><RefreshCw size={17}/>Refresh</button></div>
    <section className="pm-section"><div className="pm-section-heading"><div><h2>Install a Windows game</h2><p>Choose an installer you obtained from a trusted source. Existing Steam games are not changed.</p></div><Download className="pm-green" size={23}/></div>
      <button disabled={busy} className="pm-button pm-button-secondary" onClick={()=>void pickInstaller()}>Choose installer (.exe)</button>
      {installer && <p>Installer: <code>{installer}</code></p>}
      <p><label>Game name <input aria-label="Game name" value={name} maxLength={120} disabled={busy} onChange={e=>{setName(e.target.value);setApproved(false);}}/></label></p>
      <p><label>Runtime <select aria-label="Runtime" value={runtime} disabled={busy} onChange={e=>{setRuntime(e.target.value);setApproved(false);}}>{library.runtimes.map(r=><option value={r.id} key={r.id}>{r.name} ({r.kind})</option>)}</select></label></p>
      {!library.runtimes.length && <p role="status">No usable Wine or native Steam Proton runtime was found. Install a runtime first, then refresh. Automatic runtime downloads and Flatpak-only Proton runtimes are not supported in this build.</p>}
      <label className="pm-launch-consent"><input type="checkbox" checked={approved} disabled={busy} onChange={e=>setApproved(e.target.checked)}/><span>I trust this installer and approve running it. A separate prefix is not a security sandbox; Windows programs can access files available to my user.</span></label>
      <button className="pm-button pm-button-primary" disabled={busy || !approved || !installer || !name.trim() || !runtime} onClick={()=>void install()}><Download size={17}/>{busy?'Working…':'Run installer'}</button>
      <p>Complete the installer’s own window using its default C: location. Keep Proton Medic open. When it finishes, select the installed game’s .exe below. Installer exit code 0 alone does not prove installation succeeded.</p>
    </section>
    {error && <p role="alert" className="pm-repair-danger">{error}</p>}
    {library.warnings.map((w,i)=><p key={i} role="alert">{w}</p>)}
    <section className="pm-section"><h2>Your managed Windows games</h2>
      {!library.games.length && <p>No Windows games have been added yet.</p>}
      {library.games.map(game=>{const running=game.state.endsWith('running')||game.state==='session-ended';return <div className="pm-auto-message" key={game.id}>
        <h3>{game.name}</h3><p role="status">{game.state}{game.exitCode!==null?` · exit ${game.exitCode}`:''}: {game.message}</p>
        <button disabled={busy||running} className="pm-button pm-button-secondary" onClick={()=>void chooseGame(game)}>Select installed game .exe</button>{' '}
        <button disabled={busy||running||!game.gameExe} className="pm-button pm-button-primary" onClick={()=>void run(async()=>{await invoke('launch_windows_game',{id:game.id,confirmed:true});await refresh();})}><Play size={17}/>Launch game</button>{' '}
        <button disabled={busy||!game.logDirectory} className="pm-button pm-button-secondary" onClick={()=>void run(async()=>{const f=await invoke<LogReport>('analyze_windows_game',{id:game.id});setFindings(old=>({...old,[game.id]:f}));})}>Read runtime logs</button>
        {findings[game.id] && <div>{findings[game.id].logs.map(log=><p key={log.name}>{log.name}: {log.status}{log.truncated ? " · partial log (tail retained)" : ""}</p>)}{findings[game.id].findings.length?findings[game.id].findings.map(f=><p key={f.code}><strong>{f.title}</strong> — {f.recommendation}</p>):<p>No recognized signatures in the captured logs. This does not confirm the game works.</p>}</div>}
        <details><summary>Files and diagnostic output</summary><p>Game executable: {game.gameExe??'Not selected'}</p><p>Managed directory: <code>{game.directory}</code></p><p>Windows C: <code>{game.windowsDrive}</code></p><p>Logs: <code>{game.logDirectory??'No run yet'}</code></p><p>Captured stdout/stderr retain the latest 2 MiB each. No game data is automatically deleted.</p></details>
      </div>;})}
    </section>
  </>;
}
