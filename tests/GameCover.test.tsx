import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { GameCover } from '../src/GameCover';
const { invoke, open } = vi.hoisted(() => ({ invoke: vi.fn(), open: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open }));
const steam = { source: 'steam' as const, id: '123', libraryPath: '/Steam' };
const windows = { source: 'windows' as const, id: '1-2', libraryPath: null };
const uri = 'data:image/jpeg;base64,Y292ZXI=';
beforeEach(() => { invoke.mockReset(); open.mockReset(); invoke.mockResolvedValue(null); });
afterEach(() => { cleanup(); vi.unstubAllGlobals(); });
it('loads a separate Steam cover and opens the game without requesting its icon', async () => {
  invoke.mockResolvedValue(uri); const onOpen=vi.fn();
  const view=render(<GameCover source={steam} name="Example" onOpen={onOpen}/>);
  await waitFor(()=>expect(view.container.querySelector('img')?.getAttribute('src')).toBe(uri));
  expect(invoke).toHaveBeenCalledWith('get_game_cover',steam);
  view.rerender(<GameCover source={{...steam}} name="Renamed" onOpen={onOpen}/>);
  expect(invoke).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole('button',{name:'Open Renamed (Steam)'})); expect(onOpen).toHaveBeenCalledTimes(1);
});
it('keeps a titled fallback rather than accepting a remote URL or icon', async () => {
  invoke.mockResolvedValue('https://example.com/cover.png');
  const view=render(<GameCover source={windows} name="Example" onOpen={()=>{}}/>);
  await act(async()=>{});
  expect(view.container.querySelector('img')).toBeNull();
  expect(view.container.querySelector('.pm-cover-placeholder')?.textContent).toContain('Example');
});
it('ignores stale cover responses after changing game identity', async () => {
  let finish!:(value:string)=>void;
  invoke.mockImplementationOnce(()=>new Promise<string>(resolve=>{finish=resolve;}));
  const view=render(<GameCover source={steam} name="First" onOpen={()=>{}}/>);
  view.rerender(<GameCover source={windows} name="Second" onOpen={()=>{}}/>);
  await act(async()=>{finish(uri);}); expect(view.container.querySelector('img')).toBeNull();
});
it('imports a chosen image and does not let an earlier fetch replace it', async () => {
  let finish!:(value:string)=>void;
  invoke.mockImplementationOnce(()=>new Promise<string>(resolve=>{finish=resolve;})).mockResolvedValue(uri);
  open.mockResolvedValue('/covers/example.jpg');
  const view=render(<GameCover source={windows} name="Example" onOpen={()=>{}}/>);
  fireEvent.click(screen.getByRole('button',{name:'Change cover for Example (Windows)'}));
  await waitFor(()=>expect(invoke).toHaveBeenCalledWith('set_game_cover',{...windows,path:'/covers/example.jpg',confirmed:true}));
  await waitFor(()=>expect(view.container.querySelector('img')?.getAttribute('src')).toBe(uri));
  await act(async()=>{finish('data:image/png;base64,b2xk');});
  expect(view.container.querySelector('img')?.getAttribute('src')).toBe(uri);
});
it('cancels image selection without writing anything', async () => {
  open.mockResolvedValue(null);render(<GameCover source={windows} name="Example" onOpen={()=>{}}/>);
  fireEvent.click(screen.getByRole('button',{name:'Change cover for Example (Windows)'}));await act(async()=>{});
  expect(invoke.mock.calls.map(call=>call[0])).toEqual(['get_game_cover']);
});
it('does not import a selection returned after its card changes game', async () => {
  let finish!:(value:string)=>void;open.mockImplementation(()=>new Promise<string>(resolve=>{finish=resolve;}));
  const view=render(<GameCover source={steam} name="First" onOpen={()=>{}}/>);
  fireEvent.click(screen.getByRole('button',{name:'Change cover for First (Steam)'}));
  view.rerender(<GameCover source={windows} name="Second" onOpen={()=>{}}/>);
  await act(async()=>{finish('/cover.jpg');});
  expect(invoke.mock.calls.some(call=>call[0]==='set_game_cover')).toBe(false);
});
it('retains the previous cover and shows failed import errors', async () => {
  invoke.mockResolvedValueOnce(uri).mockRejectedValueOnce('Image is too small');open.mockResolvedValue('/tiny.png');
  const view=render(<GameCover source={windows} name="Example" onOpen={()=>{}}/>);await act(async()=>{});
  fireEvent.click(screen.getByRole('button',{name:'Change cover for Example (Windows)'}));
  expect((await screen.findByRole('alert')).textContent).toContain('Image is too small');
  expect(view.container.querySelector('img')?.getAttribute('src')).toBe(uri);
  fireEvent.error(view.container.querySelector('img')!);expect(view.container.querySelector('img')).toBeNull();
});
it('defers cover loading until the card approaches the viewport', async () => {
  let observe!:(entries:{isIntersecting:boolean}[])=>void;const disconnect=vi.fn();
  vi.stubGlobal('IntersectionObserver',class { constructor(callback:typeof observe){observe=callback;} observe(){} disconnect=disconnect; });
  invoke.mockResolvedValue(uri);render(<GameCover source={steam} name="Example" onOpen={()=>{}}/>);
  expect(invoke).not.toHaveBeenCalled();await act(async()=>{observe([{isIntersecting:true}]);});
  expect(invoke).toHaveBeenCalledTimes(1);expect(disconnect).toHaveBeenCalled();
});
