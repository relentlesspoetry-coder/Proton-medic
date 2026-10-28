import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { AutoLogMonitor } from '../src/AutoLogMonitor';
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
beforeEach(() => { vi.useFakeTimers(); invoke.mockReset(); sessionStorage.clear(); });
afterEach(() => { cleanup(); vi.useRealTimers(); });
it('reads automatically with writes disabled and polls without overlapping calls', async () => {
  let resolve!: (value: unknown) => void;
  invoke.mockImplementationOnce(() => new Promise(r => { resolve = r; })).mockResolvedValue([]);
  render(<AutoLogMonitor enabled={false} onToggle={() => {}}/>);
  expect(invoke).toHaveBeenCalledWith('monitor_game_logs', { enabled: false });
  await act(async () => { await vi.advanceTimersByTimeAsync(30_000); });
  expect(invoke).toHaveBeenCalledTimes(1);
  await act(async () => resolve([]));
  await act(async () => { await vi.advanceTimersByTimeAsync(10_000); });
  expect(invoke).toHaveBeenCalledTimes(2);
});
it('does not enable repairs without a toggle action', async () => {
  invoke.mockResolvedValue([]);const onToggle=vi.fn();
  render(<AutoLogMonitor enabled={false} onToggle={onToggle}/>);
  await act(async () => {});
  expect(onToggle).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('checkbox'));
  expect(onToggle).toHaveBeenCalledWith(true);
});
it('delivers a rollback receipt without overwriting an existing launch-attempt flag', async () => {
  const selection={backupPath:'/fixture/backup',toolName:'Proton'};
  sessionStorage.setItem('pm-fix:/fixture:123',JSON.stringify({sessionId:'session', selection, attempted:true}));
  invoke.mockResolvedValue([{appId:'123',libraryPath:'/fixture',gameName:'Fixture',state:'applied',message:'Review result',codes:['wine-exception'],session:{sessionId:'session',selection}}]);
  const event=vi.fn();window.addEventListener('pm-fix-session',event);
  render(<AutoLogMonitor enabled={true} onToggle={() => {}}/>);
  await act(async () => {});
  expect(JSON.parse(sessionStorage.getItem('pm-fix:/fixture:123')!).attempted).toBe(true);
  expect(event).toHaveBeenCalledTimes(1);
  await act(async () => { await vi.advanceTimersByTimeAsync(10_000); });
  expect(event).toHaveBeenCalledTimes(1);
  window.removeEventListener('pm-fix-session',event);
});
it('stops polling when the component closes', async () => {
  invoke.mockResolvedValue([]);
  const view=render(<AutoLogMonitor enabled={false} onToggle={() => {}}/>);
  await act(async () => {});view.unmount();
  await act(async () => { await vi.advanceTimersByTimeAsync(20_000); });
  expect(invoke).toHaveBeenCalledTimes(1);
});

it('keeps polling in compact mode without an empty panel or a duplicate toggle',async()=>{
 invoke.mockResolvedValue([]);const view=render(<AutoLogMonitor enabled={false} onToggle={()=>{}} expanded={false}/>);await act(async()=>{});
 expect(view.container.textContent).toBe('');expect(screen.queryByRole('checkbox')).toBeNull();
 await act(async()=>{await vi.advanceTimersByTimeAsync(10000);});expect(invoke).toHaveBeenCalledTimes(2);
});
it('shows actionable monitor results in compact mode',async()=>{
 invoke.mockResolvedValue([{appId:'123',libraryPath:'/fixture',gameName:'Fixture',state:'queued',message:'Close Steam to continue',codes:[],session:null}]);
 render(<AutoLogMonitor enabled={true} onToggle={()=>{}} expanded={false}/>);await act(async()=>{});
 expect(screen.getByText('Close Steam to continue')).toBeTruthy();expect(screen.queryByRole('checkbox')).toBeNull();
});
