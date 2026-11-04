import { useEffect, useId, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { Download, Info, RefreshCw, X } from 'lucide-react';
import { version as appVersion } from '../package.json';

type ActiveSession = { id: string; runId: string; name: string; kind: string; canStop: boolean };

/** Kept mounted across navigation so an active download cannot be started twice. */
export function AppUpdater() {
  const panelId = useId();
  const [expanded, setExpanded] = useState(false);
  const candidate = useRef<Update | null>(null);
  const locked = useRef(false);
  const [busy, setBusy] = useState(false);
  const [version, setVersion] = useState('');
  const [notes, setNotes] = useState('');
  const [message, setMessage] = useState('Check GitHub for the latest stable release.');
  const [error, setError] = useState('');
  const [technicalError, setTechnicalError] = useState('');
  const [installed, setInstalled] = useState(false);
  const [approved, setApproved] = useState(false);
  const [sessions, setSessions] = useState<ActiveSession[] | null>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    if (sessions && !dialog.current?.open) dialog.current?.showModal();
    if (!sessions && dialog.current?.open) dialog.current.close();
  }, [sessions]);
  const keepRunning = () => { setSessions(null); setApproved(false); setMessage('Update cancelled. Your games and installers are still running.'); };


  const checkNow = async () => {
    if (locked.current || installed || sessions) return;
    setExpanded(true);
    locked.current = true; setBusy(true); setError(''); setTechnicalError(''); setApproved(false);
    setVersion(''); setNotes(''); setMessage('Checking GitHub…');
    try {
      await candidate.current?.close(); candidate.current = null;
      const update = await check({ timeout: 20000 });
      candidate.current = update;
      if (update) {
        setVersion(update.version); setNotes(update.body ?? '');
        setMessage(`Version ${update.version} is available (installed: ${update.currentVersion}).`);
      } else setMessage('You are up to date.');
    } catch (e) {
      setMessage('');
      setError('Could not check for updates. Please try again later.');
      setTechnicalError(String(e));
    } finally { locked.current = false; setBusy(false); }
  };

  const install = async (stopSessions?: ActiveSession[]) => {
    const update = candidate.current;
    if (!update || !approved || locked.current || installed) return;
    locked.current = true; setBusy(true); setError(''); setTechnicalError('');
    setMessage('Preparing update…');
    let prepared = false; let completed = false;
    let total = 0; let received = 0;
    try {
      if (stopSessions) {
        setSessions(null); setMessage('Waiting for games and installers to close…');
        await invoke('stop_update_sessions', { sessions: stopSessions, confirmed: true });
      } else {
        const active = await invoke<ActiveSession[]>('active_update_sessions');
        if (active.length) { setSessions(active); setMessage(''); return; }
      }
      await invoke('begin_app_update'); prepared = true;
      setMessage('Downloading update…');
      await update.downloadAndInstall(event => {
        if (event.event === 'Started') total = event.data.contentLength ?? 0;
        if (event.event === 'Progress') {
          received += event.data.chunkLength;
          setMessage(total > 0 ? `Downloading update… ${Math.min(100, Math.floor(received / total * 100))}%` : `Downloading update… ${(received / 1048576).toFixed(1)} MB`);
        }
        if (event.event === 'Finished') setMessage('Verifying and installing… Approve the system prompt if asked.');
      }, { timeout: 300000 });
      setInstalled(true); setVersion(''); setApproved(false);
      completed = true;
      setMessage('Update installed. Reopening Proton Medic…');
      await invoke('restart_after_update', { version: update.version });
    } catch (e) {
      setApproved(false);
      if (completed) {
        setMessage('Update installed.');
        setError('The update installed, but Proton Medic could not open the new version. Open Proton Medic from your applications menu.');
      } else {
        setMessage(''); setError(prepared ? 'Update did not complete. You can try again.' : String(e));
        if (prepared) { try { await invoke('cancel_app_update'); } catch { setError('Update stopped. Close and reopen Proton Medic before continuing.'); } }
      }
      setTechnicalError(String(e));
    } finally { locked.current = false; setBusy(false); }
  };

  return <header className="pm-app-header">
    <dialog ref={dialog} className="pm-update-dialog" aria-labelledby={`${panelId}-stop-title`} aria-describedby={`${panelId}-stop-description`} onCancel={event => { event.preventDefault(); keepRunning(); }}>
      <h2 id={`${panelId}-stop-title`}>Close active sessions to update?</h2>
      <p id={`${panelId}-stop-description`}>These games or installers are using Proton Medic. Stopping them can lose unsaved progress or leave an installation incomplete.</p>
      <ul>{sessions?.map(session => <li key={`${session.id}:${session.runId}`}><strong>{session.name}</strong> — {session.kind === 'installer' ? 'Installer' : 'Game'}</li>)}</ul>
      {sessions?.some(session => !session.canStop) && <p>Some sessions were started by an older version. Close them manually, then retry the update.</p>}
      <div className="pm-update-dialog-actions">
        <button type="button" className="pm-button pm-button-secondary" autoFocus onClick={keepRunning}>Keep running</button>
        <button type="button" className="pm-button pm-button-primary" disabled={!sessions?.length || sessions.some(session => !session.canStop)} onClick={() => { if (sessions) void install(sessions); }}>Stop and update</button>
      </div>
    </dialog>
    <div className="pm-update-controls">
      <button type="button" className="pm-button pm-button-secondary pm-update-info" aria-label="About app updates" aria-expanded={expanded} aria-controls={panelId} onClick={() => setExpanded(value => !value)}><Info size={17}/></button>
      <div className="pm-update-actions">
        <button type="button" className="pm-button pm-button-secondary" disabled={busy || installed || !!sessions} aria-busy={busy} aria-controls={panelId} onClick={() => void checkNow()}><RefreshCw size={16} className={busy ? 'pm-spin' : ''}/>Check for updates</button>
        <span className="pm-header-version">Version {appVersion}</span>
      </div>
    </div>
    <section id={panelId} hidden={!expanded} className="pm-section pm-updater" aria-label="App updates">
    <div className="pm-section-heading">
      <div className="pm-update-heading"><span className="pm-update-icon" aria-hidden="true"><Download/></span><div><h2>App updates</h2><p>Keep Proton Medic up to date.</p></div></div>
      <button type="button" className="pm-button pm-button-secondary pm-update-info" aria-label="Close update details" onClick={() => setExpanded(false)}><X size={17}/></button>
    </div>
    {message && message !== 'Check GitHub for the latest stable release.' && <div role="status" className="pm-update-status"><p>{message}</p></div>}
    {error && <div role="alert" className="pm-update-status"><p>{error}</p><details className="pm-update-details"><summary>Technical details</summary><pre>{technicalError}</pre></details></div>}
    {version && <>
      {notes && <details className="pm-update-details"><summary>What changed</summary><pre style={{whiteSpace:'pre-wrap'}}>{notes}</pre></details>}
      <label className="pm-launch-consent"><input type="checkbox" checked={approved} disabled={busy} onChange={e => setApproved(e.target.checked)}/><span>I approve installing the update and automatically reopening Proton Medic.</span></label>
      <button className="pm-button pm-button-primary" disabled={!approved || busy || !!sessions} onClick={() => void install()}><Download size={17}/>Download &amp; install {version}</button>
    </>}
    <p className="pm-update-footer">Signed updates from GitHub. You choose when to install.</p>
    <details className="pm-update-details"><summary>How updates work</summary><p>Checks contact GitHub only when requested. Downloads are signature-verified before installation. System packages may require administrator approval. After installation, Proton Medic reopens automatically to load the new version.</p></details>
    </section>
  </header>;
}
