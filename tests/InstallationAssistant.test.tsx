import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { InstallationAssistant } from '../src/InstallationAssistant';
const {invoke}=vi.hoisted(()=>({invoke:vi.fn()}));vi.mock('@tauri-apps/api/core',()=>({invoke}));
afterEach(cleanup);beforeEach(()=>invoke.mockReset());
const game={id:'123',name:'Game Setup',runtimeId:'fixture',gameExe:null,state:'installer-exited',exitCode:0,message:'',directory:'/managed',windowsDrive:'/prefix/drive_c',prefixDirectory:'/prefix',logDirectory:'/logs/one'};
const review={candidates:[{path:'/Games/Wukong/b1.exe',suggestedName:'Wukong'}],truncated:false};
const run=async(task:()=>Promise<void>)=>{await task();};
it('discovers after setup but never selects, launches, or creates a shortcut automatically',async()=>{
 invoke.mockResolvedValue(review);const confirm=vi.fn();const view=render(<InstallationAssistant game={game} busy={false} run={run} onConfirm={confirm}/>);await screen.findByRole('combobox');
 expect((screen.getByRole('combobox') as HTMLSelectElement).value).toBe('');expect((screen.getByText('Confirm game') as HTMLButtonElement).disabled).toBe(true);expect(confirm).not.toHaveBeenCalled();
 view.rerender(<InstallationAssistant game={{...game}} busy={false} run={run} onConfirm={confirm}/>);expect(invoke).toHaveBeenCalledTimes(1);
});
it('requires executable approval and only creates shortcuts when explicitly requested',async()=>{
 invoke.mockImplementation(async(command:string)=>command==='review_windows_installation'?review:null);const confirm=vi.fn().mockResolvedValue(undefined);
 render(<InstallationAssistant game={game} busy={false} run={run} onConfirm={confirm}/>);await screen.findByRole('combobox');
 fireEvent.change(screen.getByRole('combobox'),{target:{value:review.candidates[0].path}});expect((screen.getByRole('textbox') as HTMLInputElement).value).toBe('Wukong');
 fireEvent.click(screen.getByLabelText('I trust the selected executable and approve saving this game.'));fireEvent.change(screen.getByRole('textbox'),{target:{value:'My Wukong'}});expect((screen.getByText('Confirm game') as HTMLButtonElement).disabled).toBe(true);
 fireEvent.click(screen.getByLabelText('I trust the selected executable and approve saving this game.'));fireEvent.click(screen.getByLabelText('Create desktop and applications-menu shortcuts'));
 await act(async()=>{fireEvent.click(screen.getByText('Confirm game'));});expect(confirm).toHaveBeenCalledWith('My Wukong','/Games/Wukong/b1.exe',true);
});
it('preserves manual selection when discovery fails or finds nothing',async()=>{
 invoke.mockRejectedValue(new Error('Unreadable prefix'));const view=render(<InstallationAssistant game={game} busy={false} run={run} onConfirm={vi.fn()}/>);await screen.findByText(/Automatic discovery is unavailable/);
 invoke.mockResolvedValue({candidates:[],truncated:true});view.rerender(<InstallationAssistant game={{...game,logDirectory:'/logs/two'}} busy={false} run={run} onConfirm={vi.fn()}/>);await screen.findByText(/No likely game executable found/);expect(screen.getByText(/search reached its limit/)).toBeTruthy();
});
it('updates suggested names on executable changes while preserving manual names and renewing consent',async()=>{
 const choices={candidates:[...review.candidates,{path:'/Games/Other/game.exe',suggestedName:'Other Game'}],truncated:false};
 invoke.mockImplementation(async(command:string)=>command==='review_windows_installation'?choices:null);const confirm=vi.fn().mockResolvedValue(undefined);
 render(<InstallationAssistant game={game} busy={false} run={run} onConfirm={confirm}/>);await screen.findByRole('combobox');
 const choose=(path:string)=>fireEvent.change(screen.getByRole('combobox'),{target:{value:path}});
 choose(choices.candidates[0].path);fireEvent.click(screen.getByLabelText('I trust the selected executable and approve saving this game.'));
 choose(choices.candidates[1].path);expect((screen.getByRole('textbox') as HTMLInputElement).value).toBe('Other Game');expect((screen.getByText('Confirm game') as HTMLButtonElement).disabled).toBe(true);
 fireEvent.click(screen.getByLabelText('I trust the selected executable and approve saving this game.'));await act(async()=>fireEvent.click(screen.getByText('Confirm game')));
 expect(confirm).toHaveBeenCalledWith('Other Game',choices.candidates[1].path,false);
 fireEvent.change(screen.getByRole('textbox'),{target:{value:'My chosen name'}});choose(choices.candidates[0].path);
 expect((screen.getByRole('textbox') as HTMLInputElement).value).toBe('My chosen name');
});
