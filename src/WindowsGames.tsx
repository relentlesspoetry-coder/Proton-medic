import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { Download, Monitor, Play, Plus, RefreshCw, Search, Pencil, Trash2 } from 'lucide-react';
import type { Finding } from './types';
import { PagedList } from './PagedList';
import { InstallationAssistant } from './InstallationAssistant';
import { WindowsLaunchSettings } from './WindowsLaunchSettings';
import { GameIcon } from './GameIcon';
interface LogReport { findings: Finding[]; logs: { name: string; status: string; truncated: boolean }[] }
export interface Runtime { id: string; name: string; kind: string }
export interface Game { id: string; name: string; runtimeId: string; gameExe: string | null; state: string; exitCode: number | null; message: string; directory: string; windowsDrive: string; prefixDirectory: string; logDirectory: string | null; launchArguments?: string[]; workingDirectory?: string | null }
interface Library { runtimes: Runtime[]; games: Game[]; removedGames?: Game[]; warnings: string[] }
interface ExecutableSearch { executables: string[]; truncated: boolean }
const empty: Library = { runtimes: [], games: [], warnings: [] };
const isRunning = (game: Game) => game.state.endsWith('running') || game.state === 'session-ended';
const STATUS_LABELS: Record<string, string> = { 'installer-running': 'Installing', 'game-running': 'Running', 'installer-exited': 'Choose game executable', 'ready-to-test': 'Ready to launch', 'game-exited': 'Stopped', 'process-failed': 'Process failed', 'start-failed': 'Could not start', 'session-ended': 'Previous run needs attention', interrupted: 'Previous run interrupted' };
const statusLabel = (game: Game) => STATUS_LABELS[game.state] ?? 'Setup needed';

