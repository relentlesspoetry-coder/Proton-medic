import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { AutoSelection, CompatibilityTool, Game } from './types';
export function GameProtonPicker({ game, onChanged, onManage }: { game: Game; onChanged: () => void; onManage: () => void }) {
  const [tool, setTool] = useState(''); const [approved, setApproved] = useState(false);
  const [busy, setBusy] = useState(false); const [message, setMessage] = useState(''); const [error, setError] = useState('');
  const [tools, setTools] = useState<CompatibilityTool[] | null>(null);
  useEffect(() => {
    let stopped = false;
    setTools(null); setTool(''); setApproved(false); setError('');
    void invoke<CompatibilityTool[]>('list_game_protons', { appId: game.appId, libraryPath: game.libraryPath })
      .then(next => { if (!stopped) setTools(next); })
      .catch(e => { if (!stopped) setError(String(e)); });
    return () => { stopped = true; };
  }, [game.appId, game.libraryPath]);
  return <section className="pm-section pm-game-proton"><div className="pm-section-heading"><div><h2>Choose Proton version</h2></div><button className="pm-button pm-button-secondary" onClick={onManage}>Manage Proton versions</button></div>
    <label>Installed Proton<select value={tool} disabled={busy || !tools?.length} onChange={e => { setTool(e.target.value); setApproved(false); setMessage(''); }}><option value="">{tools ? tools.length ? 'Choose a version' : 'No compatible versions installed' : error ? 'Versions unavailable' : 'Loading versions…'}</option>{tools?.map(t => <option key={t.path} value={t.path}>{t.name} · {t.source}{tools.filter(other => other.name === t.name && other.source === t.source).length > 1 ? ` · ${t.path}` : ''}</option>)}</select></label>
    <p className="pm-report-help">Current: {game.configuredTool ?? 'Steam default'}. Close Steam before saving.</p>
    <label className="pm-report-approval"><input type="checkbox" checked={approved} disabled={busy} onChange={e => setApproved(e.target.checked)}/>I approve this game’s Proton change and configuration backup.</label>
    <button className="pm-button pm-button-primary" disabled={busy || !approved || !tool} onClick={() => {
      if (busy) return; setBusy(true); setError(''); setMessage('');
      void invoke<AutoSelection>('select_game_proton', { appId: game.appId, libraryPath: game.libraryPath, toolPath: tool, confirmed: true })
        .then(result => { setMessage(result.message); setApproved(false); onChanged(); }).catch(e => setError(String(e))).finally(() => setBusy(false));
    }}>{busy ? 'Saving…' : 'Use selected Proton'}</button>
    {error && <p className="pm-repair-danger" role="alert">{error}</p>}{message && <p className="pm-auto-message" role="status">{message}</p>}
  </section>;
}
