import { afterEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { WindowsLaunchSettings } from '../src/WindowsLaunchSettings';
afterEach(cleanup);
const game={id:'123',name:'Fixture',runtimeId:'wine:old',gameExe:'/game.exe',state:'game-exited',exitCode:0,message:'',directory:'/managed',windowsDrive:'/prefix/drive_c',prefixDirectory:'/prefix',logDirectory:null};
const runtimes=[{id:'wine:old',name:'Old Wine',kind:'wine'},{id:'wine:new',name:'New Wine',kind:'wine'}];
const run=async(task:()=>Promise<void>)=>{await task();};
it('saves literal arguments and a working directory with the original expected values',async()=>{
 const save=vi.fn().mockResolvedValue(undefined);render(<WindowsLaunchSettings game={game} runtimes={runtimes} busy={false} run={run} onSave={save} onReset={vi.fn()}/>);
 fireEvent.click(screen.getByText('Edit launch settings'));
 fireEvent.change(screen.getByRole('combobox'),{target:{value:'wine:new'}});
 fireEvent.change(screen.getByRole('textbox',{name:'Working directory for Fixture'}),{target:{value:'/Games/With Spaces'}});
 fireEvent.change(screen.getByRole('textbox',{name:'Launch arguments for Fixture'}),{target:{value:'-dx12\ntwo words\n\n$HOME'}});
 fireEvent.click(screen.getByText('Save launch settings'));
 expect(save).toHaveBeenCalledWith({runtimeId:'wine:new',arguments:['-dx12','two words','$HOME'],workingDirectory:'/Games/With Spaces'},{runtimeId:'wine:old',arguments:[],workingDirectory:null});
});
it('keeps an editing snapshot across polling and does not save cancelled drafts',()=>{
 const save=vi.fn();const view=render(<WindowsLaunchSettings game={game} runtimes={runtimes} busy={false} run={run} onSave={save} onReset={vi.fn()}/>);fireEvent.click(screen.getByText('Edit launch settings'));
 fireEvent.change(screen.getByRole('textbox',{name:'Launch arguments for Fixture'}),{target:{value:'draft'}});
 view.rerender(<WindowsLaunchSettings game={{...game,launchArguments:['external']}} runtimes={runtimes} busy={false} run={run} onSave={save} onReset={vi.fn()}/>);
 expect((screen.getByRole('textbox',{name:'Launch arguments for Fixture'}) as HTMLTextAreaElement).value).toBe('draft');fireEvent.click(screen.getByText('Cancel editing'));expect(save).not.toHaveBeenCalled();
});
it('requires confirmation before resetting and blocks running profiles',()=>{
 const reset=vi.fn().mockResolvedValue(undefined);const confirm=vi.spyOn(window,'confirm').mockReturnValue(false);
 const view=render(<WindowsLaunchSettings game={game} runtimes={runtimes} busy={false} run={run} onSave={vi.fn()} onReset={reset}/>);fireEvent.click(screen.getByText('Edit launch settings'));fireEvent.click(screen.getByText('Reset launch settings'));expect(reset).not.toHaveBeenCalled();confirm.mockReturnValue(true);fireEvent.click(screen.getByText('Reset launch settings'));expect(reset).toHaveBeenCalledWith({runtimeId:'wine:old',arguments:[],workingDirectory:null});confirm.mockRestore();
 view.rerender(<WindowsLaunchSettings game={{...game,state:'game-running'}} runtimes={runtimes} busy={false} run={run} onSave={vi.fn()} onReset={reset}/>);expect((screen.getByText('Save launch settings') as HTMLButtonElement).disabled).toBe(true);
});
