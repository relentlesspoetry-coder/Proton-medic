import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import App from '../src/App';
import { version } from '../package.json';

const { check, invoke } = vi.hoisted(() => ({ check: vi.fn(), invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
beforeEach(() => {
  check.mockReset(); invoke.mockReset(); localStorage.clear();
  invoke.mockImplementation(async (command: string) => {
    if (command === 'scan_steam') return { games: [], installations: [], libraries: [], compatibilityTools: [], warnings: [] };
    if (command === 'monitor_game_logs') return [];
    if (command === 'get_auto_fix_control') return {enabled:false,revision:0};
    return { games: [] };
  });
});
afterEach(cleanup);

it('shows the installed version and keeps a header update check alive across navigation', async () => {
  let finish!: (value: null) => void;
  check.mockImplementation(() => new Promise<null>(resolve => { finish = resolve; }));
  render(<App/>);
  expect(screen.getByText('Version ' + version)).toBeTruthy();
  expect(check).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', {name: 'Check for updates'}));
  await waitFor(() => expect(check).toHaveBeenCalledTimes(1));
  fireEvent.click(screen.getByRole('button', {name: 'Settings', exact: true}));
  expect((screen.getByRole('button', {name: 'Check for updates'}) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole('button', {name: 'Home', exact: true}));
  finish(null);
  await screen.findByText('You are up to date.');
  expect(screen.getAllByRole('button', {name: 'Check for updates'})).toHaveLength(1);
  expect(check).toHaveBeenCalledTimes(1);
});
it('places automatic fix controls before privacy and technical sections in Settings',async()=>{
 render(<App/>);fireEvent.click(screen.getByRole('button',{name:'Settings',exact:true}));
 const monitor=screen.getByRole('region',{name:'Automatic game log reader'});
 expect(monitor.compareDocumentPosition(screen.getByText('Privacy & safety')) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
 expect(screen.getAllByRole('checkbox')).toHaveLength(1);
});
