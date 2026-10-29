import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
interface Session { sessionId: string; appId: string; libraryPath: string; selection: { toolName: string } | null }
export function ActiveFixSessions() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const sessionRequest = useRef(0);
  const sessionMutation = useRef(false);
  useEffect(() => {
    let stopped = false;
    const refresh = () => {
      const request = ++sessionRequest.current;
      if (sessionMutation.current) return;
      void invoke<Session[]>('list_fix_sessions').then(rows => {
        if (!stopped && request === sessionRequest.current && !sessionMutation.current && Array.isArray(rows)) setSessions(rows);
      }).catch(e => { if (!stopped && request === sessionRequest.current && !sessionMutation.current) setError(String(e)); });
    };
    refresh();
    window.addEventListener('pm-fix-session', refresh);
    const timer = setInterval(refresh, 5000);
    return () => { stopped = true; clearInterval(timer); window.removeEventListener('pm-fix-session', refresh); };
  }, []);
  const finish = async (id: string, rollback: boolean) => {
    if (sessionMutation.current) return;
    sessionMutation.current = true;
    ++sessionRequest.current;
    setBusy(true); setError('');
    try {
      await invoke(rollback ? 'rollback_game_fix' : 'finish_game_fix', { sessionId: id, confirmed: true });
      setSessions(rows => rows.filter(s => s.sessionId !== id));
    } catch (e) { setError(String(e)); } finally {
      sessionMutation.current = false;
      setBusy(false);
      window.dispatchEvent(new Event('pm-fix-session'));
    }
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
