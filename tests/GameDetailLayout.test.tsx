import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, expect, it, vi } from 'vitest';
import { SteamGameDetail } from '../src/gameweave/GameDetailLayout';
import type { Game } from '../src/types';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn().mockResolvedValue(null) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

const steam: Game = {
  appId: '440', name: 'Fixture Game', libraryPath: '/steam', installPath: '/steam/game', installPathExists: true,
  manifestPath: '/steam/game.acf', prefixPath: '/steam/pfx', prefixExists: true,
  configuredTool: 'GE-Proton', buildId: null, lastPlayedAt: null,
};

afterEach(cleanup);

it('uses Steam game facts, selects the versioned cover by identity, and leaves launch behind the existing tab', async () => {
  render(<SteamGameDetail game={steam} status={<span>Not checked</span>} note="No log read yet."
    toolbar={<button>Back</button>} tabs={[{id:'launch',label:'Launch & repair',content:<button>Existing launch action</button>}]}/>);
  await act(async () => {});
  expect(screen.getByRole('heading',{name:'Fixture Game'})).toBeTruthy();
  expect(screen.getByRole('img',{name:'Fixture Game cover art'})).toBeTruthy();
  expect(invoke).toHaveBeenCalledWith('get_game_cover',{source:'steam',id:'440',libraryPath:'/steam'});
  expect(screen.getByText('/steam/game')).toBeTruthy();
  expect(screen.getByText('Not recorded')).toBeTruthy();
  fireEvent.click(screen.getByRole('button',{name:'Test Launch'}));
  const tab = screen.getByRole('tab',{name:'Launch & repair'});
  expect(tab.getAttribute('aria-selected')).toBe('true');
  expect(document.activeElement).toBe(tab);
  expect(within(screen.getByRole('tabpanel')).getByRole('button',{name:'Existing launch action'})).toBeTruthy();
  expect(invoke.mock.calls.some(([command]) => command === 'test_game_launch')).toBe(false);
});

it('does not offer a launch test when the Steam install folder is missing', async () => {
  render(<SteamGameDetail game={{...steam,installPathExists:false}} status={null} note="Missing" toolbar={null} tabs={[]}/>);
  await act(async () => {});
  expect((screen.getByRole('button',{name:'Test Launch'}) as HTMLButtonElement).disabled).toBe(true);
});
