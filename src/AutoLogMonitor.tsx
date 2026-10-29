import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { AutoSelection } from './types';
interface Update {
  appId: string; libraryPath: string; gameName: string; state: string; message: string; codes: string[];
  session: { sessionId: string; selection: AutoSelection } | null;
}
export function AutoLogMonitor({ enabled, onToggle, expanded = true, controlPending = false }: { enabled: boolean; onToggle: (enabled: boolean) => void; expanded?: boolean; controlPending?: boolean }) {
  const [updates, setUpdates] = useState<Update[]>([]);
  const [error, setError] = useState('');
  const [cancelling, setCancelling] = useState(false);
  const cancellation = useRef(0);
  const cancelLocked = useRef(false);
  const delivered = useRef(new Set<string>());
  useEffect(() => {
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const revision = cancellation.current;
        const next = await invoke<Update[]>('monitor_game_logs');
        if (stopped || revision !== cancellation.current) return;
        setUpdates(old => JSON.stringify(old) === JSON.stringify(next) ? old : next); setError('');
        for (const item of next) {
          if (!item.session || delivered.current.has(item.session.sessionId)) continue;
          try {
            const key = `pm-fix:${item.libraryPath}:${item.appId}`;
            // Preserve launch-attempt state already saved by Fix & Test.
            const existing = JSON.parse(sessionStorage.getItem(key) ?? 'null');
            if (existing?.sessionId !== item.session.sessionId) sessionStorage.setItem(key, JSON.stringify(item.session));
            delivered.current.add(item.session.sessionId);
            window.dispatchEvent(new Event('pm-fix-session'));
          } catch { setError('A fix was backed up, but its local receipt could not be saved. Keep the app open and record the backup path shown below.'); }
        }
      } catch (e) { if (!stopped) setError(String(e)); }
      finally { if (!stopped) timer = setTimeout(() => void poll(), 10_000); }
    };
    void poll();
    return () => { stopped = true; clearTimeout(timer); };
  }, [enabled]);
  const cancelLogging = async (item: Update) => {
    if (cancelLocked.current) return;
    cancelLocked.current = true; setCancelling(true); setError(''); ++cancellation.current;
    try {
      await invoke('cancel_game_logging', {appId:item.appId,libraryPath:item.libraryPath,confirmed:true});
      setUpdates(old => old.filter(value => !(value.state==='logging-queued' && value.appId===item.appId && value.libraryPath===item.libraryPath)));
    } catch (error) { setError(String(error)); }
    finally { ++cancellation.current; cancelLocked.current=false; setCancelling(false); }
  };
  const results = updates.map(item => <div key={`${item.libraryPath}:${item.appId}:${item.state}`} className="pm-auto-message" role="status"><strong>{item.gameName} · {item.state==='logging-queued'?'Logging queued':item.state}</strong><p>{item.message}</p>
      {item.state==='logging-queued' && <button className="pm-button pm-button-secondary" disabled={cancelling} onClick={()=>void cancelLogging(item)}>Cancel queued log setup</button>}
      {item.session && <p>Backup: <code>{item.session.selection.backupPath || 'No change needed'}</code></p>}
      {item.state!=='logging-queued' && <details><summary>Log findings</summary><p>{item.codes.join(', ') || 'No recognized error signatures'}</p></details>}
    </div>);
  if (!expanded && !error && !updates.length) return null;
  return <section className="pm-section" aria-label="Automatic game log reader">
    <div className="pm-section-heading"><div><h2>{expanded ? 'Log monitoring' : 'Game log updates'}</h2>{expanded && <p>Checks new Proton logs while Proton Medic is open.</p>}</div></div>
    {expanded && <><label className="pm-auto-toggle"><input type="checkbox" disabled={controlPending} checked={enabled} onChange={e => { if (!controlPending) onToggle(e.target.checked); }}/><span><strong>Automatically apply supported fixes</strong><small>Backs up each change and waits for Steam to close. Games are never launched automatically.</small></span></label>
    <details className="pm-update-details"><summary>How log monitoring works</summary><p>Reads logs every 10 seconds. Automatic fixes use new findings and make at most one attempt per game per app session. Use Diagnose this game or a launch test to enable logging.</p></details></>}
    {error && <p role="alert" className="pm-repair-danger">{error}</p>}
    {expanded && !updates.length && <p role="status">Watching for new game logs. No findings to display.</p>}
    {expanded ? results : <details className="pm-update-details"><summary>{updates.length} game log update{updates.length === 1 ? '' : 's'} — review results</summary>{results}</details>}
  </section>;
}
