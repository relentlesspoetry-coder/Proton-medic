import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { ActiveFixSessions } from '../src/ActiveFixSessions';
import { FixAndTest } from '../src/FixAndTest';
import type { Game } from '../src/types';
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
const game: Game = { appId: '123', name: 'Fixture', libraryPath: '/disconnected', installPath: '', installPathExists: false, manifestPath: '', prefixPath: '', prefixExists: false, buildId: null };
const active = { sessionId: 'saved', appId: '123', libraryPath: '/disconnected', attempted: true, selection: { toolName: 'Proton', backupPath: '/backup' } };
beforeEach(() => { invoke.mockReset(); sessionStorage.clear(); });
afterEach(() => { cleanup(); vi.restoreAllMocks(); });
it('recovers a backend session after remount when browser storage is unavailable', async () => {
  vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => { throw new Error('Unavailable'); });
  vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('Full'); });
  invoke.mockImplementation((cmd: string) => Promise.resolve(cmd === 'list_fix_sessions' ? [active] : null));
  const first = render(<FixAndTest game={game}/>);
  await screen.findByRole('button', { name: 'Roll back fix' });
  first.unmount(); render(<FixAndTest game={game}/>);
  await screen.findByRole('button', { name: 'Keep this selection' });
  expect(screen.queryByRole('button', { name: 'Launch & test now' })).toBeNull();
});
it('finishes a session without inventory or browser receipts', async () => {
  let sessions = [active];
  invoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'list_fix_sessions') return sessions;
    if (cmd === 'finish_game_fix') { sessions = []; return null; }
    throw new Error('Unexpected command');
  });
  render(<ActiveFixSessions/>);
  fireEvent.click(await screen.findByRole('button', { name: 'Keep selection' }));
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Unfinished repairs' })).toBeNull());
  expect(invoke).toHaveBeenCalledWith('finish_game_fix', { sessionId: 'saved', confirmed: true });
});
it('keeps recovery controls if rollback fails', async () => {
  invoke.mockImplementation((cmd: string) => cmd === 'list_fix_sessions' ? Promise.resolve([active]) : Promise.reject('Close Steam first'));
  render(<ActiveFixSessions/>);
  fireEvent.click(await screen.findByRole('button', { name: 'Roll back' }));
  expect(await screen.findByRole('alert')).toHaveProperty('textContent', 'Close Steam first');
  expect(screen.getByRole('button', { name: 'Keep selection' })).toBeTruthy();
});

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(done => { resolve = done; });
  return { promise, resolve };
}

it.each(['Keep selection', 'Roll back'])('does not restore a closed global session after %s', async label => {
  const previous = deferred<typeof active[]>();
  let reads = 0;
  invoke.mockImplementation((command: string) => {
    if (command === 'list_fix_sessions') {
      ++reads;
      return reads === 1 ? Promise.resolve([active]) : reads === 2 ? previous.promise : Promise.resolve([]);
    }
    return Promise.resolve(null);
  });
  render(<ActiveFixSessions/>);
  await screen.findByRole('button', { name: label });
  act(() => { window.dispatchEvent(new Event('pm-fix-session')); });
  fireEvent.click(screen.getByRole('button', { name: label }));
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Unfinished repairs' })).toBeNull());
  await act(async () => { previous.resolve([active]); });
  expect(screen.queryByRole('region', { name: 'Unfinished repairs' })).toBeNull();
});

it('retains the latest list when earlier refreshes finish out of order', async () => {
  const previous = deferred<typeof active[]>();
  let reads = 0;
  invoke.mockImplementation((command: string) => {
    if (command === 'list_fix_sessions') return ++reads === 1 ? previous.promise : Promise.resolve([]);
    return Promise.resolve(null);
  });
  render(<ActiveFixSessions/>);
  await act(async () => { window.dispatchEvent(new Event('pm-fix-session')); });
  await act(async () => { previous.resolve([active]); });
  expect(screen.queryByRole('region', { name: 'Unfinished repairs' })).toBeNull();
});

it('clears a recovered listing error after a successful authoritative refresh', async () => {
  invoke.mockRejectedValueOnce('Temporary listing failure').mockResolvedValue([]);
  render(<ActiveFixSessions/>);
  await screen.findByRole('alert');
  await act(async () => { window.dispatchEvent(new Event('pm-fix-session')); });
  expect(screen.queryByRole('region', { name: 'Unfinished repairs' })).toBeNull();
});

it('clears a rollback error after the game view keeps that same session', async () => {
  let sessions = [active];
  invoke.mockImplementation(async (command: string) => {
    if (command === 'list_fix_sessions') return sessions;
    if (command === 'rollback_game_fix') throw new Error('Close Steam first');
    if (command === 'finish_game_fix') sessions = [];
    return null;
  });
  render(<><ActiveFixSessions/><FixAndTest game={game}/></>);
  fireEvent.click(await screen.findByRole('button', { name: 'Roll back', exact: true }));
  await screen.findByRole('alert');
  fireEvent.click(screen.getByRole('button', { name: 'Keep this selection' }));
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Unfinished repairs' })).toBeNull());
  expect(screen.queryByRole('alert')).toBeNull();
});

it('preserves an action error while its session still exists through successful refreshes', async () => {
  invoke.mockImplementation(async (command: string) => {
    if (command === 'list_fix_sessions') return [active];
    throw new Error('Newer mapping prevents rollback');
  });
  render(<ActiveFixSessions/>);
  fireEvent.click(await screen.findByRole('button', { name: 'Roll back' }));
  await screen.findByRole('alert');
  await act(async () => { window.dispatchEvent(new Event('pm-fix-session')); });
  expect(screen.getByRole('alert').textContent).toContain('Newer mapping prevents rollback');
  expect(screen.getByRole('button', { name: 'Keep selection' })).toBeTruthy();
});
