import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Download, RefreshCw } from 'lucide-react';
import type { CompatibilityTool, SteamInstallation } from './types';
interface Release { tag: string; name: string; size: number }
interface Progress { phase: string; tag: string; downloaded: number; total: number; message: string }
interface Local { installations: SteamInstallation[]; tools: CompatibilityTool[]; status: Progress | null; umuAvailable: boolean }
const active = (status?: Progress | null) => !!status && ['preparing', 'downloading', 'extracting'].includes(status.phase);
const size = (bytes: number) => `${(bytes / 1024 / 1024).toFixed(0)} MiB`;
export function ProtonManager({ onChanged }: { onChanged: () => void }) {
  const [local, setLocal] = useState<Local | null>(null);
  const [root, setRoot] = useState('');
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
    alive.current = true; let stopped = false; let timer: ReturnType<typeof setTimeout>; let previous: string | undefined;
    const poll = async () => {
      try {
        const next = await invoke<Local>('list_proton_versions'); if (stopped) return;
        setLocal(next); setReadError(''); setRoot(old => next.installations.some(i => i.path === old) ? old : next.installations[0]?.path ?? '');
        const result = next.status?.tag + ':' + next.status?.phase;
        if (next.status?.phase === 'installed' && result !== previous) changed.current();
        previous = result; if (!active(next.status)) setCanceling(false);
      } catch (e) { if (!stopped) setReadError(`Could not refresh Proton versions: ${String(e)}`); }
      finally { if (!stopped) timer = setTimeout(() => void poll(), 1000); }
    };
    void poll(); return () => { stopped = true; alive.current = false; clearTimeout(timer); };
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
    <div className="pm-page-title"><div><h1>Proton versions</h1><p>Install versions side by side and choose one for each game.</p></div></div>
    <section className="pm-section" aria-label="Download Proton"><div className="pm-section-heading"><div><h2>Download GE-Proton</h2><p>Community builds from GloriousEggroll’s official GitHub releases.</p></div><Download size={22}/></div>
      <div className="pm-form-grid"><label>Install for<select value={root} disabled={busy || installing || !local?.installations.length} onChange={e => { setRoot(e.target.value); setApproved(false); }}><option value="" disabled>Choose Steam installation</option>{local?.installations.map(i => <option key={i.path} value={i.path}>{i.kind} Steam · {i.path}</option>)}</select></label>
      <label>Available version<select value={tag} disabled={busy || installing || !releases.length} onChange={e => { setTag(e.target.value); setApproved(false); }}><option value="" disabled>Load releases to choose a version</option>{releases.map(r => <option key={r.tag} value={r.tag}>{r.tag} · {size(r.size)}</option>)}</select></label></div>
      <div className="pm-button-row"><button className="pm-button pm-button-secondary" disabled={busy || installing} onClick={() => void fetchPage(1)}><RefreshCw size={16}/>{busy ? 'Working…' : 'Load latest releases'}</button>
      {page > 1 && <button className="pm-button pm-button-secondary" disabled={busy || installing} onClick={() => void fetchPage(page - 1)}>Newer releases</button>}
      {!!releases.length && page < 10 && <button className="pm-button pm-button-secondary" disabled={busy || installing} onClick={() => void fetchPage(page + 1)}>Older releases</button>}</div>
      {local && !local.installations.length && <p className="pm-repair-danger">Open Steam once, then return here. A detected Steam installation is needed for the download destination.</p>}
      {selected && <div className="pm-proton-confirm"><p>{selected.tag}: {size(selected.size)} download. Allow extra disk space for extraction. Existing versions and game settings stay unchanged.</p>
        <label className="pm-report-approval"><input type="checkbox" checked={approved} disabled={busy || installing || installed} onChange={e => setApproved(e.target.checked)}/>I approve downloading and installing this GE-Proton release.</label>
        <button className="pm-button pm-button-primary" disabled={busy || installing || installed || !root || !approved} onClick={() => void run(async () => {
          await invoke('install_proton_version', { tag, root, confirmed: true }); if (!alive.current) return;
          setLocal(old => old && ({ ...old, status: { phase: 'preparing', tag, downloaded: 0, total: selected.size, message: 'Checking release metadata…' } })); setApproved(false);
        })}>{installed ? 'Already installed' : 'Download and install'}</button>
      </div>}
      {local?.status && <div className="pm-proton-progress" role="status"><strong>{local.status.tag}</strong><p>{local.status.message}</p>
        {local.status.phase === 'downloading' && <><progress aria-label="Proton download progress" value={local.status.downloaded} max={local.status.total || 1}/><p>{size(local.status.downloaded)} / {size(local.status.total)}</p></>}
        {installing && <button className="pm-button pm-button-secondary" disabled={canceling || busy} onClick={() => void run(async () => { await invoke('cancel_proton_download'); if (alive.current) setCanceling(true); })}>{canceling ? 'Cancel requested…' : 'Cancel download'}</button>}
      </div>}
      {error && <p className="pm-repair-danger" role="alert">{error}</p>}{readError && <p className="pm-repair-danger" role="alert">{readError}</p>}
      <p className="pm-report-help">Release checks contact GitHub only when requested. Downloads are verified against the maintainer’s SHA-512 checksum before installation. Keep Proton Medic open until installation finishes. Restart Steam after adding a version.</p>
    </section>
    <section className="pm-section"><h2>Choose a version for a game</h2><p>For Steam games, open <strong>My Games → a game → Proton</strong>. Close Steam before saving the selection; the previous configuration is backed up.</p><p>For Windows games, open <strong>Game settings and logs → Edit launch settings</strong> and choose the runtime. Existing game files and prefixes are kept.</p>
      <p>{local?.umuAvailable ? 'UMU is available for downloaded GE-Proton versions in Windows games.' : 'Downloaded GE-Proton requires umu-launcher (umu-run on PATH) for Windows games outside Steam. Install UMU through your distribution before selecting these versions. '}</p>
      <p className="pm-report-help">Flatpak Steam downloads are available to Flatpak Steam only.</p>
      <p className="pm-report-help">Valve’s official Proton and Proton Experimental are installed and updated through Steam’s Library → Tools. GE-Proton is a separate community build; a newer version is not guaranteed to work better for every game.</p>
    </section>
    <section className="pm-section"><h2>Installed versions ({local?.tools.length ?? 0})</h2>{local?.tools.length ? local.tools.map(t => <div className="pm-discovery" key={t.path}><div><strong>{t.name}</strong><small>{t.source} · {t.path}</small></div></div>) : <p>No Proton versions detected yet.</p>}</section>
  </>;
}
