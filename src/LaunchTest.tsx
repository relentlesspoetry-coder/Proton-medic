import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AlertTriangle, CheckCircle2, ChevronDown, Clock3, Gamepad2, Play, ShieldCheck } from 'lucide-react';
import type { Game, LaunchResult } from './types';

/** An explicit launch observation, never a claim of verified compatibility. */
export function LaunchTest({ game }: { game: Game }) {
  const [approved, setApproved] = useState(false);
  const [testing, setTesting] = useState(false);
  const [result, setResult] = useState<LaunchResult | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [playable, setPlayable] = useState(false);

  const test = async () => {
    if (!approved || testing) return;
    setTesting(true);
    setFailure(null);
    setResult(null);
    setPlayable(false);
    try {
      const observed = await invoke<LaunchResult>('test_game_launch', {
        appId: game.appId,
        libraryPath: game.libraryPath,
        seconds: 10,
        confirmed: true,
      });
      setResult(observed);
    } catch (error) {
      setFailure(`Could not test this game: ${String(error)}`);
    } finally { setTesting(false); }
  };

  const hasFindings = result?.status === 'log-error' || result?.status === 'log-warning' || result?.status === 'launcher-error' || result?.status === 'log-unreadable';
  return <section className="pm-section pm-launch" aria-label="Compatibility launch test">
    <div className="pm-section-heading"><div><h2>Test Game Launch</h2><p>Try this game's current Proton selection and check for a fresh error log.</p></div><Gamepad2 className="pm-green" size={23}/></div>
    <div className="pm-launch-instructions">
      <Clock3 size={21}/>
      <div><strong>10-second launch observation</strong><p>Steam will open this game. Proton Medic will monitor new log evidence for 10 seconds. If automatic logging is unavailable, the launch still proceeds using existing Steam settings. Your game may remain open afterward.</p></div>
    </div>
    <label className="pm-launch-consent"><input aria-label="I approve opening this game through Steam" type="checkbox" checked={approved} disabled={testing} onChange={e => setApproved(e.target.checked)}/><span>I approve enabling game logs and opening this game in Steam.</span></label>
    <button className="pm-button pm-button-primary" disabled={!approved || testing || !game.installPathExists} onClick={() => void test()}>
      <Play size={17}/>{testing ? 'Launching and observing…' : 'Launch & Check Game'}
    </button>
    {failure && <div className="pm-repair-danger" role="alert"><AlertTriangle size={18}/>{failure}</div>}
    {result && <div className="pm-launch-results" role="status">
      <div className="pm-launch-status"><span className={`pm-launch-indicator ${hasFindings ? 'warn' : 'neutral'}`}>{hasFindings ? <AlertTriangle size={18}/> : <ShieldCheck size={18}/>}</span><div><strong>{hasFindings ? 'Review launch findings' : 'Launch checked — confirmation needed'}</strong><p>{result.detail}</p></div></div>
      <div className="pm-launch-facts"><span>New Proton log: <strong>{result.freshLog ? 'Detected' : 'Not detected'}</strong></span><span>Observation: <strong>{result.observedSeconds}s</strong></span></div>
      {result.findingCodes.length > 0 && <details className="pm-launch-technical"><summary>Log findings <ChevronDown size={16}/></summary><code>{result.findingCodes.join(', ')}</code></details>}
      <div className="pm-launch-confirm"><p>Did the game reach a playable screen? Proton Medic cannot verify this automatically from logs alone.</p>
        <button className="pm-button pm-button-secondary" disabled={playable} onClick={() => setPlayable(true)}><CheckCircle2 size={17}/>{playable ? 'Playable — confirmed by you' : 'Yes, the game works'}</button>
        {playable && <small>Confirmed for this test only. No permanent compatibility guarantee is recorded.</small>}
      </div>
    </div>}
    <p className="pm-launch-footnote">This is a launch probe, not unattended gameplay testing. It does not change your compatibility setting, remove data, or automatically cycle through Proton versions.</p>
  </section>;
}
