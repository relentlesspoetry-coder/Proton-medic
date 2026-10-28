import { useId, useRef, useState } from 'react';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { Download, Info, RefreshCw, X } from 'lucide-react';
import { version as appVersion } from '../package.json';

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

  const checkNow = async () => {
    if (locked.current || installed) return;
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

  const install = async () => {
    const update = candidate.current;
    if (!update || !approved || locked.current || installed) return;
    locked.current = true; setBusy(true); setError(''); setTechnicalError('');
    setMessage('Downloading update…');
    let total = 0; let received = 0;
    try {
      await update.downloadAndInstall(event => {
        if (event.event === 'Started') total = event.data.contentLength ?? 0;
        if (event.event === 'Progress') {
          received += event.data.chunkLength;
          setMessage(total > 0 ? `Downloading update… ${Math.min(100, Math.floor(received / total * 100))}%` : `Downloading update… ${(received / 1048576).toFixed(1)} MB`);
        }
        if (event.event === 'Finished') setMessage('Verifying and installing… Approve the system prompt if asked.');
      }, { timeout: 300000 });
      setInstalled(true); setVersion(''); setApproved(false);
      setMessage('Update installed. Finish any current work, then close and reopen Proton Medic.');
    } catch (e) {
      setMessage(''); setApproved(false);
      setError('Update did not complete. You can try again.');
      setTechnicalError(String(e));
    } finally { locked.current = false; setBusy(false); }
  };

  return <header className="pm-app-header">
    <div className="pm-update-controls">
      <button type="button" className="pm-button pm-button-secondary pm-update-info" aria-label="About app updates" aria-expanded={expanded} aria-controls={panelId} onClick={() => setExpanded(value => !value)}><Info size={17}/></button>
      <div className="pm-update-actions">
        <button type="button" className="pm-button pm-button-secondary" disabled={busy || installed} aria-busy={busy} aria-controls={panelId} onClick={() => void checkNow()}><RefreshCw size={16} className={busy ? 'pm-spin' : ''}/>Check for updates</button>
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
      <label className="pm-launch-consent"><input type="checkbox" checked={approved} disabled={busy} onChange={e => setApproved(e.target.checked)}/><span>I have finished game installations and repairs and approve updating Proton Medic.</span></label>
      <button className="pm-button pm-button-primary" disabled={!approved || busy} onClick={() => void install()}><Download size={17}/>Download &amp; install {version}</button>
    </>}
    <p className="pm-update-footer">Signed updates from GitHub. You choose when to install.</p>
    <details className="pm-update-details"><summary>How updates work</summary><p>Checks contact GitHub only when requested. Downloads are signature-verified before installation. System packages may require administrator approval.</p></details>
    </section>
  </header>;
}
