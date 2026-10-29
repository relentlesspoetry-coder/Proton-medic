import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ChevronDown, Download, RefreshCw } from 'lucide-react';
import type { CompatibilityTool, SteamInstallation } from './types';
interface Release { tag: string; name: string; size: number }
interface Progress { phase: string; tag: string; downloaded: number; total: number; message: string }
interface Local { installations: SteamInstallation[]; tools: CompatibilityTool[]; status: Progress | null; umuAvailable: boolean }
const active = (status?: Progress | null) => !!status && ['preparing', 'downloading', 'extracting'].includes(status.phase);
const size = (bytes: number) => `${(bytes / 1024 / 1024).toFixed(0)} MiB`;
export function ProtonManager({ onChanged }: { onChanged: () => void }) {
  const [local, setLocal] = useState<Local | null>(null);
  const [root, setRoot] = useState('');
  const rootRef = useRef('');
  const refresh = useRef<() => void>(() => {});
  const chooseRoot = (value: string) => { rootRef.current = value; setRoot(value); setApproved(false); };
  const [releases, setReleases] = useState<Release[]>([]);
  const [page, setPage] = useState(1);
  const [tag, setTag] = useState('');
  const [approved, setApproved] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [readError, setReadError] = useState('');
  const [canceling, setCanceling] = useState(false);
  const alive = useRef(true); const locked = useRef(false); const changed = useRef(onChanged); changed.current = onChanged;
  useEffect(() => {
    alive.current = true;
    let stopped = false; let pending = false; let inFlight = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let previous: string | undefined; let downloading = false;
    const poll = async () => {
      clearTimeout(timer);
      if (stopped || document.hidden) return;
      if (inFlight) { pending = true; return; }
      inFlight = true;
      try {
        const next = await invoke<Local>('list_proton_versions'); if (stopped) return;
        setLocal(next); setReadError('');
        const destination = next.installations.some(i => i.path === rootRef.current) ? rootRef.current : next.installations[0]?.path ?? '';
        if (destination !== rootRef.current) chooseRoot(destination);
        const result = next.status?.tag + ':' + next.status?.phase;
        if (next.status?.phase === 'installed' && result !== previous) changed.current();
        previous = result; downloading = active(next.status);
        if (!downloading) setCanceling(false);
      } catch (e) { if (!stopped) setReadError(`Could not refresh Proton versions: ${String(e)}`); }
      finally {
        inFlight = false;
        if (!stopped && !document.hidden) timer = setTimeout(() => void poll(), pending ? 0 : downloading ? 1000 : 30000);
        pending = false;
      }
    };
    const visibility = () => { clearTimeout(timer); if (!document.hidden) void poll(); };
    refresh.current = () => void poll();
    document.addEventListener('visibilitychange', visibility);
    void poll();
    return () => { stopped = true; alive.current = false; clearTimeout(timer); document.removeEventListener('visibilitychange', visibility); };
  }, []);
  const run = async (task: () => Promise<void>) => {
    if (locked.current) return; locked.current = true; setBusy(true); setError('');
    try { await task(); } catch (e) { if (alive.current) setError(String(e)); }
    finally { locked.current = false; if (alive.current) setBusy(false); }
  };
  const fetchPage = (nextPage: number) => run(async () => {
    const next = await invoke<Release[]>('fetch_proton_releases', { page: nextPage }); if (!alive.current) return;
    setReleases(next); setPage(nextPage); setTag(next[0]?.tag ?? ''); setApproved(false);
  });
  const selected = releases.find(r => r.tag === tag);
  const installing = active(local?.status);
  const installed = !!selected && !!local?.tools.some(t => t.path === `${root}/compatibilitytools.d/${selected.name.replace(/\.tar\.gz$/, '')}`);
  return <>
    <div className="pm-page-title"><div><h1>Proton versions</h1><p>Manage your game runtimes.</p></div></div>
    <section className="pm-section" aria-label="Download Proton"><div className="pm-section-heading"><div><h2>Download GE-Proton</h2><p>Official GloriousEggroll releases.</p></div><Download size={22}/></div>
      <div className="pm-form-grid"><label>Install for<select value={root} disabled={busy || installing || !local?.installations.length} onChange={e => { chooseRoot(e.target.value); }}><option value="" disabled>Choose Steam installation</option>{local?.installations.map(i => <option key={i.path} value={i.path}>{i.kind} Steam · {i.path}</option>)}</select></label>
      <label>Available version<select value={tag} disabled={busy || installing || !releases.length} onChange={e => { setTag(e.target.value); setApproved(false); }}><option value="" disabled>Load releases to choose a version</option>{releases.map(r => <option key={r.tag} value={r.tag}>{r.tag} · {size(r.size)}</option>)}</select></label></div>
      <div className="pm-button-row"><button className="pm-button pm-button-secondary" disabled={busy || installing} onClick={() => void fetchPage(1)}><RefreshCw size={16}/>{busy ? 'Working…' : 'Load latest releases'}</button>
      {page > 1 && <button className="pm-button pm-button-secondary" disabled={busy || installing} onClick={() => void fetchPage(page - 1)}>Newer releases</button>}
      {!!releases.length && page < 10 && <button className="pm-button pm-button-secondary" disabled={busy || installing} onClick={() => void fetchPage(page + 1)}>Older releases</button>}</div>
      {local && !local.installations.length && <p className="pm-repair-danger">Open Steam once to add an installation destination.</p>}
      {selected && <div className="pm-proton-confirm"><p>{size(selected.size)} download · Extra space needed for extraction.</p>
        <label className="pm-report-approval"><input type="checkbox" checked={approved} disabled={busy || installing || installed} onChange={e => setApproved(e.target.checked)}/>I approve downloading and installing this GE-Proton release.</label>
        <button className="pm-button pm-button-primary" disabled={busy || installing || installed || !root || !approved} onClick={() => void run(async () => {
          await invoke('install_proton_version', { tag, root, confirmed: true }); if (!alive.current) return;
          setLocal(old => old && ({ ...old, status: { phase: 'preparing', tag, downloaded: 0, total: selected.size, message: 'Checking release metadata…' } })); setApproved(false); refresh.current();
        })}>{installed ? 'Already installed' : 'Download and install'}</button>
      </div>}
      {local?.status && <div className="pm-proton-progress" role="status"><strong>{local.status.tag}</strong><p>{local.status.message}</p>
        {local.status.phase === 'downloading' && <><progress aria-label="Proton download progress" value={local.status.downloaded} max={local.status.total || 1}/><p>{size(local.status.downloaded)} / {size(local.status.total)}</p></>}
        {installing && <button className="pm-button pm-button-secondary" disabled={canceling || busy} onClick={() => void run(async () => { await invoke('cancel_proton_download'); if (alive.current) setCanceling(true); })}>{canceling ? 'Cancel requested…' : 'Cancel download'}</button>}
      </div>}
      {error && <p className="pm-repair-danger" role="alert">{error}</p>}{readError && <p className="pm-repair-danger" role="alert">{readError}</p>}
      {installing && <p className="pm-report-help">Keep Proton Medic open until installation finishes.</p>}
      {local?.status?.phase === 'installed' && <p className="pm-report-help">Restart Steam to use the new version.</p>}
    </section>
    <section className="pm-section"><h2>Installed versions{local ? ` (${local.tools.length})` : ''}</h2>
      {!local ? <p role="status">{readError ? 'Proton inventory unavailable.' : 'Loading installed versions…'}</p> : local.tools.length ? local.tools.map(t => <div className="pm-discovery" key={t.path}><div><strong>{t.name}</strong><small>{t.source} · {t.path}</small></div></div>) : <p>No Proton versions installed.</p>}
      {readError && <button className="pm-button pm-button-secondary" onClick={() => refresh.current()}>Retry inventory</button>}
    </section>
    <details className="pm-section pm-technical"><summary>Help <ChevronDown size={17}/></summary>
      <p>Steam games: <strong>My Games → game → Proton</strong>. Close Steam before saving. Changes are backed up.</p>
      <p>Windows games: <strong>Game settings and logs → Edit launch settings</strong>. Downloaded GE-Proton requires <strong>umu-launcher</strong> (umu-run on PATH). Install it through your distribution.</p>
      {local && <p>{local.umuAvailable ? 'UMU is available.' : 'UMU is not detected.'}</p>}
      <p>Downloads use the maintainer’s SHA-512 checksum. Versions install side by side without changing game settings. Restart Steam after installation.</p>
      <p>Flatpak Steam downloads work with Flatpak Steam only. Valve Proton and Proton Experimental are managed through <strong>Steam → Library → Tools</strong>.</p>
      <p>Release checks contact GitHub only when requested. A newer Proton version may not work better for every game.</p>
    </details>
  </>;
}
