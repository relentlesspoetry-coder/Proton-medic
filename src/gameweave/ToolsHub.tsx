import { useEffect, useId, useRef, useState, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Activity, BarChart3, Box, ChevronDown, CircleCheck, Database, Download, ExternalLink,
  FlaskConical, Gamepad2, Info, Layers, LayoutGrid, List, RefreshCw, RotateCcw, Search,
  Settings, Sparkles, Trash2, Wine, Wrench,
} from 'lucide-react';
import { GameCover } from '../GameCover';
import { GameDiagnosis } from '../Diagnostics';
import { FixAndTest } from '../FixAndTest';
import { GameProtonPicker } from '../GameProtonPicker';
import { ProtonManager } from '../ProtonManager';
import { RepairTools } from '../Repairs';
import { ResourceMonitor } from '../ResourceMonitor';
import type { CompatibilityTool, Finding, Game, GameDiagnostics } from '../types';
import type { Game as WindowsGame, Library } from '../WindowsGames';
import './tools.css';

type Props = {
  steam: Game[];
  runtimes: CompatibilityTool[];
  diagnostics: GameDiagnostics[];
  scanning: boolean;
  onRefresh: () => void;
  onScan: () => void;
  onOpenLibrary: () => void;
  onOpenSystem: () => void;
  onOpenWindows: (game: WindowsGame) => void;
};
type Selected = { key: string; kind: 'steam'; game: Game } | { key: string; kind: 'windows'; game: WindowsGame };
type Category = 'game' | 'proton' | 'diagnostics' | 'backups' | 'system' | 'processes';
type Panel = 'fix' | 'repair' | 'proton' | 'verify';

const categories = [
  { id: 'game', label: 'Game Tools', Icon: Box },
  { id: 'proton', label: 'Proton Manager', Icon: FlaskConical },
  { id: 'diagnostics', label: 'Diagnostics', Icon: Activity },
  { id: 'backups', label: 'Backups', Icon: Database },
  { id: 'system', label: 'System', Icon: Settings },
  { id: 'processes', label: 'Processes', Icon: BarChart3 },
] as const;

function GlassCard({ title, icon, action, children }: { title: string; icon: ReactNode; action?: ReactNode; children: ReactNode }) {
  return <section className="gw-tools-card"><div className="gw-tools-card-title"><h3>{icon}{title}</h3>{action}</div>{children}</section>;
}

function WindowsScan({ game }: { game: WindowsGame }) {
  const [report, setReport] = useState<{ findings: Finding[]; logs: { name: string; status: string; truncated: boolean }[] } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const alive = useRef(false); const lock = useRef(false);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  const run = async () => {
    if (lock.current || !game.logDirectory) return;
    lock.current = true; setBusy(true); setError(''); setReport(null);
    try {
      const next = await invoke<NonNullable<typeof report>>('analyze_windows_game', { id: game.id });
      if (alive.current) setReport(next);
    } catch (failure) {
      if (alive.current) setError(`Could not read runtime logs: ${String(failure)}`);
    } finally {
      lock.current = false; if (alive.current) setBusy(false);
    }
  };
  return <GlassCard title="Game Diagnostics" icon={<Activity size={22}/>} action={<button className="gw-tools-button primary" onClick={() => void run()} disabled={busy || !game.logDirectory}>{busy ? 'Reading…' : 'Run scan'}</button>}>
    {error ? <p role="alert">{error}</p> : report ? <ul className="gw-tools-findings">{report.logs.map(log => <li key={log.name}>{log.name}: {log.status}{log.truncated ? ' (partial)' : ''}</li>)}{report.findings.map(finding => <li key={finding.code}><strong>{finding.title}</strong> · {finding.recommendation}</li>)}</ul> :
      <p>{game.logDirectory ? 'Read the logs from this game’s latest run. Nothing is changed.' : 'No runtime logs yet. Run the game before scanning.'}</p>}
    {report && report.findings.length === 0 && <p>No recognized patterns in the available logs. This does not verify game files or compatibility.</p>}
  </GlassCard>;
}

