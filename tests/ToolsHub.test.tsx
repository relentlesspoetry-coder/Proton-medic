import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { ToolsHub } from '../src/gameweave/ToolsHub';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
const steam = {
  appId: '42', name: 'Steam Example', libraryPath: '/steam', installPath: '/steam/game', installPathExists: true,
  manifestPath: '/steam/appmanifest_42.acf', prefixPath: '/steam/prefix', prefixExists: false,
  buildId: null, configuredTool: 'GE-Proton10-34',
};
const windowsGame = {
  id: 'one', name: 'Windows Example', runtimeId: 'proton', gameExe: '/windows/game.exe',
  state: 'game-exited', exitCode: 0, message: 'Stopped', directory: '/windows',
  windowsDrive: '/windows/drive_c', prefixDirectory: '/windows/prefix', logDirectory: '/windows/logs/abc',
};
const props = () => ({
  steam:[steam], runtimes:[{name:'GE-Proton10-34',path:'/runtime',source:'Native Steam'}],
  diagnostics:[], scanning:false, onRefresh:vi.fn(), onScan:vi.fn(), onOpenLibrary:vi.fn(),
  onOpenSystem:vi.fn(), onOpenWindows:vi.fn(),
});
const pick = (text: string) => fireEvent.change(screen.getByRole('combobox',{name:'Game for tools'}), {target:{value:(screen.getByRole('option',{name:text}) as HTMLOptionElement).value}});
beforeEach(() => {
  invoke.mockReset();
  invoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'list_windows_games') return {games:[windowsGame],runtimes:[{id:'proton',kind:'proton',name:'GE-Proton10-34'}]};
    if (cmd === 'list_repair_backups') return [];
    if (cmd === 'preview_repair') return {title:'Rebuild Preview',action:'proton-prefix',canApply:true,risk:'high',sourcePath:'/steam/prefix',backupRoot:'/steam/backups'};
    if (cmd === 'analyze_windows_game') return {logs:[{name:'runner.log',status:'read',truncated:false}],findings:[]};
    return null;
  });
});
afterEach(cleanup);

it('shows actual inventory, filters tools, switches grid/list and only opens externally scoped panels on click',async () => {
  const input = props();
  const result = render(<ToolsHub {...input}/>);
  await screen.findByRole('option',{name:'Windows Example (Windows)'});
  expect(screen.getByRole('rowheader',{name:'GE-Proton10-34'})).toBeTruthy();
  expect((screen.getByRole('button',{name:/Repair Game/}) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole('button',{name:'List view'}));
  expect(result.container.querySelector('.gw-tools-actions')?.getAttribute('data-view')).toBe('list');
  fireEvent.change(screen.getByRole('textbox',{name:'Search tools'}),{target:{value:'shortcut'}});
  expect(screen.queryByRole('button',{name:/Repair Game/})).toBeNull();
  pick('Windows Example (Windows)');
  fireEvent.click(screen.getByRole('button',{name:/Create Shortcut/}));
  expect(input.onOpenWindows).toHaveBeenCalledWith(windowsGame);
  expect(invoke.mock.calls.some(([cmd])=>cmd==='create_windows_game_shortcut')).toBe(false);
  fireEvent.click(screen.getByRole('button',{name:'System'}));
  fireEvent.click(screen.getByRole('button',{name:'Open System Check'}));
  expect(input.onOpenSystem).toHaveBeenCalledTimes(1);
});

it('uses read-only diagnostics for real games and keeps repair behind its existing preview and separate consent',async () => {
  const input = props();
  render(<ToolsHub {...input}/>);
  await screen.findByRole('option',{name:'Windows Example (Windows)'});
  pick('Steam Example (Steam)');
  expect(screen.getByText('Proton prefix: Not found')).toBeTruthy();
  fireEvent.click(screen.getByRole('button',{name:'Run scan'}));
  expect(input.onScan).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole('button',{name:'Cache & prefix'}));
  fireEvent.click(screen.getByRole('button',{name:/Rebuild Proton environment/}));
  await screen.findByText('Rebuild Preview');
  expect(invoke).toHaveBeenCalledWith('preview_repair',{appId:'42',libraryPath:'/steam',action:'proton-prefix'});
  expect((screen.getByRole('button',{name:'Archive & continue'}) as HTMLButtonElement).disabled).toBe(true);
  expect(invoke.mock.calls.some(([cmd])=>cmd==='apply_repair')).toBe(false);
  pick('Windows Example (Windows)');
  expect(screen.queryByText('Rebuild Preview')).toBeNull();
  fireEvent.click(screen.getByRole('button',{name:'Run scan'}));
  await waitFor(()=>expect(screen.getByText('runner.log: read')).toBeTruthy());
  expect(invoke).toHaveBeenCalledWith('analyze_windows_game',{id:'one'});
  expect(screen.getByText(/does not verify game files/)).toBeTruthy();
});

it('never silently moves repair controls to another game when the chosen game disappears',async () => {
  const input = props();
  const view = render(<ToolsHub {...input}/>);
  await screen.findByRole('option',{name:'Windows Example (Windows)'});
  pick('Steam Example (Steam)');
  fireEvent.click(screen.getByRole('button',{name:'Cache & prefix'}));
  view.rerender(<ToolsHub {...input} steam={[]}/>);
  expect((screen.getByRole('combobox',{name:'Game for tools'}) as HTMLSelectElement).value).toBe('');
  expect(screen.queryByRole('region',{name:'Selected game action'})).toBeNull();
  expect((screen.getByRole('button',{name:'Run scan'}) as HTMLButtonElement).disabled).toBe(true);
});
