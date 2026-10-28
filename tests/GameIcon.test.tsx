import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render } from '@testing-library/react';
import { GameIcon } from '../src/GameIcon';
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
beforeEach(() => invoke.mockReset());
afterEach(cleanup);
const uri = 'data:image/png;base64,aWNvbg==';
it('loads the executable icon once across ordinary list refreshes', async () => {
  invoke.mockResolvedValue(uri);
  const view = render(<GameIcon source={{kind:'windows',id:'123',executable:'/game.exe'}}/>);
  await act(async()=>{});expect(view.container.querySelector('img')?.getAttribute('src')).toBe(uri);
  view.rerender(<GameIcon source={{kind:'windows',id:'123',executable:'/game.exe'}}/>);
  expect(invoke).toHaveBeenCalledTimes(1);
  view.rerender(<GameIcon source={{kind:'windows',id:'123',executable:'/different.exe'}}/>);
  await act(async()=>{});expect(invoke).toHaveBeenCalledTimes(2);
});
it('keeps a fallback when no executable is selected or an image cannot load', async()=>{
  const view=render(<GameIcon source={{kind:'windows',id:'123',executable:null}}/>);
  expect(invoke).not.toHaveBeenCalled();expect(view.container.querySelector('svg')).toBeTruthy();
  invoke.mockResolvedValue(uri);view.rerender(<GameIcon source={{kind:'windows',id:'123',executable:'/game.exe'}}/>);await act(async()=>{});
  fireEvent.error(view.container.querySelector('img')!);expect(view.container.querySelector('svg')).toBeTruthy();
});
it('ignores an old icon response after a different game is selected',async()=>{
 let finish!:(value:string)=>void;invoke.mockImplementationOnce(()=>new Promise<string>(resolve=>{finish=resolve;})).mockResolvedValue(null);
 const view=render(<GameIcon source={{kind:'windows',id:'1',executable:'/one.exe'}}/>);
 view.rerender(<GameIcon source={{kind:'windows',id:'2',executable:'/two.exe'}}/>);await act(async()=>{finish(uri);});
 expect(view.container.querySelector('img')).toBeNull();
});
it('requests Steam artwork without accepting remote image URLs',async()=>{
 invoke.mockResolvedValue('https://example.test/tracker.png');const view=render(<GameIcon source={{kind:'steam',appId:'123',libraryPath:'/Steam'}}/>);await act(async()=>{});
 expect(invoke).toHaveBeenCalledWith('get_steam_game_icon',{appId:'123',libraryPath:'/Steam'});expect(view.container.querySelector('img')).toBeNull();
});
