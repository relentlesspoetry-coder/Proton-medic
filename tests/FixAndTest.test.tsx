import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { FixAndTest } from '../src/FixAndTest';
import type { Game } from '../src/types';
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
const game: Game = { appId: '123', name: 'Fixture', libraryPath: '/fixture', installPath: '/fixture/game', installPathExists: true, manifestPath: '', prefixPath: '', prefixExists: true, buildId: null };
const logging = {state:'ready',message:'Logging enabled',backupPath:'/fixture/log-backup',canRestore:true};
function mockCommands(handler: (name: string) => Promise<unknown>) { invoke.mockImplementation((name: string) => name === 'get_game_logging' ? Promise.resolve(null) : name === 'prepare_game_logging' ? Promise.resolve(logging) : handler(name)); }
const plan = { canApply: true, tool: 'Proton Experimental', reason: 'Supported candidate', evidence: ['wine-exception'] };
const started = { sessionId: 'fixture-session', selection: { toolName: 'Proton Experimental', backupPath: '/fixture/config.bak', message: 'Selection saved', changed: true } };
beforeEach(() => { invoke.mockReset(); sessionStorage.clear(); });
afterEach(() => { cleanup(); vi.restoreAllMocks(); });
async function preview() {
  render(<FixAndTest game={game}/>);
  fireEvent.click(screen.getByRole('button', { name: 'Diagnose this game' }));
  await screen.findByText('Try Proton Experimental');
}
function approveAndApply() {
  fireEvent.click(screen.getByRole('checkbox'));
  fireEvent.click(screen.getByRole('button', { name: 'Fix & Test' }));
}
describe('Fix & Test interaction', () => {
  it('requires approval before requesting a write or launch', async () => {
    mockCommands(() => Promise.resolve(plan));
    await preview();
    expect((screen.getByRole('button', { name: 'Fix & Test' }) as HTMLButtonElement).disabled).toBe(true);
    expect(invoke.mock.calls.map(c => c[0])).toEqual(['get_game_logging', 'prepare_game_logging', 'plan_game_fix']);
  });
  it('does not launch when the configuration change fails', async () => {
    mockCommands((name: string) => name === 'plan_game_fix' ? Promise.resolve(plan) : Promise.reject('Close Steam first'));
    await preview(); approveAndApply();
    expect(await screen.findByRole('alert')).toHaveProperty('textContent', 'Close Steam first');
    expect(invoke.mock.calls.some(c => c[0] === 'test_game_fix')).toBe(false);
  });
  it('keeps rollback available after a failed launch and component remount', async () => {
    mockCommands((name: string) => name === 'plan_game_fix' ? Promise.resolve(plan) : name === 'start_game_fix' ? Promise.resolve(started) : Promise.reject('Launcher unavailable'));
    await preview(); approveAndApply();
    await screen.findByRole('alert');
    expect(screen.getByText('/fixture/config.bak')).toBeTruthy();
    cleanup(); render(<FixAndTest game={game}/>);
    expect(screen.getByRole('button', { name: 'Roll back fix' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Fix & Test' })).toBeNull();
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    invoke.mockResolvedValueOnce('Prior configuration restored');
    fireEvent.click(screen.getByRole('button', { name: 'Roll back fix' }));
    await screen.findByText('Prior configuration restored');
    expect(invoke).toHaveBeenLastCalledWith('rollback_game_fix', { sessionId: 'fixture-session', confirmed: true });
    expect(sessionStorage.getItem('pm-fix:/fixture:123')).toBeNull();
  });
  it('does not label a clean log as verified gameplay', async () => {
    mockCommands((name: string) => Promise.resolve(name === 'plan_game_fix' ? plan : name === 'start_game_fix' ? started : { status: 'log-no-known-errors', detail: 'No recognized errors', findingCodes: [] }));
    await preview(); approveAndApply();
    await screen.findByText('Launch observed — confirm playability yourself');
    expect(invoke).toHaveBeenCalledWith('start_game_fix', expect.objectContaining({ expectedTool: 'Proton Experimental', confirmed: true }));
    expect(invoke).toHaveBeenCalledWith('test_game_fix', { sessionId: 'fixture-session', confirmed: true });
    await waitFor(() => expect((screen.getByRole('button', { name: 'Keep this selection' }) as HTMLButtonElement).disabled).toBe(false));
  });
  it('does not offer a mutation for unsupported evidence', async () => {
    mockCommands(() => Promise.resolve({ ...plan, canApply: false, reason: 'System repair required' }));
    render(<FixAndTest game={game}/>);
    fireEvent.click(screen.getByRole('button', { name: 'Diagnose this game' }));
    await screen.findByText('System repair required');
    expect(screen.queryByRole('checkbox')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Fix & Test' })).toBeNull();
  });
  it('continues diagnosis and approved launch when account selection is ambiguous', async () => {
    invoke.mockImplementation((name: string) => {
      if (name === 'get_game_logging') return Promise.resolve(null);
      if (name === 'prepare_game_logging') return Promise.reject('Ambiguous Steam account');
      if (name === 'plan_game_fix') return Promise.resolve(plan);
      if (name === 'start_game_fix') return Promise.resolve(started);
      return Promise.resolve({status:'no-fresh-log',detail:'Log setup unavailable; launch continued',findingCodes:[]});
    });
    await preview();
    expect(screen.getByText(/Automatic log setup unavailable: Ambiguous Steam account/)).toBeTruthy();
    approveAndApply();
    await screen.findByText('Log setup unavailable; launch continued');
    expect(invoke).toHaveBeenCalledWith('test_game_fix', {sessionId:'fixture-session',confirmed:true});
  });
  it('offers restoration of the previous logging options', async () => {
    mockCommands(() => Promise.resolve(plan));
    await preview();
    expect(screen.getByText('/fixture/log-backup')).toBeTruthy();
    vi.spyOn(window,'confirm').mockReturnValue(true);
    invoke.mockResolvedValueOnce({state:'restored',message:'Logging options restored',backupPath:'/fixture/restored-backup',canRestore:false});
    fireEvent.click(screen.getByRole('button', {name:'Restore previous log settings'}));
    await screen.findByText('Logging options restored');
    expect(invoke).toHaveBeenLastCalledWith('restore_game_logging',{appId:'123',libraryPath:'/fixture',confirmed:true});
  });

  it('launches an approved fix even while automatic logging is queued', async () => {
    invoke.mockImplementation((name: string) => {
      if (name === 'get_game_logging') return Promise.resolve(null);
      if (name === 'prepare_game_logging') return Promise.resolve({state:'waiting-for-steam',message:'Queued until Steam closes',backupPath:null,canRestore:false});
      if (name === 'plan_game_fix') return Promise.resolve(plan);
      if (name === 'start_game_fix') return Promise.resolve(started);
      return Promise.resolve({status:'no-fresh-log',detail:'Launch continued with existing settings',findingCodes:[]});
    });
    await preview();approveAndApply();
    await screen.findByText('Launch continued with existing settings');
    expect(invoke).toHaveBeenCalledWith('test_game_fix',{sessionId:'fixture-session',confirmed:true});
    expect(JSON.parse(sessionStorage.getItem('pm-fix:/fixture:123')!).attempted).toBe(true);
  });

});

it('reloads the log-settings restore receipt when reopening a game',async()=>{
 invoke.mockImplementation((name:string)=>name==='get_game_logging'?Promise.resolve(logging):Promise.resolve(null));
 render(<FixAndTest game={game}/>);
 expect(await screen.findByRole('button',{name:'Restore previous log settings'})).toBeTruthy();
 expect(invoke).toHaveBeenCalledWith('get_game_logging',{appId:'123',libraryPath:'/fixture'});
});
