import { ProtonManager } from './ProtonManager';
import { GameProtonPicker } from './GameProtonPicker';
import { SupportReport } from './SupportReport';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  AlertCircle, AlertTriangle, ArrowLeft, ArrowRight, Check, CheckCircle2,
  ChevronDown, CircleHelp, Clock3, Download,
  Gamepad2, HardDrive, Home, Info, Layers, Monitor,
  RefreshCw, Search, Settings2, ShieldCheck, Wrench, X,
} from 'lucide-react';
import type { AdvisorReport, AutoSelection, DiagnosticsReport, Game, GameDiagnostics, ScanReport, VulkanProbe } from './types';
import { GameDiagnosis } from './Diagnostics';
import { RepairTools } from './Repairs';
import { ProtonAdvisor } from './ProtonAdvisor';
import { WindowsGames } from './WindowsGames';
import { UnifiedLibrary, type LibraryFilter, type LibrarySource, type LibrarySort } from './UnifiedLibrary';
import { AutoLogMonitor } from './AutoLogMonitor';
import { useAutoFixControl } from './useAutoFixControl';
import { ActiveFixSessions } from './ActiveFixSessions';
import { FixAndTest } from './FixAndTest';
import { AppUpdater } from './AppUpdater';
import { PagedList } from './PagedList';
import { GameDetailTabs } from './GameDetailTabs';
import { GameIcon } from './GameIcon';
import { LaunchTest } from './LaunchTest';
import programLogo from './assets/proton-medic-logo.webp';

type Page = 'protons' | 'windows' | 'home' | 'games' | 'fixes' | 'system' | 'settings';
const MENU: { page: Page; label: string; icon: typeof Home }[] = [
  { page: 'home', label: 'Home', icon: Home },
  { page: 'games', label: 'My Games', icon: Gamepad2 },
  { page: 'windows', label: 'Windows Games', icon: HardDrive },
  { page: 'protons', label: 'Proton versions', icon: Layers },
  { page: 'fixes', label: 'Fixes', icon: Wrench },
  { page: 'system', label: 'System Check', icon: Monitor },
  { page: 'settings', label: 'Settings', icon: Settings2 },
];
const gameKey = (g: { libraryPath: string; appId: string }) => `${g.libraryPath}:${g.appId}`;

function getIssue(game: Game, result: GameDiagnostics | undefined) {
  // A prefix existing, a missing log, or a clean signature scan never proves a game works.
  if (!game.installPathExists) return { label: 'Folder missing', tone: 'error' as const, note: 'Steam lists this game, but its installation folder could not be found.' };
  if (result?.status === 'unreadable') return { label: 'Needs review', tone: 'warning' as const, note: 'A Proton log exists, but it could not be read.' };
  if (result?.findings.some(f => f.severity === 'error')) return { label: 'Needs attention', tone: 'error' as const, note: 'Proton log messages suggest a possible compatibility problem.' };
  if (result?.findings.length) return { label: 'Needs review', tone: 'warning' as const, note: 'Proton log messages need a closer look.' };
  if (result?.status === 'no-log') return { label: 'Not checked', tone: 'neutral' as const, note: 'No Proton log was found. This does not mean the game has a problem.' };
  if (result?.status === 'no-signatures') return { label: 'No known errors', tone: 'ok' as const, note: 'No recognized error patterns were found. This does not confirm the game works.' };
  return { label: 'Not checked', tone: 'neutral' as const, note: 'No diagnostic result is available yet.' };
}

function StatusTag({ label, tone }: { label: string; tone: 'error' | 'warning' | 'neutral' | 'ok' }) {
  return <span className={`pm-tag ${tone}`}>{tone === 'ok' ? <CheckCircle2 size={14}/> : tone === 'neutral' ? <CircleHelp size={14}/> : <AlertCircle size={14}/>} {label}</span>;
}

