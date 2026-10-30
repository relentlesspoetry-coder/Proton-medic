import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AlertCircle, Archive, ArrowRight, CheckCircle2, ChevronDown, RotateCcw, ShieldCheck } from 'lucide-react';
import type { Game, RepairAction, RepairBackup, RepairOutcome, RepairPlan } from './types';

type Proposed = { kind: 'apply'; plan: RepairPlan } | { kind: 'restore'; backup: RepairBackup };

export function RepairTools({ game, onChanged }: { game: Game; onChanged: () => void }) {
  const [proposal, setProposal] = useState<Proposed | null>(null);
  const [backups, setBackups] = useState<RepairBackup[]>([]);
  const [busy, setBusy] = useState(false);
  const [loadingBackups, setLoadingBackups] = useState(true);
  const [confirmed, setConfirmed] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [backupError, setBackupError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const refreshBackups = useCallback(async () => {
    setLoadingBackups(true);
    try {
      const found = await invoke<RepairBackup[]>('list_repair_backups', { appId: game.appId, libraryPath: game.libraryPath });
      setBackups(found);
      setBackupError(null);
    } catch (err) {
      setBackupError(`Could not read saved backups: ${String(err)}`);
    } finally { setLoadingBackups(false); }
  }, [game.appId, game.libraryPath]);

  useEffect(() => { void refreshBackups(); }, [refreshBackups]);

  const preview = async (action: RepairAction) => {
    setBusy(true); setError(null); setSuccess(null); setConfirmed(false); setProposal(null);
    try {
      const plan = await invoke<RepairPlan>('preview_repair', { appId: game.appId, libraryPath: game.libraryPath, action });
      setProposal({ kind: 'apply', plan });
    } catch (err) { setError(`Could not prepare the change: ${String(err)}`); }
    finally { setBusy(false); }
  };
  const previewRestore = (backup: RepairBackup) => {
    setError(null); setSuccess(null); setConfirmed(false);
    setProposal({ kind: 'restore', backup });
  };
  const execute = async () => {
    if (!proposal || !confirmed) return;
    setBusy(true); setError(null); setSuccess(null);
    try {
      const args = { appId: game.appId, libraryPath: game.libraryPath, confirmed: true };
      const result = proposal.kind === 'apply'
        ? await invoke<RepairOutcome>('apply_repair', { ...args, action: proposal.plan.action })
        : await invoke<RepairOutcome>('restore_repair', { ...args, action: proposal.backup.action, backupName: proposal.backup.name });
      setSuccess(`${result.message}${result.displacedBackup ? ` The newer directory was saved as ${result.displacedBackup.name}.` : ''}`);
      setProposal(null); setConfirmed(false);
      await refreshBackups();
      onChanged();
    } catch (err) { setError(`Operation failed. Review saved backups before retrying: ${String(err)}`); setConfirmed(false); }
    finally { setBusy(false); }
  };

  return <section className="pm-section pm-repair-section" aria-label="Safe repair tools">
    <div className="pm-section-heading"><div><h2>Safe Repair & Recovery</h2><p>Optional changes. We keep the original data and let you undo them.</p></div><ShieldCheck size={23} className="pm-green"/></div>
    <div className="pm-repair-options">
      <button type="button" disabled={busy} className="pm-repair-choice" onClick={() => void preview('shader-cache')}>
        <Archive size={22}/><span><strong>Refresh shader cache</strong><small>Archive generated graphics files; Steam can rebuild them. May help shader-related issues.</small></span><ArrowRight size={18}/>
      </button>
      <button type="button" disabled={busy} className="pm-repair-choice" onClick={() => void preview('proton-prefix')}>
        <Archive size={22}/><span><strong>Rebuild Proton environment</strong><small>Advanced. Archives Windows compatibility settings, which may contain game saves.</small></span><ArrowRight size={18}/>
      </button>
    </div>
    {proposal && <div className="pm-repair-preview" role="region" aria-label="Confirm repair">
      <div className="pm-repair-preview-heading"><h3>{proposal.kind === 'apply' ? proposal.plan.title : 'Restore saved backup'}</h3><button type="button" className="pm-back" onClick={() => { setProposal(null); setConfirmed(false); }} disabled={busy}>Cancel</button></div>
      <p>{proposal.kind === 'apply' ? proposal.plan.explanation : 'Restore the previous directory from the selected backup. If Steam generated a new directory since then, it will be saved as another backup instead of being deleted.'}</p>
      {proposal.kind === 'apply' ? <>
        <dl className="pm-repair-paths"><dt>Existing data</dt><dd>{proposal.plan.sourcePath}</dd><dt>Backups kept in</dt><dd>{proposal.plan.backupRoot}</dd></dl>
        {proposal.plan.blocker && <div className="pm-repair-blocked" role="alert"><AlertCircle size={17}/>{proposal.plan.blocker}</div>}
      </> : <dl className="pm-repair-paths"><dt>Saved backup</dt><dd>{proposal.backup.backupPath}</dd><dt>Restore location</dt><dd>{proposal.backup.originalPath}</dd><dt>Newer data present?</dt><dd>{proposal.backup.restoredTargetExists ? 'Yes — it will be archived separately' : 'No'}</dd></dl>}
      {(proposal.kind === 'restore' || proposal.plan.risk === 'high') && <p className="pm-repair-danger"><AlertCircle size={18}/> Proton prefixes may contain local saves, configuration and other important data. Close Steam, protect unsynced saves, and review these paths before continuing.</p>}
      <label className="pm-repair-consent"><input type="checkbox" checked={confirmed} disabled={busy || (proposal.kind === 'apply' && !proposal.plan.canApply)} onChange={e => setConfirmed(e.target.checked)}/><span>I have closed Steam and understand the change and backup location.</span></label>
      <button type="button" className="pm-button pm-button-primary" disabled={busy || !confirmed || (proposal.kind === 'apply' && !proposal.plan.canApply)} onClick={() => void execute()}>{busy ? 'Working...' : proposal.kind === 'apply' ? 'Archive & continue' : 'Restore this backup'}</button>
      <p className="pm-repair-footnote">No deletion or privileged command is performed. This operation is not guaranteed to fix a game.</p>
    </div>}
    {error && <div role="alert" className="pm-repair-blocked"><AlertCircle size={17}/>{error}</div>}
    {backupError && <div role="alert" className="pm-repair-blocked"><AlertCircle size={17}/>{backupError}</div>}
    {success && <div role="status" className="pm-repair-success"><CheckCircle2 size={18}/>{success}</div>}
    <details className="pm-backup-details" open={undefined}>
      <summary><RotateCcw size={16}/> Saved backups ({backups.length}) <ChevronDown size={16}/></summary>
      <button type="button" className="pm-subtle-button pm-backup-refresh" onClick={() => void refreshBackups()} disabled={busy || loadingBackups}>Refresh backups</button>
      {loadingBackups ? <p>Loading available backups...</p> : backups.length === 0 ? <p>No backups have been created for this game.</p> : <div className="pm-backup-list">
        {backups.map(b => <div key={b.name} className="pm-backup-row"><div><strong>{b.action === 'shader-cache' ? 'Shader cache' : 'Proton environment'}</strong><small>{b.name}</small></div><button className="pm-subtle-button" disabled={busy} onClick={() => previewRestore(b)} type="button"><RotateCcw size={15}/> Restore</button></div>)}
      </div>}
    </details>
  </section>;
}
