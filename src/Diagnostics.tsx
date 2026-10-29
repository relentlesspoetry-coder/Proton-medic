import { AlertTriangle, Activity, CircleHelp, ClipboardList, SearchCheck } from 'lucide-react';
import type { Finding, GameDiagnostics } from './types';

function dateLabel(seconds: number | null): string {
  return seconds == null ? 'Unknown' : new Date(seconds * 1000).toLocaleString();
}

function DiagnosticBadge({ status }: { status: GameDiagnostics['status'] }) {
  const labels: Record<GameDiagnostics['status'], string> = {
    'no-log': 'No Proton log',
    'review': 'Signatures to review',
    'no-signatures': 'No recognized signatures',
    'unreadable': 'Log unreadable',
  };
  return <span className={`diagnostic-status ${status}`}>{labels[status]}</span>;
}

function FindingView({ finding }: { finding: Finding }) {
  return <article className={`finding-item ${finding.severity}`}>
    <div className="finding-head">
      {finding.severity === 'error' ? <AlertTriangle size={17}/> : finding.severity === 'warning' ? <Activity size={17}/> : <CircleHelp size={17}/>}
      <strong>{finding.title}</strong>
      <span className={`severity-chip ${finding.severity}`}>{finding.severity}</span>
    </div>
    <p>{finding.description}</p>
    <div className="finding-action"><strong>Suggested next step</strong><span>{finding.recommendation}</span></div>
    <details className="evidence-details">
      <summary>Evidence ({finding.evidence.length} excerpts)</summary>
      {finding.evidence.map((evidence, i) => <pre key={`${finding.code}:${i}`}>{evidence}</pre>)}
    </details>
  </article>;
}

export function GameDiagnosis({ result }: { result: GameDiagnostics | null }) {
  if (!result) return <div className="diag-placeholder"><ClipboardList size={19}/> No analysis has been performed for this game yet. Choose <strong>Analyze logs</strong> to scan available Proton logs.</div>;
  return <div className="diagnosis">
    <div className="diag-label-row"><strong>Log analysis</strong><DiagnosticBadge status={result.status}/></div>
    {result.logPath ? <details className="pm-update-details"><summary>Log file details</summary>
      <div className="diag-field"><span>Log location</span><code>{result.logPath}</code></div>
      <div className="diag-field"><span>Modified</span><span>{dateLabel(result.modifiedUnixSeconds)}</span></div>
      <div className="diag-field"><span>Size</span><span>{result.logSize == null ? 'Unknown' : `${(result.logSize / 1024).toFixed(1)} KiB`}</span></div>
    </details> : null}
    {result.status === 'no-log' && <div className="capture-guide"><strong>Capture a Proton log</strong>
      <p>Open Steam → select the game → Properties → General → Launch Options. Add the following before the existing command (preserve any existing options), then launch the game again:</p>
      <code>PROTON_LOG=1 %command%</code>
      <p>After reproducing the issue, return here and select Analyze logs. Steam normally writes <code>~/steam-APPID.log</code>. An existing custom PROTON_LOG_DIR is not auto-discovered.</p>
    </div>}
    {result.status === 'no-signatures' && <p className="diag-neutral"><SearchCheck size={17}/> No known patterns were found in the analyzed log tail. This does not verify compatibility or rule out crashes.</p>}
    {result.status === 'unreadable' && <p className="diag-neutral">This file could not be analyzed; review the note below.</p>}
    {result.notes.map((note, index) => <p className="diag-note" key={index}>{note}</p>)}
    {result.findings.map(finding => <FindingView key={finding.code} finding={finding}/>)}
  </div>;
}