export function WindowsGames() {
  const [library, setLibrary] = useState<Library>(empty);
  const [loaded, setLoaded] = useState(false);
  const [mode, setMode] = useState<'install' | 'existing'>('install');
  const [name, setName] = useState('');
  const [installer, setInstaller] = useState('');
  const [prefix, setPrefix] = useState('');
  const [runtime, setRuntime] = useState('');
  const [approved, setApproved] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const sequence = useRef(0);
  const taskLocked = useRef(false);
  const refresh = useCallback(async () => {
    const request = ++sequence.current;
    const next = await invoke<Library>('list_windows_games');
    if (request !== sequence.current) return;
    setLibrary(old => JSON.stringify(old) === JSON.stringify(next) ? old : next); setLoaded(true);
    setRuntime(current => next.runtimes.some(r => r.id === current) ? current : next.runtimes[0]?.id ?? '');
  }, []);
  useEffect(() => {
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try { await refresh(); } catch (e) { if (!stopped) setError(`Could not read Windows games: ${String(e)}`); }
      finally { if (!stopped) timer = setTimeout(() => void poll(), 5000); }
    };
    void poll();
    return () => { stopped = true; ++sequence.current; clearTimeout(timer); };
  }, [refresh]);
  const run = async (task: () => Promise<void>) => {
    if (taskLocked.current) return;
    taskLocked.current = true; setBusy(true); setError(''); setNotice('');
    try { await task(); } catch (e) { setError(String(e)); } finally { taskLocked.current = false; setBusy(false); }
  };
  const remember = (game: Game) => {
    if (game?.id) setLibrary(old => ({ ...old, games: [...old.games.filter(g => g.id !== game.id), game].sort((a, b) => a.name.localeCompare(b.name)) }));
  };
  const pickExecutable = () => run(async () => {
    const file = await open({ multiple: false, directory: false, filters: [{ name: mode === 'install' ? 'Windows installer' : 'Windows game', extensions: ['exe'] }] });
    if (typeof file !== 'string') return;
    setInstaller(file); setName(file.split('/').pop()?.replace(/\.exe$/i, '') ?? 'Windows game'); setApproved(false);
    const drive = file.lastIndexOf('/drive_c/');
    setPrefix(mode === 'existing' && drive >= 0 ? file.slice(0, drive) : '');
  });
  const submit = () => run(async () => {
    if (!approved || !installer || !name.trim() || !runtime || (mode === 'existing' && !prefix)) return;
    const game = mode === 'install'
      ? await invoke<Game>('install_windows_game', { name, installer, runtimeId: runtime, confirmed: true })
      : await invoke<Game>('register_windows_game', { name, path: installer, prefix, runtimeId: runtime, confirmed: true });
    if (game?.id) remember(game);
    setApproved(false); await refresh();
    setNotice(mode === 'install' ? 'Installer started. Complete its setup window, then review the suggested game executable in its card.' : 'Installed game added using its existing prefix.');
  });
  const switchMode = (next: 'install' | 'existing') => { setMode(next); setInstaller(''); setName(''); setPrefix(''); setApproved(false); };

  return <>
    <div className="pm-page-title"><div><h1>Windows Games</h1><p>Launch games with their saved Wine or Proton environment.</p></div><button disabled={busy} className="pm-button pm-button-secondary" onClick={() => void run(refresh)}><RefreshCw size={17}/>Refresh</button></div>
    {error && <p role="alert" className="pm-repair-danger">{error}</p>}
    {notice && <p role="status" className="pm-auto-message">{notice}</p>}
    {library.warnings.map((warning, i) => <p key={i} role="alert" className="pm-repair-danger">{warning}</p>)}
    <section className="pm-section" aria-label="Managed Windows games">
      <div className="pm-section-heading"><div><h2>Your Windows games</h2><p>Choose a game to launch, rename, or manage.</p></div></div>
      {!library.games.length && <p>{loaded ? 'No Windows games have been added yet.' : 'Loading Windows games…'}</p>}
      <PagedList className="pm-windows-list" items={library.games} render={game => <WindowsGameCard key={game.id} game={game} runtimes={library.runtimes} busy={busy} run={run} remember={remember} refresh={refresh} notify={setNotice} forget={id => setLibrary(old => ({ ...old, games: old.games.filter(g => g.id !== id) }))}/>}/>
      {!!library.removedGames?.length && <details className="pm-update-details"><summary>Removed games ({library.removedGames.length})</summary><p>Game files and prefixes are kept. Restore a game to show it in this list again.</p><div className="pm-removed-games">{library.removedGames.map(game => <div className="pm-removed-game" key={game.id}><span>{game.name}</span><button disabled={busy || isRunning(game)} className="pm-button pm-button-secondary" onClick={() => void run(async () => { await invoke('set_windows_game_hidden', { id: game.id, hidden: false, confirmed: true }); await refresh(); })}>Restore {game.name}</button></div>)}</div></details>}

    </section>
    <section className="pm-section" aria-label="Add Windows game">
      <div className="pm-windows-tabs" role="group" aria-label="Add game method">
        <button type="button" aria-pressed={mode === 'install'} className={`pm-button ${mode === 'install' ? 'pm-button-primary' : 'pm-button-secondary'}`} disabled={busy} onClick={() => switchMode('install')}><Download size={17}/>Install a new game</button>
        <button type="button" aria-pressed={mode === 'existing'} className={`pm-button ${mode === 'existing' ? 'pm-button-primary' : 'pm-button-secondary'}`} disabled={busy} onClick={() => switchMode('existing')}><Plus size={17}/>Add installed game</button>
      </div>
      <h2>{mode === 'install' ? 'Install a Windows game' : 'Add an existing installation'}</h2>
      <p>{mode === 'install' ? 'Run a trusted .exe installer in a separate Windows prefix.' : 'Choose the game executable and the prefix used to install it. Game files are kept in place.'}</p>
      <button disabled={busy} className="pm-button pm-button-secondary" onClick={() => void pickExecutable()}>{mode === 'install' ? 'Choose installer (.exe)' : 'Choose game (.exe)'}</button>
      {installer && <p className="pm-selected-path"><code>{installer}</code></p>}
      <div className="pm-form-grid">
        <label>Game name<input aria-label="Game name" value={name} maxLength={120} disabled={busy} onChange={e => { setName(e.target.value); setApproved(false); }}/></label>
        <label>Runtime<select aria-label="Runtime" value={runtime} disabled={busy} onChange={e => { setRuntime(e.target.value); setApproved(false); }}>{library.runtimes.map(r => <option value={r.id} key={r.id}>{r.name} ({r.kind})</option>)}</select></label>
      </div>
      {mode === 'existing' && <div className="pm-prefix-picker"><button className="pm-button pm-button-secondary" disabled={busy} onClick={() => void run(async () => {
        const directory = await open({ multiple: false, directory: true, title: 'Choose the existing Windows prefix containing drive_c and system.reg' });
        if (typeof directory === 'string') { setPrefix(directory); setApproved(false); }
      })}>Choose existing prefix</button><p>{prefix ? <code>{prefix}</code> : 'Choose the folder containing drive_c and system.reg. For Proton, choose compatdata/pfx.'}</p></div>}
      {!library.runtimes.length && loaded && <p role="status">No usable Wine or native Steam Proton runtime found. Install a runtime, then refresh.</p>}
      <label className="pm-launch-consent"><input type="checkbox" checked={approved} disabled={busy} onChange={e => setApproved(e.target.checked)}/><span>{mode === 'install' ? 'I trust this installer and approve running it. Windows programs can access my user files.' : 'I trust this executable and approve using the selected runtime with its existing prefix.'}</span></label>
      <button className="pm-button pm-button-primary" disabled={busy || !approved || !installer || !name.trim() || !runtime || (mode === 'existing' && !prefix)} onClick={() => void submit()}><Download size={17}/>{busy ? 'Working…' : mode === 'install' ? 'Run installer' : 'Add game'}</button>
      <details className="pm-update-details"><summary>{mode === 'install' ? 'Installation help' : 'Which prefix should I use?'}</summary><p>{mode === 'install' ? 'Complete the installer’s setup window and keep Proton Medic open. When it finishes, select the installed game’s .exe in its card above. Files on other drives can be selected. An installer exit code alone does not confirm installation.' : 'Use the same prefix and runtime that previously ran the game. A new empty prefix will not contain its registry settings or installed dependencies. Adding a game saves a launcher profile without copying or reinstalling it.'}</p></details>
    </section>
  </>;
}

