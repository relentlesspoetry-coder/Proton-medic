import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { LaunchHistory } from '../src/LaunchHistory';
import type { Game } from '../src/WindowsGames';
const {invoke}=vi.hoisted(()=>({invoke:vi.fn()}));
vi.mock('@tauri-apps/api/core',()=>({invoke}));
const game:Game={id:'123',name:'Fixture',runtimeId:'wine',gameExe:'/game.exe',state:'game-exited',exitCode:0,message:'Ended',directory:'/managed/123',windowsDrive:'/prefix/drive_c',prefixDirectory:'/prefix',logDirectory:'/logs/1'};
const entry={id:'1-100',kind:'game',runtime:'Wine fixture',startedAt:1700000000000,endedAt:1700000065000,state:'game-exited',exitCode:0,message:'Game process ended.',summaries:[],partialLogs:false};
const expand=async()=>{await act(async()=>{const details=screen.getByText('Launch history').closest('details')!;details.open=true;fireEvent(details,new Event('toggle'));});};
beforeEach(()=>{invoke.mockReset();invoke.mockResolvedValue({entries:[entry],warnings:[]});});
afterEach(cleanup);
it('loads only on expansion and reports runtime results without claiming gameplay success',async()=>{
 render(<LaunchHistory game={game}/>);expect(invoke).not.toHaveBeenCalled();await expand();
 expect(invoke).toHaveBeenCalledWith('get_windows_launch_history',{id:'123'});
 expect(screen.getByText('Game · Runtime exit 0')).toBeTruthy();expect(screen.getByText('Wine fixture · 1m 5s')).toBeTruthy();
 expect(screen.getByText(/do not confirm gameplay/)).toBeTruthy();
});
it('shows failures, unknown exits, partial logs and relevant summaries',async()=>{
 invoke.mockResolvedValue({entries:[{...entry,state:'unknown',exitCode:null,endedAt:null,summaries:['A Windows exception was recorded'],partialLogs:true}],warnings:['One record unavailable']});
 render(<LaunchHistory game={game}/>);await expand();expect(screen.getByText('Game · Exit unknown')).toBeTruthy();expect(screen.getByText('A Windows exception was recorded')).toBeTruthy();expect(screen.getByText(/Logs are incomplete/)).toBeTruthy();expect(screen.getByRole('alert').textContent).toContain('One record unavailable');
});
it('keeps previous history on refresh failure and clears the error after retry',async()=>{
 render(<LaunchHistory game={game}/>);await expand();invoke.mockRejectedValueOnce(new Error('temporary read failure'));
 await act(async()=>{fireEvent.click(screen.getByRole('button',{name:'Refresh history'}));});
 expect(screen.getByRole('alert').textContent).toContain('temporary read failure');expect(screen.getByText('Game · Runtime exit 0')).toBeTruthy();
 await act(async()=>{fireEvent.click(screen.getByRole('button',{name:'Refresh history'}));});expect(screen.queryByRole('alert')).toBeNull();
});
it('refreshes an expanded history when a running game finishes',async()=>{
 const view=render(<LaunchHistory game={{...game,state:'game-running'}}/>);await expand();invoke.mockClear();
 view.rerender(<LaunchHistory game={game}/>);await act(async()=>{});expect(invoke).toHaveBeenCalledTimes(1);
});
it('explains the empty history without inventing old results',async()=>{
 invoke.mockResolvedValue({entries:[],warnings:[]});render(<LaunchHistory game={game}/>);await expand();expect(screen.getByText(/earlier logs remain available/)).toBeTruthy();
});
it('ignores a response after the history panel is closed',async()=>{
 let resolve!:(value:unknown)=>void;invoke.mockImplementation(()=>new Promise(done=>{resolve=done;}));render(<LaunchHistory game={game}/>);await expand();
 await act(async()=>{const details=screen.getByText('Launch history').closest('details')!;details.open=false;fireEvent(details,new Event('toggle'));});
 await act(async()=>resolve({entries:[entry],warnings:[]}));expect(screen.queryByText('Game · Runtime exit 0')).toBeNull();
});