export function ToolsHub({ steam, runtimes, diagnostics, scanning, onRefresh, onScan, onOpenLibrary, onOpenSystem, onOpenWindows }: Props) {
  const prefix = useId();
  const [category, setCategory] = useState<Category>('game');
  const [view, setView] = useState<'grid' | 'list'>('grid');
  const [query, setQuery] = useState('');
  const [panel, setPanel] = useState<Panel | null>(null);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [windows, setWindows] = useState<Library | null>(null);
  const [windowsError, setWindowsError] = useState('');
  const [revision, setRevision] = useState(0);
  const [monitor, setMonitor] = useState(false);
  const [windowsLoading, setWindowsLoading] = useState(true);
  const actionPane = useRef<HTMLElement>(null);
  useEffect(() => { if (panel) actionPane.current?.focus(); }, [panel]);
  useEffect(() => {
    let disposed = false; setWindowsLoading(true);
    void invoke<Library>('list_windows_games').then(next => {
      if (!Array.isArray(next?.games) || !Array.isArray(next?.runtimes)) throw new Error('Invalid Windows library response');
      if (!disposed) { setWindows(next); setWindowsError(''); }
    }).catch(failure => { if (!disposed) setWindowsError(`Could not read Windows games: ${String(failure)}`); })
      .finally(() => { if (!disposed) setWindowsLoading(false); });
    return () => { disposed = true; };
  }, [revision]);

  const choices: Selected[] = [
    ...steam.map(game => ({ key: JSON.stringify(['steam', game.libraryPath, game.appId]), kind: 'steam' as const, game })),
    ...(windows?.games ?? []).map(game => ({ key: JSON.stringify(['windows', game.id]), kind: 'windows' as const, game })),
  ];
  // A removed or refreshed target must not silently redirect a repair to another game.
  const selected = choices.find(game => game.key === selectedKey);
  const changeCategory = (next: Category) => { setCategory(next); setMonitor(false); setPanel(next === 'backups' ? 'repair' : null); };
  const selectGame = (key: string) => { setSelectedKey(key || null); setMonitor(false); setPanel(null); };
  const show = (next: Panel) => { setPanel(next); setMonitor(false); };
  const refresh = () => { onRefresh(); setRevision(value => value + 1); };
  const steamGame = selected?.kind === 'steam' ? selected.game : null;
  const windowsGame = selected?.kind === 'windows' ? selected.game : null;
  const windowsRuntime = windowsGame ? windows?.runtimes.find(runtime => runtime.id === windowsGame.runtimeId) : undefined;
  const usedRuntime = steamGame?.configuredTool ?? windowsRuntime?.name;
  const diagnosis = steamGame ? diagnostics.find(result => result.appId === steamGame.appId && result.libraryPath === steamGame.libraryPath) ?? null : null;
  const openSelectedWindows = () => { if (windowsGame) onOpenWindows(windowsGame); };
  const tools = [
    { name: 'Repair Game', copy: 'Review supported, recoverable fixes', Icon: Wrench, active: !!steamGame, click: () => show('fix') },
    { name: 'Optimize Game', copy: windowsGame ? 'Review launch options' : 'Review per-game Proton settings', Icon: Sparkles, active: !!selected, click: windowsGame ? openSelectedWindows : () => show('proton') },
    { name: 'Verify Files', copy: steamGame ? 'See Steam verification steps' : 'Steam games only', Icon: RefreshCw, active: !!steamGame, click: () => show('verify') },
    { name: 'Clean Prefix', copy: 'Review cache and prefix options', Icon: Trash2, active: !!steamGame, click: () => show('repair') },
    { name: 'Configure Proton', copy: 'Set Proton and launch options', Icon: Wine, active: !!selected, click: windowsGame ? openSelectedWindows : () => show('proton') },
    { name: 'Create Shortcut', copy: windowsGame ? 'Open the game’s shortcut control' : 'Windows games only', Icon: ExternalLink, active: !!windowsGame?.gameExe, click: openSelectedWindows },
  ].filter(tool => `${tool.name} ${tool.copy}`.toLowerCase().includes(query.trim().toLowerCase()));
  const selectProtonManager = () => changeCategory('proton');

  return <div className="gw-tools" aria-label="Gameweave Tools">
    <div className="gw-tools-main">
      <header className="gw-tools-header"><div><h1><Wrench size={34}/>Tools</h1><p>Manage Proton, diagnose issues, repair games, and more.</p></div>
        <div className="gw-tools-header-controls">
          <label className="gw-tools-search"><Search size={18}/><input aria-label="Search tools" placeholder="Search tools…" value={query} onChange={event => { setQuery(event.target.value); changeCategory('game'); }}/></label>
          <div className="gw-tools-view" aria-label="Tool layout"><button type="button" className="gw-tools-button" aria-label="Grid view" aria-pressed={view === 'grid'} onClick={() => setView('grid')}><LayoutGrid size={19}/></button><button type="button" className="gw-tools-button" aria-label="List view" aria-pressed={view === 'list'} onClick={() => setView('list')}><List size={19}/></button></div>
        </div>
      </header>
      <nav aria-label="Tools categories" className="gw-tools-tabs">{categories.map(({ id, label, Icon }) => <button type="button" className="gw-tools-button" aria-pressed={category === id} key={id} onClick={() => changeCategory(id)}><Icon size={19}/>{label}</button>)}</nav>
      {category === 'game' && <>
        <section className="gw-tools-surface gw-tools-actions" data-view={view} aria-label="Game tools">
          {tools.map(({ name, copy, Icon, active, click }) => <button type="button" className="gw-tools-tile" key={name} onClick={click} disabled={!active} title={active ? undefined : selected ? copy : 'Choose a game first'}><Icon size={36}/><span><strong>{name}</strong><small>{copy}</small></span></button>)}
          {tools.length === 0 && <p role="status">No tools match your search.</p>}
        </section>
        <section className="gw-tools-surface gw-tools-inventory" aria-labelledby={`${prefix}-runtimes`}>
          <div className="gw-tools-inventory-title"><div><h2 id={`${prefix}-runtimes`}><Layers size={31}/>Proton Manager</h2><p>Install, update, and manage Proton versions.</p></div>
            <div><button type="button" className="gw-tools-button primary" onClick={selectProtonManager}><Download size={19}/>Install Proton</button><button type="button" className="gw-tools-button" onClick={refresh} disabled={scanning || windowsLoading}><RefreshCw size={17}/>Refresh</button></div>
          </div>
          <div className="gw-tools-table-wrap"><table><thead><tr><th>Version</th><th>Status</th><th>Source</th><th>Actions</th></tr></thead><tbody>
            {runtimes.map(tool => <tr key={tool.path}><th scope="row" title={tool.path}>{tool.name}</th><td><span className="gw-tools-installed"><CircleCheck size={14}/>Detected</span></td><td>{tool.source}</td><td>{selected ? <button type="button" className="gw-tools-button" aria-label={`Configure Proton for ${selected.game.name} (inventory entry ${tool.name})`} onClick={windowsGame ? openSelectedWindows : () => show('proton')}>Configure</button> : <span className="gw-tools-muted">Choose a game</span>}</td></tr>)}
            {runtimes.length === 0 && <tr><td colSpan={4}>{scanning ? 'Checking installed Proton versions…' : 'No Proton versions detected in the current Steam scan.'}</td></tr>}
          </tbody></table></div>
        </section>
      </>}
      {category === 'proton' && <div className="gw-tools-surface gw-tools-work"><ProtonManager onChanged={onRefresh}/></div>}
      {category === 'diagnostics' && <GlassCard title="Diagnostics" icon={<Activity size={22}/>}><p>Choose a game to see its existing runtime logs and findings. A scan does not modify game files.</p></GlassCard>}
      {category === 'backups' && <GlassCard title="Backups" icon={<Database size={22}/>}><p>{steamGame ? 'Review the recoverable actions and saved backups below.' : 'Select a Steam game to review its saved repair backups.'}</p></GlassCard>}
      {category === 'system' && <GlassCard title="System Check" icon={<Settings size={22}/>} action={<button type="button" className="gw-tools-button primary" onClick={onOpenSystem}>Open System Check</button>}><p>Inspect GPU, Vulkan and runtime tools on this computer.</p></GlassCard>}
      {category === 'processes' && <GlassCard title="Processes" icon={<BarChart3 size={22}/>}><p>Select a Windows game to view process and resource monitoring. Live readings are available while the game or installer is running.</p></GlassCard>}
      {steamGame && panel && <section className="gw-tools-surface gw-tools-work" aria-label="Selected game action" key={selected?.key} tabIndex={-1} ref={actionPane}>
        {panel === 'fix' && <FixAndTest game={steamGame}/>}
        {panel === 'repair' && <RepairTools game={steamGame} onChanged={onRefresh}/>}
        {panel === 'proton' && <GameProtonPicker game={steamGame} onChanged={onRefresh} onManage={selectProtonManager}/>}
        {panel === 'verify' && <GlassCard title="Verify Files in Steam" icon={<RefreshCw size={22}/>}><p>In Steam, open Library → {steamGame.name} → Properties → Installed Files → Verify integrity of game files.</p></GlassCard>}
        {<button type="button" className="gw-tools-button" onClick={() => setPanel(null)}>Close action</button>}
      </section>}
    </div>
    <aside className="gw-tools-inspector" aria-label="Selected game">
      {selected ? <div className="gw-tools-art"><GameCover source={selected.kind === 'steam' ? { source: 'steam', id: selected.game.appId, libraryPath: selected.game.libraryPath } : { source: 'windows', id: selected.game.id, libraryPath: null }} name={selected.game.name} onOpen={() => {}} readOnly/></div> : <div className="gw-tools-art gw-tools-empty-art"><Gamepad2 size={50}/><span>Game tools</span></div>}
      <div className="gw-tools-inspector-body">
        <div className="gw-tools-game-heading"><h2>{selected?.game.name ?? 'Choose a game'}</h2>{selected && <p>{windowsGame ? 'Windows game' : 'Steam game'}{usedRuntime ? ` · ${usedRuntime}` : ''}</p>}</div>
        <label className="gw-tools-game-picker">Game <ChevronDown size={16}/><select aria-label="Game for tools" value={selected?.key ?? ''} onChange={event => selectGame(event.target.value)}>
          <option value="">{windowsLoading && !choices.length ? 'Loading games…' : 'Choose a game…'}</option>{choices.map(choice => <option key={choice.key} value={choice.key}>{choice.game.name} ({choice.kind === 'windows' ? 'Windows' : 'Steam'})</option>)}
        </select></label>
        {windowsError && <p className="gw-tools-inline-error" role="alert">{windowsError} <button type="button" className="gw-tools-button" onClick={() => setRevision(n => n + 1)} disabled={windowsLoading}>Retry</button></p>}
        {!selected && <p className="gw-tools-muted">Select an installed game to see its cover, diagnostics and maintenance options. <button type="button" className="gw-tools-button" onClick={onOpenLibrary}>Open Library</button></p>}
        {windowsGame ? <WindowsScan key={JSON.stringify([windowsGame.id, windowsGame.logDirectory])} game={windowsGame}/> :
          <GlassCard title="Game Diagnostics" icon={<Activity size={22}/>} action={<button type="button" className="gw-tools-button primary" onClick={onScan} disabled={!steamGame || scanning}>{scanning ? 'Checking…' : 'Run scan'}</button>}>
            {steamGame ? <><ul className="gw-tools-findings"><li>Game folder: {steamGame.installPathExists ? 'Found' : 'Not found'}</li><li>Proton prefix: {steamGame.prefixExists ? 'Found' : 'Not found'}</li><li>Game file integrity: Not verified here</li></ul>{diagnosis && <details className="gw-tools-log-detail"><summary>Log analysis</summary><GameDiagnosis result={diagnosis}/></details>}</> : <p>No game selected.</p>}
          </GlassCard>}
        <GlassCard title="Repair & Maintenance" icon={<Wrench size={22}/>}>
          <div className="gw-tools-card-actions"><button type="button" className="gw-tools-button primary" disabled={!steamGame} onClick={() => show('fix')}><Wrench size={17}/>Review repair</button><button type="button" className="gw-tools-button" disabled={!steamGame} onClick={() => show('repair')}><Trash2 size={17}/>Cache & prefix</button></div>
          {windowsGame && <p>These recovery tools are for Steam games. <button type="button" className="gw-tools-button" onClick={openSelectedWindows}>Open Windows game settings</button></p>}
        </GlassCard>
        <GlassCard title="Backup & Rollback" icon={<Database size={22}/>}>
          <div className="gw-tools-card-actions"><button type="button" className="gw-tools-button" disabled={!steamGame} onClick={() => show('repair')}><Layers size={17}/>Review backups</button><button type="button" className="gw-tools-button" disabled={!steamGame} onClick={() => show('repair')}><RotateCcw size={17}/>Restore options</button></div>
        </GlassCard>
        {category === 'processes' && windowsGame && <GlassCard title="Game Processes" icon={<BarChart3 size={22}/>}><ResourceMonitor key={windowsGame.id} game={windowsGame} enabled={monitor} onToggle={setMonitor}/><button type="button" className="gw-tools-button" onClick={refresh} disabled={scanning || windowsLoading}>Refresh game state</button></GlassCard>}
        <GlassCard title="Game Information" icon={<Info size={22}/>}>
          {selected ? <dl className="gw-tools-paths"><dt>Install path</dt><dd>{steamGame?.installPath ?? windowsGame?.directory}</dd><dt>Prefix path</dt><dd>{steamGame?.prefixPath ?? windowsGame?.prefixDirectory ?? 'Not available'}</dd></dl> : <p>Game details appear here after selection.</p>}
        </GlassCard>
      </div>
    </aside>
  </div>;
}
