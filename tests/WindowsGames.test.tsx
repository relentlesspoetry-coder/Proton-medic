import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { WindowsGames } from '../src/WindowsGames';
const { invoke, open } = vi.hoisted(() => ({ invoke: vi.fn(), open: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open }));
const runtime={id:'wine:/fixture/wine',name:'Fixture Wine',kind:'wine'};
const game={id:'123',name:'Fixture',runtimeId:runtime.id,gameExe:null,state:'installer-exited',exitCode:0,message:'Select and test the installed game',directory:'/managed/123',windowsDrive:'/managed/123/compatdata/pfx/drive_c',logDirectory:'/managed/123/logs/1'};
beforeEach(()=>{vi.useFakeTimers();invoke.mockReset();open.mockReset();invoke.mockResolvedValue({runtimes:[runtime],games:[],warnings:[]});});
afterEach(()=>{cleanup();vi.useRealTimers();});
const mount=async()=>{render(<WindowsGames/>);await act(async()=>{});};
const click=async(name:string)=>{await act(async()=>{fireEvent.click(screen.getByRole('button',{name}));});};
it('does not run a chosen installer until the user approves it',async()=>{
 await mount();open.mockResolvedValue('/Downloads/Game Setup.exe');await click('Choose installer (.exe)');
 expect((screen.getByRole('button',{name:'Run installer'}) as HTMLButtonElement).disabled).toBe(true);
 expect(invoke.mock.calls.every(c=>c[0]==='list_windows_games')).toBe(true);
 fireEvent.click(screen.getByRole('checkbox'));await click('Run installer');
 expect(invoke).toHaveBeenCalledWith('install_windows_game',{name:'Game Setup',installer:'/Downloads/Game Setup.exe',runtimeId:runtime.id,confirmed:true});
 expect((screen.getByRole('checkbox') as HTMLInputElement).checked).toBe(false);
});
it('requires a runtime even after choosing and approving an installer',async()=>{
 invoke.mockResolvedValue({runtimes:[],games:[],warnings:[]});await mount();open.mockResolvedValue('/tmp/setup.exe');await click('Choose installer (.exe)');fireEvent.click(screen.getByRole('checkbox'));
 expect((screen.getByRole('button',{name:'Run installer'}) as HTMLButtonElement).disabled).toBe(true);
});
it('canceling the picker does not execute anything',async()=>{
 await mount();open.mockResolvedValue(null);await click('Choose installer (.exe)');expect(invoke.mock.calls.every(c=>c[0]==='list_windows_games')).toBe(true);
});
it('editing the installer name clears prior approval',async()=>{
 await mount();fireEvent.click(screen.getByRole('checkbox'));fireEvent.change(screen.getByRole('textbox',{name:'Game name'}),{target:{value:'Changed'}});expect((screen.getByRole('checkbox') as HTMLInputElement).checked).toBe(false);
});
it('installer exit zero does not enable launch before executable selection',async()=>{
 invoke.mockResolvedValue({runtimes:[runtime],games:[game],warnings:[]});await mount();expect((screen.getByRole('button',{name:'Launch game'}) as HTMLButtonElement).disabled).toBe(true);
 open.mockResolvedValue(game.windowsDrive+'/Game/game.exe');await click('Select installed game .exe');
 expect(invoke).toHaveBeenCalledWith('select_windows_game_executable',{id:'123',path:game.windowsDrive+'/Game/game.exe',confirmed:true});
});
it('launches a selected executable only after a launch click',async()=>{
 invoke.mockResolvedValue({runtimes:[runtime],games:[{...game,gameExe:game.windowsDrive+'/game.exe'}],warnings:[]});await mount();expect(invoke.mock.calls.every(c=>c[0]==='list_windows_games')).toBe(true);await click('Launch game');expect(invoke).toHaveBeenCalledWith('launch_windows_game',{id:'123',confirmed:true});
});

it('blocks relaunch and executable changes for an unresolved previous run',async()=>{
 invoke.mockResolvedValue({runtimes:[runtime],games:[{...game,state:'session-ended',gameExe:'/game.exe'}],warnings:[]});await mount();
 expect((screen.getByRole('button',{name:'Launch game'}) as HTMLButtonElement).disabled).toBe(true);
 expect((screen.getByRole('button',{name:'Select installed game .exe'}) as HTMLButtonElement).disabled).toBe(true);
});
it('shows unreadable and partial log states independently of signatures',async()=>{
 invoke.mockImplementation((name:string)=>Promise.resolve(name==='analyze_windows_game'?{findings:[],logs:[{name:'stderr.log',status:'unreadable',truncated:true}]}:{runtimes:[runtime],games:[game],warnings:[]}));
 await mount();await click('Read runtime logs');expect(screen.getByText(/stderr.log: unreadable/)).toBeTruthy();expect(screen.getByText(/partial log/)).toBeTruthy();
});