export default function App() {
  const [page, setPage] = useState<Page>('games');
  const [scan, setScan] = useState<ScanReport | null>(null);
  const [report, setReport] = useState<DiagnosticsReport | null>(null);
  const [advice, setAdvice] = useState<AdvisorReport | null>(null);
  const [advising, setAdvising] = useState(false);
  const [vulkan, setVulkan] = useState<VulkanProbe | null>(null);
  const [loading, setLoading] = useState(false);
  const [analyzing, setAnalyzing] = useState(false);
  const [probing, setProbing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastScan, setLastScan] = useState<Date | null>(null);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const autoControl = useAutoFixControl();
  const [autoMessage, setAutoMessage] = useState<{ gameKey: string; message: string } | null>(null);
  const [autoBusy, setAutoBusy] = useState(false);
  const [query, setQuery] = useState('');
  const [libraryPage, setLibraryPage] = useState(0);
  const [filter, setFilter] = useState<LibraryFilter>('all');
  const [librarySource,setLibrarySource]=useState<LibrarySource>('all');
  const [librarySort,setLibrarySort]=useState<LibrarySort>('name');
  const [selectedWindowsId,setSelectedWindowsId]=useState<string|undefined>();
  const taskId = useRef(0);
  const analysisId = useRef(0);
  const advisorId = useRef(0);

  const analyzeLogs = useCallback(async () => {
    const current = ++analysisId.current;
    setAnalyzing(true);
    try {
      const diagnostics = await invoke<DiagnosticsReport>('analyze_diagnostics');
      if (current === analysisId.current) setReport(diagnostics);
    } catch (e) {
      if (current === analysisId.current) setError(`Could not read diagnostic logs: ${String(e)}`);
    } finally {
      if (current === analysisId.current) setAnalyzing(false);
    }
  }, []);

  const scanGames = useCallback(async () => {
    const current = ++taskId.current;
    ++analysisId.current;
    const recommendationTask = ++advisorId.current;
    setAdvice(null);
    setAdvising(true);
    setLoading(true);
    setAnalyzing(false);
    setError(null);
    try {
      const next = await invoke<ScanReport>('scan_steam');
      if (current !== taskId.current) return;
      setScan(next);
      setSelectedKey(key => key && next.games.some(g => gameKey(g) === key) ? key : null);
      setLastScan(new Date());
      setReport(null);
      // Proton advice is calculated locally; opt-in selection runs separately after diagnostics.
      void invoke<AdvisorReport>('recommend_proton')
        .then(result => {
          if (current === taskId.current && recommendationTask === advisorId.current) setAdvice(result);
        })
        .catch(e => {
          if (current === taskId.current && recommendationTask === advisorId.current)
            setError(`Could not recommend a Proton version: ${String(e)}`);
        })
        .finally(() => {
          if (current === taskId.current && recommendationTask === advisorId.current) setAdvising(false);
        });
      // Existing logs can be analyzed read-only without asking the user to find them.
      const analysisTask = ++analysisId.current;
      setAnalyzing(true);
      try {
        const diagnostics = await invoke<DiagnosticsReport>('analyze_diagnostics');
        if (current === taskId.current && analysisTask === analysisId.current) setReport(diagnostics);
      } catch (e) {
        if (current === taskId.current) setError(`Games were found, but logs could not be analyzed: ${String(e)}`);
      } finally {
        if (current === taskId.current) setAnalyzing(false);
      }
    } catch (e) {
      if (current === taskId.current) {
        setAdvising(false);
        setError(`Could not scan Steam libraries: ${String(e)}`);
      }
    } finally {
      if (current === taskId.current) setLoading(false);
    }
  }, []);

  useEffect(() => { void scanGames(); }, [scanGames]);

  const selectAutomatically = useCallback(async (appId: string, libraryPath: string) => {
    setAutoBusy(true);
    setAutoMessage(null);
    const selectionKey = gameKey({ appId, libraryPath });
    try {
      const result = await invoke<AutoSelection>('auto_select_proton', { appId, libraryPath, confirmed: true });
      setAutoMessage({ gameKey: selectionKey, message: result.changed
        ? `${result.gameName}: ${result.toolName} selected. Restart Steam, then test your game. Config backup: ${result.backupPath}`
        : `${result.gameName}: ${result.toolName} is already selected.` });
      // A new scan shows the updated per-game override.
      if (result.changed) void scanGames();
    } catch (e) {
      setAutoMessage({ gameKey: selectionKey, message: `Could not apply the selection: ${String(e)}` });
    } finally { setAutoBusy(false); }
  }, [scanGames]);

  const checkVulkan = useCallback(async () => {
    setProbing(true);
    setError(null);
    try { setVulkan(await invoke<VulkanProbe>('probe_vulkan')); }
    catch (e) { setError(`Vulkan check failed: ${String(e)}`); }
    finally { setProbing(false); }
  }, []);

  const issueByGame = useMemo(() => new Map(report?.games.map(d => [gameKey(d), d]) ?? []), [report]);
  const problems = useMemo(() => (scan?.games ?? []).filter(g => {
    const issue = getIssue(g, issueByGame.get(gameKey(g)));
    return issue.tone === 'error' || issue.tone === 'warning';
  }), [scan, issueByGame]);
  const problemKeys = useMemo(() => new Set(problems.map(gameKey)), [problems]);
  const selectedGame = scan?.games.find(g => gameKey(g) === selectedKey) ?? null;
  const selectedDiagnosis = selectedGame ? issueByGame.get(gameKey(selectedGame)) ?? null : null;
  const openGame = (game: Game) => { setSelectedWindowsId(undefined); setSelectedKey(gameKey(game)); setPage('games'); };
  const showProblems = () => { setLibraryPage(0); setSelectedWindowsId(undefined); setLibrarySource('all'); setSelectedKey(null); setQuery(''); setFilter('issues'); setPage('games'); };
  const showAllGames = () => { setLibraryPage(0); setSelectedWindowsId(undefined); setLibrarySource('all'); setSelectedKey(null); setQuery(''); setFilter('all'); setPage('games'); };
  const status = !scan ? 'Not scanned' : scan.installations.length ? 'Steam detected' : 'Steam not found';

  return <div className="pm-app">
    <aside className="pm-sidebar">
      <div className="pm-brand"><img className="pm-brand-logo" src={programLogo} alt="Proton Medic" width={768} height={512}/></div>
      <nav aria-label="Main navigation" className="pm-nav">
        <div className="pm-nav-group">{(['games', 'protons', 'fixes', 'settings'] as Page[]).map(id => {
          const { label, icon: Icon } = MENU.find(item => item.page === id)!;
          return <button className={`pm-nav-link ${page === id ? 'active' : ''}`} aria-label={id === 'games' ? 'Library' : id === 'fixes' ? 'Diagnostics' : label} aria-current={page === id ? 'page' : undefined} type="button" key={id} onClick={() => { if (id === 'games') { setSelectedKey(null); setSelectedWindowsId(undefined); } setPage(id); }}><Icon size={21}/><span>{id === 'games' ? 'Library' : id === 'fixes' ? 'Diagnostics' : label}</span></button>;
        })}</div>
        <div className="pm-nav-group pm-nav-secondary">{(['home', 'windows', 'system'] as Page[]).map(id => {
          const { label, icon: Icon } = MENU.find(item => item.page === id)!;
          return <button className={`pm-nav-link ${page === id ? 'active' : ''}`} aria-current={page === id ? 'page' : undefined} type="button" key={id} onClick={() => setPage(id)}><Icon size={18}/><span>{label}</span></button>;
        })}</div>
      </nav>
    </aside>
    <main className="pm-main">
      <div className="pm-workspace-header">{page === 'games' && !selectedGame && !selectedWindowsId ? <h1>My Games</h1> : <span aria-hidden="true"/>}<AppUpdater/></div>
      <ActiveFixSessions/>
      {error && <div className="pm-error" role="alert"><AlertTriangle size={18}/><p>{error}</p><button type="button" onClick={() => setError(null)} aria-label="Dismiss error"><X size={17}/></button></div>}
      {page === 'home' && <>
        <div className="pm-welcome"><div><span className="pm-eyebrow">Your gaming workspace</span><h1>Ready for your<br/><span>next game.</span></h1><p>One place for your library, Proton, and game repairs.</p></div><button className="pm-button pm-button-primary pm-scan-action" onClick={() => void scanGames()} disabled={loading}>{loading ? <RefreshCw size={18} className="pm-spin"/> : <Search size={18}/>} {loading ? 'Scanning library…' : 'Scan your library'}</button></div>
        <div className="pm-metrics">
          <div className="pm-metric steam"><div className="pm-metric-icon"><CheckCircle2 size={24}/></div><div><strong>{status}</strong><span>{scan?.installations[0]?.kind ?? 'Native or Flatpak'} Steam</span></div></div>
          <div className="pm-metric"><div className="pm-metric-icon"><Gamepad2 size={25}/></div><div><strong className="pm-metric-number">{scan?.games.length ?? '—'}</strong><span>Steam games found</span></div></div>
          <div className="pm-metric"><div className="pm-metric-icon attention"><AlertCircle size={24}/></div><div><strong className="pm-metric-number pm-error-number">{scan && report ? problems.length : '—'}</strong><span>{analyzing ? 'checking logs...' : 'need attention'}</span></div></div>
          <div className="pm-metric"><div className="pm-metric-icon"><Clock3 size={24}/></div><div><strong>Last scan</strong><span>{lastScan ? lastScan.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }) : 'Not yet'}</span></div></div>
        </div>
        <div className="pm-action-grid">
          <button className="pm-action-card featured" onClick={showAllGames}><div className="pm-action-icon"><Gamepad2 size={32}/></div><span><strong>Explore your games</strong><small>Logs, launch tests, and repairs</small></span><ArrowRight size={20}/></button>
          <button className="pm-action-card" onClick={() => setPage('windows')}><div className="pm-action-icon"><Download size={26}/></div><span><strong>Windows games</strong><small>Install or add a game</small></span><ArrowRight size={20}/></button>
          <button className="pm-action-card" onClick={() => setPage('protons')}><div className="pm-action-icon"><Layers size={26}/></div><span><strong>Manage Proton versions</strong><small>Find the right runtime</small></span><ArrowRight size={20}/></button>
        </div>
        <section className="pm-section pm-home-games"><div className="pm-section-heading"><div><h2>Games needing attention</h2><p>Review findings from your latest scan.</p></div><button className="pm-subtle-button" onClick={showAllGames}>View all games <ArrowRight size={16}/></button></div>
          {loading && !scan ? <Empty title="Checking Steam libraries" detail="Looking for installed games and Proton logs."/> : !scan?.games.length ? <Empty title="No games found yet" detail="Open Steam, then select Scan your library."/> : !report && analyzing ? <Empty title="Checking for issues" detail="Reading existing Proton logs. No changes are made."/> : problems.length === 0 ? <Empty title="No known issues found" detail="Open a game in My Games for diagnostics and a launch test."/> : <div className="pm-problem-list">{problems.slice(0, 5).map(game => <GameRow key={gameKey(game)} game={game} diagnostic={issueByGame.get(gameKey(game))} onClick={() => openGame(game)} />)}</div>}
          {problems.length > 5 && <button className="pm-more" onClick={showProblems}>See all {problems.length} games needing attention <ArrowRight size={16}/></button>}
        </section>

      </>}

      {page === 'windows' && <WindowsGames/>}
      {page === 'games' && <>

        {selectedWindowsId ? <WindowsGames selectedId={selectedWindowsId} onBack={()=>setSelectedWindowsId(undefined)}/> : selectedGame ? <div className="pm-detail-view"><div className="pm-detail-toolbar"><button className="pm-back" onClick={() => setSelectedKey(null)}><ArrowLeft size={17}/> Back to My Games</button><button className="pm-button pm-button-secondary" disabled={loading} onClick={() => void scanGames()}><RefreshCw size={16} className={loading ? 'pm-spin' : ''}/>{loading ? 'Scanning...' : 'Scan Again'}</button></div><div className="pm-selected-head"><GameIcon className="big" source={{ kind: 'steam', appId: selectedGame.appId, libraryPath: selectedGame.libraryPath }}/><div><h1>{selectedGame.name}</h1><span className="pm-game-subtitle">Steam App {selectedGame.appId}</span><div className="pm-detail-status"><StatusTag {...getIssue(selectedGame, selectedDiagnosis ?? undefined)}/></div><p>{getIssue(selectedGame, selectedDiagnosis ?? undefined).note}</p></div></div>
          <GameDetailTabs key={gameKey(selectedGame)} tabs={[
            { id: 'diagnostics', label: 'Diagnostics', content: <>
          <div className="pm-section"><div className="pm-section-heading"><div><h2>What Proton Medic Found</h2><p>Existing logs are checked without changing your game.</p></div><button className="pm-subtle-button" disabled={analyzing} onClick={() => void analyzeLogs()}><RefreshCw size={16} className={analyzing ? 'pm-spin' : ''}/>{analyzing ? 'Checking...' : 'Analyze Again'}</button></div>
            {!selectedGame.installPathExists && <div className="pm-callout"><AlertTriangle size={18}/> Game install folder is missing. Check the Steam installation or mount the drive.</div>}
            <GameDiagnosis result={selectedDiagnosis}/>
          </div>
            </> },
            { id: 'proton', label: 'Proton', content: <>
          <GameProtonPicker key={gameKey(selectedGame)} game={selectedGame} onChanged={() => void scanGames()} onManage={() => setPage('protons')}/>
          <ProtonAdvisor advice={advice?.games.find(a => a.appId === selectedGame.appId && a.libraryPath === selectedGame.libraryPath)} loading={advising}
            applying={autoBusy} message={autoMessage?.gameKey === gameKey(selectedGame) ? autoMessage.message : null} onApply={() => void selectAutomatically(selectedGame.appId, selectedGame.libraryPath)}/>
            </> },
            { id: 'launch', label: 'Launch & repair', content: <>
          <FixAndTest key={gameKey(selectedGame)} game={selectedGame}/>
          <LaunchTest key={gameKey(selectedGame)} game={selectedGame}/>

            </> },
            { id: 'backups', label: 'Backups & details', content: <>
          <details className="pm-section pm-technical"><summary>Advanced repairs and backups <ChevronDown size={17}/></summary><RepairTools key={gameKey(selectedGame)} game={selectedGame} onChanged={() => void scanGames()}/></details>
          <details className="pm-section pm-technical"><summary>Technical Details <ChevronDown size={17}/></summary><dl><dt>Install directory</dt><dd>{selectedGame.installPath}</dd><dt>Steam library</dt><dd>{selectedGame.libraryPath}</dd><dt>Proton prefix folder</dt><dd>{selectedGame.prefixPath} — {selectedGame.prefixExists ? 'present' : 'not found'}</dd><dt>Manifest</dt><dd>{selectedGame.manifestPath}</dd><dt>Build ID</dt><dd>{selectedGame.buildId ?? 'Unknown'}</dd><dt>Steam compatibility override</dt><dd>{selectedGame.configuredTool ?? 'Automatic / no override detected'}</dd></dl></details>
            </> },
          ]}/>
        </div> : <UnifiedLibrary steam={scan?.games??[]} steamLoading={loading} scanAvailable={!!scan} refreshKey={lastScan?.getTime()??0} problemKeys={problemKeys}
          page={libraryPage} onPageChange={setLibraryPage}
          query={query} onQuery={value => { setQuery(value); setLibraryPage(0); }} filter={filter} onFilter={value => { setFilter(value); setLibraryPage(0); }} source={librarySource} onSource={value => { setLibrarySource(value); setLibraryPage(0); }} sort={librarySort} onSort={value => { setLibrarySort(value); setLibraryPage(0); }}
          onAddGame={() => setPage('windows')} onScan={() => void scanGames()} scanning={loading}
          onSteam={openGame} onWindows={game=>{setSelectedKey(null);setSelectedWindowsId(game.id);}} steamStatus={game=><StatusTag {...getIssue(game,issueByGame.get(gameKey(game)))}/>}/>}

      </>}

      {page === 'fixes' && <><div className="pm-page-title"><div><h1>Fixes</h1><p>Find an issue. Review a repair. Get back to your game.</p></div></div><section className="pm-section"><div className="pm-section-heading"><div><h2>Games needing attention</h2><p>Choose a game to review its logs and repair options.</p></div><button className="pm-subtle-button" disabled={analyzing} onClick={() => void analyzeLogs()}><RefreshCw size={16} className={analyzing ? 'pm-spin' : ''}/> Check logs again</button></div>{problems.length ? <PagedList items={problems} render={g => <GameRow key={gameKey(g)} game={g} diagnostic={issueByGame.get(gameKey(g))} onClick={() => openGame(g)}/>}/> : <Empty title="No problems identified yet" detail="You can still open My Games to view backup and repair options."/>}</section><div className="pm-callout"><ShieldCheck size={19}/><div><strong>Only approved changes</strong><p>Game data is archived instead of deleted. Close Steam before repairing or restoring. All backups remain on the same Steam library drive.</p></div></div></>}

      {page === 'protons' && <ProtonManager onChanged={() => void scanGames()}/>}
      {page === 'system' && <><div className="pm-page-title"><div><h1>System Check</h1><p>Graphics, drivers, and the tools your games need.</p></div><button className="pm-button pm-button-primary" disabled={probing} onClick={() => void checkVulkan()}><Monitor size={18}/>{probing ? 'Checking Vulkan...' : 'Check Vulkan'}</button></div>
        <section className="pm-section"><h2>Your system at a glance</h2>{!report ? <Empty title="System information isn't available yet" detail="Select Check Logs & System to inspect GPU and driver information."/> : <div className="pm-check-list"><CheckRow label="Linux system" detail={report.system.os} state="ok"/><CheckRow label="Kernel" detail={report.system.kernel} state="neutral"/><CheckRow label="CPU architecture" detail={report.system.arch} state="neutral"/><CheckRow label="Graphics devices" detail={`${report.system.gpuDevices.length} device(s) detected`} state={report.system.gpuDevices.length ? 'ok' : 'warning'}/><CheckRow label="Vulkan utility" detail={report.system.vulkaninfoPath ? 'vulkaninfo is available' : 'vulkaninfo was not found in PATH'} state={report.system.vulkaninfoPath ? 'ok' : 'warning'}/><CheckRow label="Vulkan driver manifests" detail={`${report.system.icdManifests.length} found in checked locations`} state={report.system.icdManifests.length ? 'ok' : 'warning'}/></div>}
          <div className="pm-system-actions"><button className="pm-button pm-button-secondary" onClick={() => void analyzeLogs()} disabled={analyzing}><RefreshCw size={16} className={analyzing ? 'pm-spin' : ''}/>{analyzing ? 'Reading logs...' : 'Check Logs & System'}</button></div>
          {vulkan && <div className="pm-probe"><StatusTag tone={vulkan.status === 'available' ? 'ok' : 'warning'} label={vulkan.status === 'available' ? 'Vulkan probe succeeded' : `Vulkan: ${vulkan.status}`}/><p>{vulkan.detail}</p><details><summary>Technical output</summary><pre>{vulkan.output || 'No output returned.'}</pre></details></div>}
        </section><details className="pm-section pm-technical"><summary>Advanced system details <ChevronDown size={17}/></summary>{report && <div className="pm-system-advanced"><h3>Graphics devices</h3>{report.system.gpuDevices.map((g,i) => <pre key={i}>{g}</pre>)}<h3>Vulkan ICD files</h3>{report.system.icdManifests.map((v,i) => <pre key={i}>{v}</pre>)}{report.system.notes.map((n,i) => <p key={i}>{n}</p>)}</div>}</details></>}

      {page === 'settings' && <div className="pm-page-title"><div><h1>Settings</h1><p>Manage automatic fixes and privacy.</p></div></div>}
      {page === 'settings' && (autoControl.message || autoMessage) && <p className="pm-auto-message" role="status">{autoControl.message || autoMessage?.message}</p>}
      <AutoLogMonitor enabled={autoControl.enabled} controlPending={autoControl.pending} onToggle={enabled => void autoControl.toggle(enabled)} expanded={page === 'settings'}/>
      {page === 'settings' && <><SupportReport/><section className="pm-section"><div className="pm-section-heading"><div><h2>Privacy & safety</h2><p>Game scans and logs stay on this computer.</p></div><ShieldCheck size={23} className="pm-green"/></div><div className="pm-safety-grid"><div><ShieldCheck size={20}/><h3>Private scans</h3><p>Diagnostics stay local, with no telemetry or administrator access.</p></div><div><HardDrive size={20}/><h3>Recoverable repairs</h3><p>Approved repairs archive data on the same library drive. Close Steam before repairing or restoring.</p></div><div><Download size={20}/><h3>Verified updates</h3><p>Update checks contact GitHub. Missing Steam covers are requested from Steam using the game’s app ID. Custom covers stay local.</p></div></div></section>
        <details className="pm-section pm-technical"><summary>Available Proton versions ({scan?.compatibilityTools.length ?? 0}) <ChevronDown size={17}/></summary>{scan?.compatibilityTools.length ? scan.compatibilityTools.map(tool => <div className="pm-discovery" key={tool.path}><Layers size={18}/><div><strong>{tool.name}</strong><small>{tool.source} · {tool.path}</small></div></div>) : <p>No compatibility tools detected.</p>}</details>
        <details className="pm-section pm-technical"><summary>Steam libraries ({scan?.libraries.length ?? 0}) <ChevronDown size={17}/></summary>{scan?.libraries.length ? scan.libraries.map(lib => <div className="pm-discovery" key={lib.path}><HardDrive size={18}/><div><strong>{lib.installKind} library</strong><small>{lib.path}</small></div></div>) : <p>No libraries found.</p>}</details>
        <details className="pm-section pm-technical"><summary>Scan warnings ({scan?.warnings.length ?? 0}) <ChevronDown size={17}/></summary>{scan?.warnings.length ? scan.warnings.map((w,i) => <div key={i} className="pm-discovery"><AlertTriangle size={18}/><div>{w}</div></div>) : <p>No scan warnings.</p>}</details>
        </>}
    </main>
  </div>;
}

function GameRow({ game, diagnostic, onClick }: { game: Game; diagnostic?: GameDiagnostics; onClick: () => void }) {
  const issue = getIssue(game, diagnostic);
  return <button className="pm-game-row" onClick={onClick} type="button"><GameIcon source={{ kind: 'steam', appId: game.appId, libraryPath: game.libraryPath }}/><span className="pm-game-title"><strong>{game.name}</strong><small>Steam · App {game.appId}</small></span><span className="pm-game-issue"><StatusTag tone={issue.tone} label={issue.label}/></span><span className="pm-view-game" aria-hidden="true"><ArrowRight size={16}/></span></button>;
}
function Empty({ title, detail }: { title: string; detail: string }) { return <div className="pm-empty"><Gamepad2 size={29}/><strong>{title}</strong><p>{detail}</p></div>; }
function CheckRow({ label, detail, state }: { label: string; detail: string; state: 'ok' | 'warning' | 'neutral' }) {
  return <div className="pm-check-row"><span className={`pm-check-icon ${state}`}>{state === 'ok' ? <Check size={18}/> : state === 'warning' ? <AlertTriangle size={18}/> : <Info size={18}/>}</span><div><strong>{label}</strong><small>{detail}</small></div></div>;
}