function WindowsGameCard({ game, runtimes, busy, run, remember, refresh, notify, forget }: { game: Game; runtimes: Runtime[]; busy: boolean; run: (task: () => Promise<void>) => Promise<void>; remember: (game: Game) => void; refresh: () => Promise<void>; notify: (message: string) => void; forget: (id: string) => void }) {
  const [search, setSearch] = useState<ExecutableSearch | null>(null);
  const [candidate, setCandidate] = useState('');
  const [name, setName] = useState(game.name);
  const [editing, setEditing] = useState(false);
  const [removing, setRemoving] = useState(false);
  useEffect(() => { setName(game.name); }, [game.name]);
  const [logs, setLogs] = useState<LogReport | null>(null);
  const running = isRunning(game);
  const select = async (path: string) => { remember(await invoke<Game>('select_windows_game_executable', { id: game.id, path, confirmed: true })); setSearch(null); await refresh(); };
  return <article className="pm-windows-game">
    <div className="pm-section-heading"><div className="pm-windows-game-title"><GameIcon source={{ kind: 'windows', id: game.id, executable: game.gameExe }}/><div><h3>{game.name}</h3><p role="status">{statusLabel(game)}</p></div></div>{game.gameExe && <button disabled={busy || running} className="pm-button pm-button-primary" onClick={() => void run(async () => { remember(await invoke<Game>('launch_windows_game', { id: game.id, confirmed: true })); await refresh(); })}><Play size={17}/>Launch game</button>}</div>
    <div className="pm-button-row pm-game-management">
      <button type="button" className="pm-button pm-button-secondary" aria-label={`Rename ${game.name}`} disabled={busy || running} onClick={() => { setName(game.name); setEditing(true); setRemoving(false); }}><Pencil size={15}/>Rename</button>
      <button type="button" className="pm-button pm-button-secondary" aria-label={`Remove ${game.name} from list`} disabled={busy || running} onClick={() => { setRemoving(true); setEditing(false); }}><Trash2 size={15}/>Remove from list</button>
    </div>
    {editing && <form className="pm-name-editor" onSubmit={event => { event.preventDefault(); if (name.trim() && name.trim() !== game.name) void run(async () => { remember(await invoke<Game>('rename_windows_game', { id: game.id, name, confirmed: true })); setEditing(false); await refresh(); }); }}>
      <label>Display name<input aria-label={`Display name for ${game.name}`} value={name} maxLength={120} disabled={busy || running} onChange={event => setName(event.target.value)} autoFocus/></label>
      <div className="pm-button-row"><button type="submit" className="pm-button pm-button-primary" disabled={busy || running || !name.trim() || name.trim() === game.name}>Save name</button><button type="button" className="pm-button pm-button-secondary" disabled={busy} onClick={() => setEditing(false)}>Cancel</button></div>
    </form>}
    {removing && <div className="pm-remove-confirm" role="group" aria-label={`Confirm removal of ${game.name}`}><p>Remove <strong>{game.name}</strong> from this list? Game files, prefixes, and desktop shortcuts are kept. You can restore it from Removed games.</p><div className="pm-button-row"><button className="pm-button pm-button-secondary" disabled={busy || running} onClick={() => void run(async () => { await invoke('set_windows_game_hidden', { id: game.id, hidden: true, confirmed: true }); forget(game.id); await refresh(); })}>Confirm removal</button><button className="pm-button pm-button-secondary" disabled={busy} onClick={() => setRemoving(false)}>Cancel</button></div></div>}
    <p>{game.message}</p>
    {game.state==='installer-exited'&&!game.gameExe&&<InstallationAssistant game={game} busy={busy} run={run} onConfirm={async(name,path,shortcut)=>{
      remember(await invoke<Game>('confirm_windows_installation',{id:game.id,name,path,confirmed:true}));await refresh();notify('Game saved. Launch it when you are ready to test.');
      if(shortcut){try{await invoke('create_windows_game_shortcut',{id:game.id,confirmed:true});notify('Game saved. Desktop and applications-menu shortcuts created.');}catch(error){throw new Error('Game saved, but shortcut creation failed: '+String(error)+'. Use Create desktop shortcut to retry.');}}
    }}/>}

    <div className="pm-button-row">
      <button disabled={busy || running} className="pm-button pm-button-secondary" onClick={() => void run(async () => {
        const file = await open({ multiple: false, directory: false, defaultPath: game.windowsDrive, filters: [{ name: 'Installed Windows game', extensions: ['exe'] }] });
        if (typeof file === 'string') await select(file);
      })}>{game.gameExe ? 'Change game .exe' : 'Select installed game .exe'}</button>
      {!game.gameExe && <button disabled={busy || running} className="pm-button pm-button-secondary" onClick={() => void run(async () => { const result = await invoke<ExecutableSearch>('find_windows_game_executables', { id: game.id }); setSearch(result); setCandidate(''); })}><Search size={17}/>Find game executables</button>}
      {game.gameExe && <button disabled={busy} className="pm-button pm-button-secondary" onClick={() => void run(async () => { const paths = await invoke<string[]>('create_windows_game_shortcut', { id: game.id, confirmed: true }); notify(`Shortcuts created in your desktop and applications menu. If prompted, choose Allow Launching. ${paths.join(' · ')}`); })}><Monitor size={17}/>Create desktop shortcut</button>}
    </div>
    {search && <div className="pm-executable-search">
      {search.executables.length ? <><label>Game executable<select aria-label={`Executable for ${game.name}`} value={candidate} onChange={e => setCandidate(e.target.value)}><option value="">Choose a game executable…</option>{search.executables.map(path => <option key={path} value={path}>{path}</option>)}</select></label><button className="pm-button pm-button-primary" disabled={busy || running || !candidate} onClick={() => void run(() => select(candidate))}>Use selected executable</button></> : <p>No game executables found. Use Select installed game .exe to choose a file on another drive.</p>}
      {search.truncated && <p>The search reached its limit. You can choose a file manually.</p>}
    </div>}
    <details className="pm-update-details"><summary>Game settings and logs</summary>
      <WindowsLaunchSettings game={game} runtimes={runtimes} busy={busy} run={run} onSave={async(value,expected)=>{remember(await invoke<Game>('save_windows_launch_settings',{id:game.id,value,expected,confirmed:true}));await refresh();notify('Launch settings saved with a profile backup. Existing shortcuts use the saved settings.');}} onReset={async(expected)=>{remember(await invoke<Game>('reset_windows_launch_settings',{id:game.id,expected,confirmed:true}));await refresh();notify('Original launch settings restored. Previous settings were backed up.');}}/>


      <dl className="pm-windows-paths"><dt>Game executable</dt><dd>{game.gameExe ?? 'Not selected'}</dd><dt>Windows prefix</dt><dd>{game.prefixDirectory ?? game.windowsDrive.replace(/\/drive_c$/, '')}</dd><dt>Last exit code</dt><dd>{game.exitCode ?? 'Not available'}</dd><dt>Logs</dt><dd>{game.logDirectory ?? 'No run yet'}</dd></dl>
      <button disabled={busy || !game.logDirectory} className="pm-button pm-button-secondary" onClick={() => void run(async () => setLogs(await invoke<LogReport>('analyze_windows_game', { id: game.id })))}>Read runtime logs</button>
      {logs && <div>{logs.logs.map(log => <p key={log.name}>{log.name}: {log.status}{log.truncated ? ' · partial log (tail retained)' : ''}</p>)}{logs.findings.length ? logs.findings.map(f => <p key={f.code}><strong>{f.title}</strong> — {f.recommendation}</p>) : <p>No recognized errors in the captured logs.</p>}</div>}
    </details>
  </article>;
}
