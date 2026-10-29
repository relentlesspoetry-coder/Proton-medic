import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Wrench, Play, RotateCcw } from 'lucide-react';
import type { Game, AutoSelection, LaunchResult } from './types';
interface FixPlan { canApply: boolean; reason: string; tool: string | null; evidence: string[] }
interface LoggingStatus { state: string; message: string; backupPath: string | null; canRestore: boolean }
interface Session { sessionId: string; selection: AutoSelection; attempted?: boolean }

/** A single backed-up fix; never treat launcher/log success as gameplay proof. */
export function FixAndTest({ game }: { game: Game }) {
  const key = `pm-fix:${game.libraryPath}:${game.appId}`;
  const [session, setSession] = useState<Session | null>(() => {
    try { return JSON.parse(sessionStorage.getItem(key) ?? 'null'); } catch { return null; }
  });
  useEffect(() => {
    let stopped = false;
    const receive = () => {
      void invoke<(Session & { appId: string; libraryPath: string })[]>('list_fix_sessions').then(sessions => {
        if (!stopped && Array.isArray(sessions)) setSession(sessions.find(s => s.appId === game.appId && s.libraryPath === game.libraryPath) ?? null);
      }).catch(e => { if (!stopped) setError(String(e)); });
    };
    receive();
    window.addEventListener('pm-fix-session', receive);
    return () => { stopped = true; window.removeEventListener('pm-fix-session', receive); };
  }, [key]);
  const [plan, setPlan] = useState<FixPlan | null>(null);
  const [approved, setApproved] = useState(false);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<LaunchResult | null>(null);
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  const [logging, setLogging] = useState<LoggingStatus | null>(null);
  useEffect(() => {
    let stopped=false;
    void invoke<LoggingStatus | null>('get_game_logging', {appId:game.appId,libraryPath:game.libraryPath})
      .then(next=>{if(!stopped)setLogging(next);}).catch(e=>{if(!stopped)setError(String(e));});
    return()=>{stopped=true;};
  },[game.appId,game.libraryPath]);
  useEffect(() => {
    if (logging?.state !== 'waiting-for-steam') return;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    const check = async () => {
      try {
        const next = await invoke<LoggingStatus | null>('get_game_logging', { appId: game.appId, libraryPath: game.libraryPath });
        if (!stopped && next) setLogging(next);
      } catch (e) { if (!stopped) setError(String(e)); }
      if (!stopped) timer = setTimeout(() => void check(), 5000);
    };
    timer = setTimeout(() => void check(), 5000);
    return () => { stopped = true; clearTimeout(timer); };
  }, [logging?.state, game.appId, game.libraryPath]);
  const remember = (next: Session | null) => {
    setSession(next);
    try { if (next) sessionStorage.setItem(key, JSON.stringify(next)); else sessionStorage.removeItem(key); } catch { /* backend listing recovers the session */ }
    window.dispatchEvent(new Event('pm-fix-session'));
  };
  const run = async (task: () => Promise<void>) => {
    setBusy(true); setError('');
    try { await task(); } catch (e) { setError(String(e)); } finally { setBusy(false); }
  };
  const diagnose = () => run(async () => {
    setPlan(null); setApproved(false); setMessage('');
    try {
      setLogging(await invoke<LoggingStatus>('prepare_game_logging', { appId: game.appId, libraryPath: game.libraryPath, confirmed: true }));
    } catch (e) {
      setLogging({state:'needs-attention',message:`Automatic log setup unavailable: ${String(e)}. Existing Steam settings are preserved.`,backupPath:null,canRestore:false});
    }
    setPlan(await invoke<FixPlan>('plan_game_fix', { appId: game.appId, libraryPath: game.libraryPath }));
  });
  const launch = async (current: Session) => {
    remember({ ...current, attempted: true });
    setResult(await invoke<LaunchResult>('test_game_fix', { sessionId: current.sessionId, confirmed: true }));
  };
  const fix = () => run(async () => {
    if (!approved || !plan?.canApply || !plan.tool || session) return;
    const current = await invoke<Session>('start_game_fix', {
      appId: game.appId, libraryPath: game.libraryPath, expectedTool: plan.tool, confirmed: true,
    });
    remember(current); setMessage(current.selection.message); setResult(null);
    await launch(current);
  });
  const rollback = () => run(async () => {
    if (!session || !window.confirm('Close Steam first. Restore the configuration from before this fix? Newer changes to this game’s mapping will block automatic rollback.')) return;
    setMessage(await invoke<string>('rollback_game_fix', { sessionId: session.sessionId, confirmed: true }));
    remember(null); setPlan(null); setResult(null); setApproved(false);
  });
  const keep = () => run(async () => {
    if (!session) return;
    await invoke('finish_game_fix', { sessionId: session.sessionId, confirmed: true });
    remember(null); setPlan(null); setResult(null); setApproved(false);
    setMessage('Selection kept by you. The configuration backup remains on disk. This does not certify game compatibility.');
  });
  return <section className="pm-section" aria-label="Fix and test game">
    <div className="pm-section-heading"><div><h2>Fix &amp; Test</h2><p>Diagnose this game to prepare logging automatically, then apply a backed-up fix and test.</p></div><Wrench className="pm-green" size={23}/></div>
    {!session && <button className="pm-button pm-button-secondary" disabled={busy} onClick={() => void diagnose()}>{busy ? 'Working…' : 'Diagnose this game'}</button>}
    <p>Diagnosis and approved launch tests attempt to enable Proton logs automatically. Close Steam to allow setup; existing launch options are backed up and preserved. If setup is unavailable, you can still launch using existing Steam settings.</p>
    {logging && <div className="pm-auto-message" role="status"><p>{logging.message}</p>{logging.backupPath && <p>Launch-options backup: <code>{logging.backupPath}</code></p>}
      {logging.canRestore && <button disabled={busy} className="pm-button pm-button-secondary" onClick={() => void run(async () => {
        if (!window.confirm('Close Steam first. Restore this game’s launch options from before automatic log setup?')) return;
        setLogging(await invoke<LoggingStatus>('restore_game_logging', { appId: game.appId, libraryPath: game.libraryPath, confirmed: true }));
      })}>Restore previous log settings</button>}
    </div>}
    {plan && !session && <div className="pm-advisor-step"><div><strong>{plan.canApply ? `Try ${plan.tool}` : 'No supported automatic fix yet'}</strong><p>{plan.reason}</p>
      {plan.canApply && <><p>Close Steam completely first. Your configuration is backed up before changing Proton. Steam will reopen and the game may remain running after the 10-second observation.</p>
        <label className="pm-launch-consent"><input type="checkbox" checked={approved} disabled={busy} onChange={e => setApproved(e.target.checked)}/><span>I approve this Proton change and opening the game.</span></label>
        <button className="pm-button pm-button-primary" disabled={busy || !approved} onClick={() => void fix()}><Play size={17}/>{busy ? 'Fixing and testing…' : 'Fix & Test'}</button></>}
      <details><summary>Diagnostic evidence</summary><p>{plan.evidence.join(', ') || 'No matching signatures'}</p></details>
    </div></div>}
    {session && <div className="pm-auto-message"><p>Selected: {session.selection.toolName}</p><p>Backup: <code>{session.selection.backupPath || 'No change needed'}</code></p>
      <p>Keep this app open until you keep or roll back the fix. Automatic rollback requires Steam closed and an unchanged game mapping. Backups remain on disk after exit.</p>
      {!session.attempted && <button disabled={busy} className="pm-button pm-button-primary" onClick={() => void run(() => launch(session))}>Launch &amp; test now</button>}
      <button disabled={busy} className="pm-button pm-button-secondary" onClick={() => void rollback()}><RotateCcw size={17}/>Roll back fix</button>{' '}
      <button disabled={busy} className="pm-button pm-button-secondary" onClick={() => void keep()}>Keep this selection</button>
    </div>}
    {result && <div role="status" className="pm-auto-message"><strong>Launch observed — confirm playability yourself</strong><p>{result.detail}</p><p>{result.findingCodes.join(', ')}</p></div>}
    {message && <p role="status">{message}</p>}
    {error && <p role="alert" className="pm-repair-danger">{error}</p>}
  </section>;
}
