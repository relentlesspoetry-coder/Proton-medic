import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { SupportReport } from '../src/SupportReport';
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
const preview = { token: 7, content: '{"pathsRedacted":true}\n' };
const click = async (name: string) => { await act(async () => { fireEvent.click(screen.getByRole('button', { name, exact: true })); }); };
const start = async () => { render(<SupportReport/>); await click('Create report'); };
const prepare = async () => { await start(); await click('Generate preview'); };
beforeEach(() => { invoke.mockReset(); invoke.mockResolvedValue(preview); });
afterEach(cleanup);
it('collects nothing until requested and defaults to redaction without history', async () => {
  await start(); expect(invoke).not.toHaveBeenCalled(); await click('Generate preview');
  expect(invoke).toHaveBeenCalledWith('prepare_support_report', { options: { steam: true, windows: true, history: false, redactPaths: true } });
  expect((screen.getByLabelText('Exact file contents') as HTMLTextAreaElement).value).toBe(preview.content);
  expect((screen.getByRole('button', { name: 'Save report' }) as HTMLButtonElement).disabled).toBe(true);
});
it('saves only the approved snapshot token and reports a completed save', async () => {
  await prepare(); fireEvent.click(screen.getByLabelText('I reviewed this report and approve saving it.'));
  invoke.mockResolvedValue('/reports/report.json'); await click('Save report');
  expect(invoke).toHaveBeenLastCalledWith('save_support_report', { token: 7, confirmed: true });
  expect(screen.getByRole('status').textContent).toContain('Nothing was uploaded');
});
it('invalidates preview and approval when scope or privacy changes', async () => {
  await prepare(); fireEvent.click(screen.getByLabelText('I reviewed this report and approve saving it.'));
  fireEvent.click(screen.getByLabelText('Redact personal paths')); expect(screen.queryByLabelText('Exact file contents')).toBeNull();
  expect(screen.getByRole('alert').textContent).toContain('username'); await click('Generate preview');
  expect((screen.getByRole('button', { name: 'Save report' }) as HTMLButtonElement).disabled).toBe(true);
});
it('disables history selection when Windows status is excluded', async () => {
  await start(); fireEvent.click(screen.getByLabelText('Windows game status and runtime'));
  expect((screen.getByLabelText('Recent Windows launch history') as HTMLInputElement).disabled).toBe(true);
});
it('keeps a canceled save distinct from success and permits retry', async () => {
  await prepare(); fireEvent.click(screen.getByLabelText('I reviewed this report and approve saving it.'));
  invoke.mockResolvedValue(null); await click('Save report'); expect(screen.getByRole('status').textContent).toContain('Save canceled');
  invoke.mockResolvedValue('/new.json'); await click('Save report'); expect(screen.getByRole('status').textContent).toContain('Report saved');
});
it('shows write failures without claiming the report was saved', async () => {
  await prepare(); fireEvent.click(screen.getByLabelText('I reviewed this report and approve saving it.'));
  invoke.mockRejectedValue(new Error('Choose a new filename')); await click('Save report');
  expect(screen.getByRole('alert').textContent).toContain('Choose a new filename'); expect(screen.queryByRole('status')).toBeNull();
});
it('removes the previous preview if refresh fails', async () => {
  await prepare(); invoke.mockRejectedValue(new Error('Read failed')); await click('Refresh preview');
  expect(screen.queryByLabelText('Exact file contents')).toBeNull(); expect(screen.getByRole('alert').textContent).toContain('Read failed');
});
it('prevents duplicate preparation while a request is running', async () => {
  let resolve!:(value:unknown)=>void; invoke.mockImplementation(() => new Promise(done => { resolve=done; }));
  await start(); await click('Generate preview'); expect((screen.getByRole('button',{name:'Preparing preview…'}) as HTMLButtonElement).disabled).toBe(true);
  await act(async () => resolve(preview)); expect(invoke).toHaveBeenCalledTimes(1);
});
it('discards the preview when closing and reopening the report', async () => {
  await prepare(); await click('Close report'); await click('Create report'); expect(screen.queryByLabelText('Exact file contents')).toBeNull();
});
