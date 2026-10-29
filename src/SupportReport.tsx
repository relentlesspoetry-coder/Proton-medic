import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Download, FileText } from 'lucide-react';
interface Options { steam: boolean; windows: boolean; history: boolean; redactPaths: boolean }
interface Preview { token: number; content: string }
export function SupportReport() {
  const [expanded, setExpanded] = useState(false);
  const [options, setOptions] = useState<Options>({ steam: true, windows: true, history: false, redactPaths: true });
  const [preview, setPreview] = useState<Preview | null>(null);
  const [approved, setApproved] = useState(false);
  const [busy, setBusy] = useState<'prepare' | 'save' | null>(null);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const sequence = useRef(0);
  const locked = useRef(false);
  useEffect(() => () => { ++sequence.current; }, []);
  const change = (key: keyof Options, value: boolean) => {
    setOptions(old => ({ ...old, [key]: value })); setPreview(null); setApproved(false); setError(''); setNotice('');
  };
  const run = async (action: 'prepare' | 'save') => {
    if (locked.current || (action === 'save' && (!preview || !approved))) return;
    locked.current = true; const request = ++sequence.current;
    setBusy(action); setError(''); setNotice('');
    try {
      if (action === 'prepare') {
        setPreview(null); setApproved(false);
        const next = await invoke<Preview>('prepare_support_report', { options });
        if (request === sequence.current) setPreview(next);
      } else {
        const path = await invoke<string | null>('save_support_report', { token: preview!.token, confirmed: true });
        if (request === sequence.current) setNotice(path ? `Report saved to ${path}. Nothing was uploaded.` : 'Save canceled. Your preview is still available.');
      }
    } catch (e) { if (request === sequence.current) setError(String(e)); }
    finally { locked.current = false; if (request === sequence.current) setBusy(null); }
  };
  return <section className="pm-section pm-support-report" aria-label="Troubleshooting report">
    <div className="pm-section-heading"><div><h2>Troubleshooting report</h2><p>Review diagnostics and save a local file to share.</p></div><button className="pm-button pm-button-secondary" disabled={!!busy} onClick={() => { setExpanded(!expanded); setPreview(null); setApproved(false); setNotice(''); setError(''); }}><FileText size={17}/>{expanded ? 'Close report' : 'Create report'}</button></div>
    {expanded && <>
      <p>System details are included. Choose game information below. Raw logs, log excerpts, launch arguments, environment variables, and account data are excluded.</p>
      <fieldset className="pm-report-options" disabled={!!busy}><legend>Include in this report</legend>
        <label><input type="checkbox" checked={options.steam} onChange={e => change('steam', e.target.checked)}/>Steam game diagnostics</label>
        <label><input type="checkbox" checked={options.windows} onChange={e => change('windows', e.target.checked)}/>Windows game status and runtime</label>
        <label><input type="checkbox" disabled={!options.windows} checked={options.history} onChange={e => change('history', e.target.checked)}/>Recent Windows launch history</label>
        <label><input type="checkbox" checked={options.redactPaths} onChange={e => change('redactPaths', e.target.checked)}/>Redact personal paths</label>
      </fieldset>
      <p className="pm-report-help">Includes up to 100 Steam games, 50 visible Windows games, and 20 runs per Windows game. Omitted counts and unreadable records are marked. Game names, runtime names, and selected launch timestamps remain visible.</p>
      {!options.redactPaths && <p className="pm-repair-danger" role="alert">Paths may reveal your username and folder names. Review them before sharing.</p>}
      <button className="pm-button pm-button-secondary" disabled={!!busy} onClick={() => void run('prepare')}>{busy === 'prepare' ? 'Preparing preview…' : preview ? 'Refresh preview' : 'Generate preview'}</button>
      {preview && <div className="pm-report-preview"><label htmlFor="support-report-preview">Exact file contents</label><textarea id="support-report-preview" readOnly spellCheck={false} value={preview.content}/>
        <label className="pm-report-approval"><input type="checkbox" disabled={!!busy} checked={approved} onChange={e => setApproved(e.target.checked)}/>I reviewed this report and approve saving it.</label>
        <button className="pm-button pm-button-primary" disabled={!!busy || !approved} onClick={() => void run('save')}><Download size={17}/>{busy === 'save' ? 'Saving…' : 'Save report'}</button>
        <p className="pm-report-help">Choose a new .json file. Existing files are never replaced. Saving does not send the report anywhere.</p>
      </div>}
      {error && <p className="pm-repair-danger" role="alert">{error}</p>}
      {notice && <p className="pm-auto-message" role="status">{notice}</p>}
    </>}
  </section>;
}
