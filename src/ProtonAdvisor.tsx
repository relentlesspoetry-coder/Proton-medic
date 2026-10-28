import { ChevronDown, CircleHelp, Gamepad2, Info, ShieldCheck, RefreshCw } from 'lucide-react';
import type { VersionAdvice } from './types';

export function ProtonAdvisor({ advice, loading, applying, message, onApply }: {
  advice?: VersionAdvice; loading: boolean; applying: boolean; message: string | null; onApply: () => void;
}) {
  const pick = advice?.suggestedTool;
  const recommendedLabel = advice?.status === 'steam-default'
    ? 'Steam default (automatic)'
    : pick ?? (advice?.status === 'check-system' ? 'Check your system first' :
      advice?.status === 'check-install' ? 'Find your game files first' :
      advice?.status === 'need-proton' ? 'Install Proton through Steam' : 'More information needed');
  const canSwitch = advice?.status === 'try-version' && Boolean(pick);

  return <section className="pm-section pm-advisor" aria-label="Proton recommendation">
    <div className="pm-section-heading">
      <div><h2>Recommended Proton</h2><p>We check your Steam settings and diagnostic evidence automatically.</p></div>
      <span className="pm-advisor-label"><Gamepad2 size={16}/> Proton Advisor</span>
    </div>
    {loading || !advice ? <div className="pm-advisor-loading"><CircleHelp size={17}/> {loading ? 'Checking available Proton versions…' : 'No recommendation is available yet. Try Scan Again.'}</div> : <>
      <div className="pm-advisor-primary">
        <span className="pm-advisor-icon"><Gamepad2 size={26}/></span>
        <div><small>Suggested next step</small><strong>{recommendedLabel}</strong><p>{advice.reason}</p></div>
      </div>
      <div className="pm-advisor-step"><ShieldCheck size={18}/><span>{advice.nextStep}</span></div>
      {canSwitch && <div className="pm-auto-action">
        <button type="button" className="pm-button pm-button-primary" disabled={applying} onClick={onApply}>
          <RefreshCw size={17} className={applying ? 'pm-spin' : ''}/>{applying ? 'Selecting Proton…' : `Automatically select ${pick}`}
        </button>
        <p>Proton Medic will configure this version for the game. Steam must be closed. Your current configuration is backed up first.</p>
      </div>}
      {message && <div className="pm-auto-message" role="status">{message}</div>}
      {canSwitch && <details className="pm-advisor-steps">
        <summary>How to try {pick} in Steam <ChevronDown size={17}/></summary>
        <ol><li>Open Steam and select this game in your Library.</li><li>Open <strong>Properties → Compatibility</strong>.</li><li>Check <strong>Force the use of a specific Steam Play compatibility tool</strong>.</li><li>Select <strong>{pick}</strong> and try launching the game.</li><li>If it still fails, return to Proton Medic and analyze the new log.</li></ol>
      </details>}
      <details className="pm-advisor-steps pm-advisor-secondary"><summary>Other installed options and technical details <ChevronDown size={17}/></summary>
        <p>Currently selected in Steam: {advice.configuredTool ?? 'No per-game override found'}</p>
        <p>Detected alternatives: {advice.alternatives.length ? advice.alternatives.join(', ') : 'None found'}</p>
        <p>Evidence identifiers: {advice.evidence.length ? advice.evidence.join(', ') : 'No matching error signatures'}</p>
      </details>
      <p className="pm-advisor-disclaimer"><Info size={15}/> Proton Medic chooses the candidate; game launch is required to verify it works. Automatic changes are opt-in and backed up.</p>
    </>}
  </section>;
}
