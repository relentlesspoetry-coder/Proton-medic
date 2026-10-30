import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import App from '../src/App';
import type { AutoSelection, Game, VersionAdvice } from '../src/types';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check: vi.fn() }));

const games: Game[] = ['Alpha', 'Beta'].map(name => ({
  appId: '123', name, libraryPath: `/fixture/${name}`, installPath: `/fixture/${name}/game`,
  installPathExists: true, manifestPath: '/fixture/manifest', prefixPath: '/fixture/prefix',
  prefixExists: true, buildId: null,
}));
const advice: VersionAdvice[] = games.map(game => ({
  appId: game.appId, libraryPath: game.libraryPath, gameName: game.name, status: 'try-version',
  suggestedTool: 'GE-Proton', configuredTool: null, reason: 'A different installed version may help.',
  nextStep: 'Try the suggested version.', confidence: 'tentative', alternatives: [], evidence: [],
}));
const selection = (name: string) => ({ gameName: name, appId: '123', toolName: 'GE-Proton', changed: false }) as AutoSelection;
const successMessage = 'Alpha: GE-Proton is already selected.';

beforeEach(() => {
  invoke.mockReset(); localStorage.clear(); sessionStorage.clear();
  invoke.mockImplementation(async (command: string) => {
    if (command === 'scan_steam') return { games, installations: [], libraries: [], compatibilityTools: [], warnings: [] };
    if (command === 'list_windows_games') return { games: [], runtimes: [], removedGames: [], warnings: [] };
    if (command === 'get_auto_fix_control') return { enabled: false, revision: 0 };
    if (['monitor_game_logs', 'list_fix_sessions', 'list_repair_backups', 'list_game_protons'].includes(command)) return [];
    if (command === 'analyze_diagnostics') return { games: [] };
    if (command === 'recommend_proton') return { games: advice };
    if (command === 'auto_select_proton') return selection('Alpha');
    return null;
  });
});
afterEach(cleanup);

async function openGame(name: string) {
  fireEvent.click(await screen.findByRole('button', { name: `Game settings for ${name} (Steam)` }));
  fireEvent.click(screen.getByRole('tab', { name: 'Proton', exact: true }));
  await screen.findByRole('button', { name: 'Automatically select GE-Proton' });
}
function back() { fireEvent.click(screen.getByRole('button', { name: 'Back to My Games' })); }
function apply() { fireEvent.click(screen.getByRole('button', { name: 'Automatically select GE-Proton' })); }

it('keeps completed feedback with its game and preserves the latest result in Settings', async () => {
  render(<App/>); await openGame('Alpha'); apply();
  await screen.findByText(successMessage);
  back(); await openGame('Beta');
  expect(screen.queryByText(successMessage)).toBeNull();
  back(); await openGame('Alpha');
  expect(screen.getByText(successMessage)).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Settings', exact: true }));
  expect(screen.getByText(successMessage)).toBeTruthy();
});

it('does not show a late failure in another game with the same app ID', async () => {
  let rejectSelection!: (reason: Error) => void;
  const original = invoke.getMockImplementation()!;
  invoke.mockImplementation((command: string, args: unknown) => command === 'auto_select_proton'
    ? new Promise((_, reject) => { rejectSelection = reject; }) : original(command, args));
  render(<App/>); await openGame('Alpha'); apply();
  await waitFor(() => expect(invoke).toHaveBeenCalledWith('auto_select_proton', { appId: '123', libraryPath: '/fixture/Alpha', confirmed: true }));
  back();
  fireEvent.click(await screen.findByRole('button', { name: 'Game settings for Beta (Steam)' }));
  fireEvent.click(screen.getByRole('tab', { name: 'Proton', exact: true }));
  expect((screen.getByRole('button', { name: 'Selecting Proton…' }) as HTMLButtonElement).disabled).toBe(true);
  await act(async () => rejectSelection(new Error('Configuration unavailable')));
  expect(screen.queryByText('Could not apply the selection: Error: Configuration unavailable')).toBeNull();
  back(); await openGame('Alpha');
  expect(screen.getByText('Could not apply the selection: Error: Configuration unavailable')).toBeTruthy();
});

it('clears old feedback while retrying a selection', async () => {
  render(<App/>); await openGame('Alpha'); apply();
  await screen.findByText(successMessage);
  let finish!: (value: AutoSelection) => void;
  const original = invoke.getMockImplementation()!;
  invoke.mockImplementation((command: string, args: unknown) => command === 'auto_select_proton'
    ? new Promise(resolve => { finish = resolve; }) : original(command, args));
  apply();
  expect(screen.queryByText(successMessage)).toBeNull();
  expect((screen.getByRole('button', { name: 'Selecting Proton…' }) as HTMLButtonElement).disabled).toBe(true);
  await act(async () => finish(selection('Alpha')));
  expect(screen.getByText(successMessage)).toBeTruthy();
});
