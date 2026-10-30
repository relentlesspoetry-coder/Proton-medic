import { useState } from 'react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { UnifiedLibrary, libraryItems, steamLibraryKey, windowsLibraryKey, type LibraryFilter, type LibrarySource, type LibrarySort } from '../src/UnifiedLibrary';
import type { Game } from '../src/types';
import type { Game as WindowsGame } from '../src/WindowsGames';
const {invoke}=vi.hoisted(()=>({invoke:vi.fn()}));vi.mock('@tauri-apps/api/core',()=>({invoke}));
const steam:Game={appId:'100',name:'Shared name',libraryPath:'/Steam/A',installPath:'/Steam/A/game',installPathExists:true,manifestPath:'/manifest',prefixPath:'/prefix',prefixExists:true,buildId:null,lastPlayedAt:1700000000000};
const windows:WindowsGame={id:'123',name:'Shared name',runtimeId:'wine',gameExe:'/game.exe',state:'game-exited',exitCode:0,message:'Ended',directory:'/managed/123',windowsDrive:'/prefix/drive_c',prefixDirectory:'/prefix',logDirectory:'/logs/1',lastLaunchedAt:1700000060000};
const onSteam=vi.fn(),onWindows=vi.fn();let windowsData={games:[windows],removedGames:[{...windows,id:'999',name:'Hidden game'}],warnings:[],runtimes:[]};
function Harness({games=[steam],refreshKey=0}:{games?:Game[];refreshKey?:number}){
 const [query,onQuery]=useState('');const [filter,onFilter]=useState<LibraryFilter>('all');const [source,onSource]=useState<LibrarySource>('all');const [sort,onSort]=useState<LibrarySort>('name');
 return <UnifiedLibrary steam={games} steamLoading={false} scanAvailable refreshKey={refreshKey} problemKeys={new Set(['/Steam/A:100'])} query={query} onQuery={onQuery} filter={filter} onFilter={onFilter} source={source} onSource={onSource} sort={sort} onSort={onSort} onSteam={onSteam} onWindows={onWindows} steamStatus={()=>'Not checked'}/>;
}
const choose=async(name:string,value:string)=>{await act(async()=>{fireEvent.change(screen.getByRole('combobox',{name}),{target:{value}});});};
const mount=async()=>{render(<Harness/>);await act(async()=>{});};
beforeEach(()=>{vi.useFakeTimers();vi.spyOn(document,'hidden','get').mockReturnValue(false);localStorage.clear();invoke.mockReset();onSteam.mockReset();onWindows.mockReset();windowsData={games:[windows],removedGames:[{...windows,id:'999',name:'Hidden game'}],warnings:[],runtimes:[]};invoke.mockImplementation(async(command:string)=>command==='list_windows_games'?windowsData:null);});
afterEach(()=>{cleanup();vi.restoreAllMocks();vi.useRealTimers();});
it('combines identical names without merging source identities or showing removed profiles',async()=>{
 await mount();expect(screen.getByRole('button',{name:'Open Shared name (Steam)'})).toBeTruthy();expect(screen.getByRole('button',{name:'Open Shared name (Windows)'})).toBeTruthy();expect(screen.queryByText('Hidden game')).toBeNull();
 fireEvent.click(screen.getByRole('button',{name:'Open Shared name (Windows)'}));expect(onWindows).toHaveBeenCalledWith(windows);expect(onSteam).not.toHaveBeenCalled();
 expect(invoke.mock.calls.every(c=>['list_windows_games','get_windows_game_icon','get_steam_game_icon'].includes(c[0]))).toBe(true);
});
it('filters by source and searches both game collections',async()=>{
 await mount();await choose('Game source','windows');expect(screen.queryByRole('button',{name:'Open Shared name (Steam)'})).toBeNull();
 await choose('Game source','all');fireEvent.change(screen.getByRole('textbox',{name:'Search games'}),{target:{value:'100'}});expect(screen.getByRole('button',{name:'Open Shared name (Steam)'})).toBeTruthy();expect(screen.queryByRole('button',{name:'Open Shared name (Windows)'})).toBeNull();
});
it('persists favorites independently of names and keeps Steam library copies distinct',async()=>{
 await mount();fireEvent.click(screen.getByRole('button',{name:'Add Shared name (Windows) to favorites'}));await choose('Filter games','favorites');expect(screen.queryByRole('button',{name:'Open Shared name (Steam)'})).toBeNull();
 cleanup();windowsData.games=[{...windows,name:'Renamed game'}];await mount();await choose('Filter games','favorites');expect(screen.getByRole('button',{name:'Open Renamed game (Windows)'})).toBeTruthy();
 expect(steamLibraryKey(steam)).not.toBe(steamLibraryKey({...steam,libraryPath:'/Steam/B'}));expect(windowsLibraryKey(windows)).not.toBe(steamLibraryKey(steam));
});
it('sorts known recent dates first and unknown dates last',async()=>{
 render(<Harness games={[steam,{...steam,appId:'200',name:'A unknown',lastPlayedAt:null}]}/>);await act(async()=>{});await choose('Sort games','recent');
 const names=within(screen.getByRole('region',{name:'Unified game library'})).getAllByRole('button',{name:/^Open /}).map(e=>e.getAttribute('aria-label'));
 expect(names).toEqual(['Open Shared name (Windows)','Open Shared name (Steam)','Open A unknown (Steam)']);expect(screen.getByText(/Last launched/)).toBeTruthy();
});
it('treats future and invalid timestamps as unknown',()=>{
 expect(libraryItems([{...steam,lastPlayedAt:Infinity},{...steam,lastPlayedAt:Date.now()+999999}],[{...windows,lastLaunchedAt:-1}]).every(item=>item.recent===0)).toBe(true);
});
it('preserves Steam-specific filters and includes failed Windows games under attention',async()=>{
 windowsData.games=[{...windows,state:'process-failed'}];await mount();await choose('Filter games','issues');expect(screen.getAllByRole('button',{name:/^Open /})).toHaveLength(2);
 await choose('Filter games','prefix');expect(screen.getAllByRole('button',{name:/^Open /})).toHaveLength(1);expect(screen.getByRole('button',{name:'Open Shared name (Steam)'})).toBeTruthy();
});
it('does not replace malformed saved favorites',async()=>{
 const key='proton-medic.library.favorites.v1';localStorage.setItem(key,'broken');await mount();expect(screen.getByRole('alert').textContent).toContain('left unchanged');fireEvent.click(screen.getByRole('button',{name:'Add Shared name (Steam) to favorites'}));expect(localStorage.getItem(key)).toBe('broken');
});
it('reports storage failure without pretending the favorite was saved',async()=>{
 await mount();vi.spyOn(Storage.prototype,'setItem').mockImplementation(()=>{throw new Error('full');});fireEvent.click(screen.getByRole('button',{name:'Add Shared name (Steam) to favorites'}));expect(screen.getByRole('alert').textContent).toContain('Could not save');expect(screen.getByRole('button',{name:'Add Shared name (Steam) to favorites'}).getAttribute('aria-pressed')).toBe('false');
});
it('keeps both collections visible during a Windows refresh failure and recovers',async()=>{
 await mount();invoke.mockRejectedValueOnce(new Error('temporary failure'));await act(async()=>{vi.advanceTimersByTime(5000);});expect(screen.getByRole('alert').textContent).toContain('temporary failure');expect(screen.getAllByRole('button',{name:/^Open /})).toHaveLength(2);
 await act(async()=>{fireEvent.click(screen.getByRole('button',{name:'Retry Windows library'}));});expect(screen.queryByRole('alert')).toBeNull();
});
it('ignores an older Windows response after the scan refresh key changes',async()=>{
 let finish!:(value:unknown)=>void;invoke.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve;}));const view=render(<Harness refreshKey={0}/>);view.rerender(<Harness refreshKey={1}/>);await act(async()=>{});
 await act(async()=>finish({games:[{...windows,name:'Old response'}],warnings:[],runtimes:[]}));expect(screen.queryByText('Old response')).toBeNull();expect(screen.getByRole('button',{name:'Open Shared name (Windows)'})).toBeTruthy();
});
const inventoryCalls=()=>invoke.mock.calls.filter(call=>call[0]==='list_windows_games').length;
const visibility=async(hidden:boolean)=>{
 vi.spyOn(document,'hidden','get').mockReturnValue(hidden);
 await act(async()=>{document.dispatchEvent(new Event('visibilitychange'));});
};
it('keeps loaded games while hidden and refreshes immediately when shown',async()=>{
 await mount();expect(inventoryCalls()).toBe(1);
 await visibility(true);
 await act(async()=>{await vi.advanceTimersByTimeAsync(30000);});
 expect(inventoryCalls()).toBe(1);expect(screen.getByRole('button',{name:'Open Shared name (Windows)'})).toBeTruthy();
 await visibility(false);expect(inventoryCalls()).toBe(2);
 await act(async()=>{await vi.advanceTimersByTimeAsync(5000);});expect(inventoryCalls()).toBe(3);
 cleanup();await act(async()=>{await vi.advanceTimersByTimeAsync(10000);});expect(inventoryCalls()).toBe(3);
});
it('does not scan on a hidden mount and ignores the response from before hiding',async()=>{
 vi.spyOn(document,'hidden','get').mockReturnValue(true);await mount();expect(inventoryCalls()).toBe(0);
 let finish!:(value:unknown)=>void;
 invoke.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve;}));
 await visibility(false);expect(inventoryCalls()).toBe(1);
 await visibility(true);
 await act(async()=>finish({games:[{...windows,name:'Obsolete response'}],warnings:[],runtimes:[]}));
 expect(screen.queryByText('Obsolete response')).toBeNull();
 await act(async()=>{await vi.advanceTimersByTimeAsync(30000);});expect(inventoryCalls()).toBe(1);
 await visibility(false);expect(inventoryCalls()).toBe(2);expect(screen.getByRole('button',{name:'Open Shared name (Windows)'})).toBeTruthy();
});
