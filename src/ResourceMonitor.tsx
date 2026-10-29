import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Game } from './WindowsGames';
interface Sample { runId:string; capturedAt:number; cpuPercent:number|null; memoryBytes:number; peakMemoryBytes:number; processCount:number; partial:boolean }
interface Report { status:string; sample:Sample|null; message:string }
const memory=(bytes:number)=>bytes>=1024**3?`${(bytes/1024**3).toFixed(2)} GiB`:`${(bytes/1024**2).toFixed(1)} MiB`;
export function ResourceMonitor({game,enabled,onToggle}:{game:Game;enabled:boolean;onToggle:(enabled:boolean)=>void}) {
  const [visible,setVisible]=useState(!document.hidden);
  const [report,setReport]=useState<Report|null>(null);
  const [error,setError]=useState('');
  const [points,setPoints]=useState<Sample[]>([]);
  const runId=game.logDirectory?.split('/').pop()??'';
  const running=game.state.endsWith('running');
  useEffect(()=>{const change=()=>setVisible(!document.hidden);document.addEventListener('visibilitychange',change);return()=>document.removeEventListener('visibilitychange',change);},[]);
  useEffect(()=>{
    setReport(null);setError('');setPoints([]);
    if(!enabled||!visible||!running||!runId)return;
    let stopped=false;let timer:ReturnType<typeof setTimeout>;let staleTimer:ReturnType<typeof setTimeout>;
    const poll=async()=>{
      try{
        const result=await invoke<Report>('sample_windows_resources',{id:game.id,runId});
        if(stopped)return;
        clearTimeout(staleTimer);setReport(result);setError('');
        if(result.status==='live')staleTimer=setTimeout(()=>{if(!stopped)setReport({status:'stale',sample:null,message:'Waiting for a fresh resource reading…'});},4000);
        if(result.status==='live'&&result.sample){const sample=result.sample;setPoints(old=>old[old.length-1]?.capturedAt===sample.capturedAt?old:[...old,sample].slice(-60));}
        if(result.status==='inactive')return;
      }catch(error){if(stopped)return;setError(String(error));setReport(null);}
      if(!stopped)timer=setTimeout(()=>void poll(),2000);
    };
    void poll();
    return()=>{stopped=true;clearTimeout(timer);clearTimeout(staleTimer);void invoke('stop_windows_resource_monitor',{id:game.id,runId}).catch(()=>{});};
  },[enabled,visible,running,runId,game.id]);
  const sample=enabled&&visible&&running&&report?.status==='live'?report.sample:null;
  const max=Math.max(1,...points.map(p=>p.memoryBytes));
  const line=points.map((p,i)=>`${points.length<2?0:i/(points.length-1)*600},${76-p.memoryBytes/max*70}`).join(' ');
  return <div className="pm-resource-monitor">
    <button className="pm-subtle-button" aria-pressed={enabled} disabled={!enabled&&(!running||!runId)} title={!running?'Available while a game or installer is running':undefined} onClick={()=>onToggle(!enabled)}>{enabled?'Stop monitoring':'Monitor resources'}</button>
    {enabled&&<section aria-label={`Resources for ${game.name}`}>
      <p role="status">{!running?'Run ended. Monitoring stopped.':!visible?'Monitoring paused while this window is hidden.':error?`Resource readings unavailable: ${error}`:sample?'Monitoring this runtime and its observed child processes.':report?.message||'Waiting for the first sample…'}</p>
      <dl className="pm-resource-metrics">
        <div><dt>CPU</dt><dd>{sample?.cpuPercent!=null?`${sample.cpuPercent.toFixed(1)}%`:'—'}</dd></div>
        <div><dt>RAM (RSS)</dt><dd>{sample?memory(sample.memoryBytes):'—'}</dd></div>
        <div><dt>Peak RAM</dt><dd>{sample?memory(sample.peakMemoryBytes):'—'}</dd></div>
        <div><dt>Processes</dt><dd>{sample?.processCount??'—'}</dd></div>
      </dl>
      {sample&&points.length>1&&<figure className="pm-memory-trend"><svg viewBox="0 0 600 80" role="img" aria-label="Recent RAM usage, zero-based scale"><polyline points={line}/></svg><figcaption>RAM trend · last {points.length} samples · scale 0–{memory(max)}</figcaption></figure>}
      {sample?.partial&&<p>Some process readings are unavailable. These totals are incomplete.</p>}
      <p className="pm-resource-help">Updates about every 2 seconds. CPU 100% equals one core. RAM is an estimate; shared pages can be counted more than once. A rise alone does not confirm a memory leak.</p>
    </section>}
  </div>;
}
