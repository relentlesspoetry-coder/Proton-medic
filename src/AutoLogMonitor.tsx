import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { AutoSelection } from './types';
interface Update {
  appId: string; libraryPath: string; gameName: string; state: string; message: string; codes: string[];
  session: { sessionId: string; selection: AutoSelection } | null;
}
export function AutoLogMonitor({ enabled, onToggle }: { enabled: boolean; onToggle: (enabled: boolean) => void }) {
  const [updates, setUpdates] = useState<Update[]>([]);
  const [error, setError] = useState('');
  const delivered = useRef(new Set<string>());
  useEffect(() => {
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const next = await invoke<Update[]>('monitor_game_logs', { enabled });
        if (stopped) return;
        setUpdates(next); setError('');
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
  return <section className="pm-section" aria-label="Automatic game log reader">
    <div className="pm-section-heading"><div><h2>Automatic Game Log Reader</h2><p>Checks Steam Proton logs every 10 seconds while Proton Medic is open. No manual log upload needed.</p></div></div>
    <label className="pm-auto-toggle"><input type="checkbox" checked={enabled} onChange={e => onToggle(e.target.checked)}/><span><strong>Automatically apply supported fixes</strong><small>Changes are backed up and wait for Steam to close. One automatic attempt per game per app session. Games are not opened automatically.</small></span></label>
    <p>Existing logs are read on startup; automatic fixes respond to new log evidence. Choose Diagnose this game or approve a launch test to enable its Proton logging automatically. Close Steam when prompted; no manual launch-option entry is needed for supported configurations.</p>
    {error && <p role="alert" className="pm-repair-danger">{error}</p>}
    {!updates.length && <p role="status">Watching for new game logs. No findings to display.</p>}
    {updates.map(item => <div key={`${item.libraryPath}:${item.appId}`} className="pm-auto-message" role="status"><strong>{item.gameName} · {item.state}</strong><p>{item.message}</p>
      {item.session && <p>Backup: <code>{item.session.selection.backupPath || 'No change needed'}</code></p>}
      <details><summary>Log findings</summary><p>{item.codes.join(', ') || 'No recognized error signatures'}</p></details>
    </div>)}
  </section>;
}
