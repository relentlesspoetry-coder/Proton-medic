import { AlertTriangle, Activity, CircleHelp, ClipboardList, Cpu, FileText, HardDrive, Monitor, SearchCheck, ShieldCheck, Terminal } from 'lucide-react';
import type { DiagnosticsReport, Finding, Game, GameDiagnostics, VulkanProbe } from './types';

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
    {result.logPath ? <>
      <div className="diag-field"><span>Log location</span><code>{result.logPath}</code></div>
      <div className="diag-field"><span>Modified</span><span>{dateLabel(result.modifiedUnixSeconds)}</span></div>
      <div className="diag-field"><span>Size</span><span>{result.logSize == null ? 'Unknown' : `${(result.logSize / 1024).toFixed(1)} KiB`}</span></div>
    </> : null}
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

export function DiagnosticsView({
  report, games, loading, probe, probing, onAnalyze, onProbe, onSelectGame,
}: {
  report: DiagnosticsReport | null;
  games: Game[];
  loading: boolean;
  probe: VulkanProbe | null;
  probing: boolean;
  onAnalyze: () => void;
  onProbe: () => void;
  onSelectGame: (game: Game) => void;
}) {
  const analyzed = report?.games.filter(game => game.logPath !== null).length ?? 0;
  const flagged = report?.games.filter(game => game.findings.length > 0).length ?? 0;
  return <>
    <div className="page-header"><div className="eyebrow"><span className="accent-line"/> PHASE 2 / DIAGNOSTICS</div>
      <h1>Proton diagnostics</h1>
      <p>Analyze existing Proton logs and inspect host graphics configuration. Findings are indicators, not confirmed root causes. All actions are read-only.</p>
    </div>
    <div className="diagnostics-toolbar">
      <button type="button" className="button primary" disabled={loading} onClick={onAnalyze}><FileText size={16}/>{loading ? 'Analyzing...' : 'Analyze Proton logs'}</button>
      <button type="button" className="button secondary" disabled={probing} onClick={onProbe}><Cpu size={16}/>{probing ? 'Checking Vulkan...' : 'Run Vulkan probe'}</button>
    </div>
    <div className="diag-metrics">
      <div><span>Games inspected</span><strong>{report?.games.length ?? '—'}</strong></div>
      <div><span>Logs discovered</span><strong>{report ? analyzed : '—'}</strong></div>
      <div><span>Games with findings</span><strong>{report ? flagged : '—'}</strong></div>
    </div>
    <section className="surface-panel diag-section">
      <div className="section-heading"><div><span className="eyebrow">HOST ENVIRONMENT</span><h2>Graphics & system inspection</h2></div><Monitor size={20}/></div>
      {!report ? <div className="diag-section-content"><p className="diag-neutral">Run Analyze Proton logs to inspect GPU identity, host Vulkan ICD files, and system metadata without running external processes.</p></div> : <div className="diag-section-content">
        <div className="system-grid"><div><span>Distribution</span><strong>{report.system.os}</strong></div><div><span>Kernel</span><strong>{report.system.kernel}</strong></div><div><span>Architecture</span><strong>{report.system.arch}</strong></div><div><span>vulkaninfo</span><strong>{report.system.vulkaninfoPath ? 'Available' : 'Not installed / not in PATH'}</strong></div></div>
        <h3>GPU devices ({report.system.gpuDevices.length})</h3>
        {report.system.gpuDevices.length ? report.system.gpuDevices.map((item, i) => <code className="diag-line" key={i}>{item}</code>) : <p className="diag-neutral">No PCI GPU devices were identified through /sys/class/drm in this session.</p>}
        <h3>Vulkan ICD manifests ({report.system.icdManifests.length})</h3>
        {report.system.icdManifests.length ? report.system.icdManifests.map((item, i) => <code className="diag-line" key={i}>{item}</code>) : <p className="diag-neutral">No ICD manifests found in the inspected system directories. Flatpak may have a different view.</p>}
        {report.system.notes.map((note, i) => <p key={i} className="diag-note">{note}</p>)}
      </div>}
      {probe && <div className="probe-results">
        <div className="diag-label-row"><strong>Optional Vulkan probe result</strong><span className={`diagnostic-status ${probe.status === 'available' ? 'no-signatures' : probe.status === 'unavailable' ? 'no-log' : 'unreadable'}`}>{probe.status}</span></div>
        <p>{probe.detail}</p>
        {probe.output.trim() && <details className="evidence-details"><summary>View vulkaninfo output (limited)</summary><pre>{probe.output}</pre></details>}
      </div>}
    </section>
    <section className="surface-panel diag-section">
      <div className="section-heading"><div><span className="eyebrow">LOG REVIEW</span><h2>Game-specific findings</h2></div><HardDrive size={19}/></div>
      {!report ? <div className="diag-section-content"><p className="diag-neutral"><ShieldCheck size={17}/> No game logs analyzed yet. The scanner never changes your game files.</p></div>
        : <div className="diagnostic-game-list">
          {report.games.length === 0 && <div className="diag-section-content"><p className="diag-neutral">No games were found in supported Steam libraries.</p></div>}
          {report.games.map(result => {
            const game = games.find(g => g.appId === result.appId && g.libraryPath === result.libraryPath);
            return <div className="diagnostic-game" key={`${result.libraryPath}:${result.appId}`}>
              <div className="diagnostic-game-heading"><div><strong>{game?.name ?? `App ${result.appId}`}</strong><span className="mono">Steam {result.appId}</span></div><DiagnosticBadge status={result.status}/></div>
              <div className="diagnostic-game-foot"><span>{result.findings.length ? `${result.findings.length} signature group(s) detected` : result.logPath ? 'No matching error patterns found' : 'Enable PROTON_LOG to capture evidence'}</span>
              {game && <button className="text-button" type="button" onClick={() => onSelectGame(game)}>Inspect game →</button>}</div>
            </div>;
          })}
        </div>}
    </section>
    <div className="note-box"><Terminal size={16}/><span>No automatic fixes, downloads, telemetry, or game launches. The optional Vulkan probe executes only <code>vulkaninfo --summary</code> with a 6-second timeout; it cannot verify the GPU selected by Steam Runtime.</span></div>
  </>;
}
