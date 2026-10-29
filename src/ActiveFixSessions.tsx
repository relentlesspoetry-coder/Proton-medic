import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
interface Session { sessionId: string; appId: string; libraryPath: string; selection: { toolName: string } | null }
export function ActiveFixSessions() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let stopped = false;
    const refresh = () => { void invoke<Session[]>('list_fix_sessions').then(rows => {
      if (!stopped && Array.isArray(rows)) setSessions(rows);
    }).catch(e => { if (!stopped) setError(String(e)); }); };
    refresh();
    window.addEventListener('pm-fix-session', refresh);
    const timer = setInterval(refresh, 5000);
    return () => { stopped = true; clearInterval(timer); window.removeEventListener('pm-fix-session', refresh); };
  }, []);
  const finish = async (id: string, rollback: boolean) => {
    setBusy(true); setError('');
    try {
      await invoke(rollback ? 'rollback_game_fix' : 'finish_game_fix', { sessionId: id, confirmed: true });
      setSessions(rows => rows.filter(s => s.sessionId !== id));
      window.dispatchEvent(new Event('pm-fix-session'));
    } catch (e) { setError(String(e)); } finally { setBusy(false); }
  };
  if (!sessions.length && !error) return null;
  return <section className="pm-section" aria-label="Unfinished repairs">
    {!!sessions.length && <h2>Finish repairs</h2>}
    {sessions.map(s => <div key={s.sessionId} className="pm-callout">
      <div><strong>{s.selection?.toolName ?? 'Proton selection'} · Game {s.appId}</strong><p>{s.libraryPath}</p>
      <button className="pm-button pm-button-secondary" disabled={busy} onClick={() => void finish(s.sessionId, false)}>Keep selection</button>{' '}
      <button className="pm-button pm-button-secondary" disabled={busy} onClick={() => void finish(s.sessionId, true)}>Roll back</button></div>
    </div>)}
    {error && <p role="alert">{error}</p>}
  </section>;
}
