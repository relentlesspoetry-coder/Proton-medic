import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Game } from './WindowsGames';
interface Entry { id:string; kind:string; runtime:string; startedAt:number; endedAt:number|null; state:string; exitCode:number|null; message:string; summaries:string[]; partialLogs:boolean }
interface History { entries:Entry[]; warnings:string[] }
function result(entry:Entry) {
  if(entry.state==='running')return 'Running';
  if(entry.state==='start-failed')return 'Could not start';
  if(entry.state==='interrupted')return 'Interrupted by restart';
  if(entry.state==='unknown')return 'Exit unknown';
  if(entry.exitCode!==null)return `Runtime exit ${entry.exitCode}`;
  return 'No exit code recorded';
}
function duration(entry:Entry) {
  if(entry.endedAt===null||entry.endedAt<entry.startedAt)return null;
  const seconds=Math.floor((entry.endedAt-entry.startedAt)/1000);
  return seconds<60?`${seconds}s`:`${Math.floor(seconds/60)}m ${seconds%60}s`;
}
export function LaunchHistory({game}:{game:Game}) {
  const [expanded,setExpanded]=useState(false);
  const [history,setHistory]=useState<History|null>(null);
  const [error,setError]=useState('');
  const [loading,setLoading]=useState(false);
  const [revision,setRevision]=useState(0);
  useEffect(()=>{
    if(!expanded)return;
    let stopped=false;setLoading(true);setError('');
    void invoke<History>('get_windows_launch_history',{id:game.id}).then(value=>{
      if(!Array.isArray(value?.entries)||!Array.isArray(value?.warnings))throw new Error('Invalid history response');
      if(!stopped)setHistory(value);
    }).catch(error=>{if(!stopped)setError(String(error));}).finally(()=>{if(!stopped)setLoading(false);});
    return ()=>{stopped=true;};
  },[expanded,game.id,game.state,game.logDirectory,revision]);
  return <details className="pm-update-details pm-launch-history" onToggle={event=>setExpanded(event.currentTarget.open)}>
    <summary>Launch history</summary>
    {expanded&&<section aria-label={`Launch history for ${game.name}`} aria-busy={loading}>
      <div className="pm-history-heading"><p>Latest 20 runs from Proton Medic and its shortcuts. Runtime exit codes do not confirm gameplay.</p><button className="pm-button pm-button-secondary" disabled={loading} onClick={()=>setRevision(value=>value+1)}>Refresh history</button></div>
      {loading&&!history&&<p role="status">Loading launch history…</p>}
      {error&&<p role="alert">Could not load launch history: {error}. Use Refresh history to retry.</p>}
      {history?.warnings.map(warning=><p role="alert" key={warning}>{warning}</p>)}
      {history&&!history.entries.length&&<p>No recorded launches yet. History starts with runs made in v0.6.16; earlier logs remain available.</p>}
      {!!history?.entries.length&&<ol className="pm-history-list">{history.entries.map(entry=><li key={entry.id}>
        <div className="pm-history-heading"><strong>{entry.kind==='installer'?'Installer':'Game'} · {result(entry)}</strong><time dateTime={new Date(entry.startedAt).toISOString()}>{new Date(entry.startedAt).toLocaleString()}</time></div>
        <p className="pm-history-runtime">{entry.runtime}{duration(entry)!==null?` · ${duration(entry)}`:''}</p>
        <p>{entry.message}</p>
        {entry.summaries.length>0?<ul>{entry.summaries.map(summary=><li key={summary}>{summary}</li>)}</ul>:entry.endedAt!==null&&entry.state!=='start-failed'?<p>No recognized errors in the captured logs.</p>:null}
        {entry.partialLogs&&<p>Logs are incomplete or contain only the retained tail.</p>}
      </li>)}</ol>}
    </section>}
  </details>;
}
