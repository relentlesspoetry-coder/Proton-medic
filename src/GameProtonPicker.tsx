import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { AutoSelection, CompatibilityTool, Game } from './types';
export function GameProtonPicker({ game, tools, onChanged, onManage }: { game: Game; tools: CompatibilityTool[]; onChanged: () => void; onManage: () => void }) {
  const [tool, setTool] = useState(''); const [approved, setApproved] = useState(false);
  const [busy, setBusy] = useState(false); const [message, setMessage] = useState(''); const [error, setError] = useState('');
  return <section className="pm-section pm-game-proton"><div className="pm-section-heading"><div><h2>Choose Proton version</h2><p>Select an installed version for {game.name}.</p></div><button className="pm-button pm-button-secondary" onClick={onManage}>Manage Proton versions</button></div>
    <label>Installed Proton<select value={tool} disabled={busy} onChange={e => { setTool(e.target.value); setApproved(false); setMessage(''); }}><option value="">Choose a version</option>{tools.map(t => <option key={t.path} value={t.path}>{t.name} · {t.source}</option>)}</select></label>
    <p className="pm-report-help">Current Steam setting: {game.configuredTool ?? 'Steam default'}. Close Steam first. Only versions installed for this game’s Steam installation can be selected.</p>
    <label className="pm-report-approval"><input type="checkbox" checked={approved} disabled={busy} onChange={e => setApproved(e.target.checked)}/>I approve this game’s Proton change and configuration backup.</label>
    <button className="pm-button pm-button-primary" disabled={busy || !approved || !tool} onClick={() => {
      if (busy) return; setBusy(true); setError(''); setMessage('');
      void invoke<AutoSelection>('select_game_proton', { appId: game.appId, libraryPath: game.libraryPath, toolPath: tool, confirmed: true })
        .then(result => { setMessage(result.message); setApproved(false); onChanged(); }).catch(e => setError(String(e))).finally(() => setBusy(false));
    }}>{busy ? 'Saving…' : 'Use selected Proton'}</button>
    {error && <p className="pm-repair-danger" role="alert">{error}</p>}{message && <p className="pm-auto-message" role="status">{message}</p>}
  </section>;
}
