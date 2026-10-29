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
 invoke.mockResolvedValue({runtimes:[runtime],games:[game],warnings:[]});await mount();expect(screen.queryByRole('button',{name:'Launch game'})).toBeNull();
 open.mockResolvedValue(game.windowsDrive+'/Game/game.exe');await click('Select installed game .exe');
 expect(invoke).toHaveBeenCalledWith('select_windows_game_executable',{id:'123',path:game.windowsDrive+'/Game/game.exe',confirmed:true});
});
it('launches a selected executable only after a launch click',async()=>{
 invoke.mockResolvedValue({runtimes:[runtime],games:[{...game,gameExe:game.windowsDrive+'/game.exe'}],warnings:[]});await mount();expect(invoke.mock.calls.every(c=>['list_windows_games','get_windows_game_icon'].includes(c[0]))).toBe(true);await click('Launch game');expect(invoke).toHaveBeenCalledWith('launch_windows_game',{id:'123',confirmed:true});
});

it('blocks relaunch and executable changes for an unresolved previous run',async()=>{
 invoke.mockResolvedValue({runtimes:[runtime],games:[{...game,state:'session-ended',gameExe:'/game.exe'}],warnings:[]});await mount();
 expect((screen.getByRole('button',{name:'Launch game'}) as HTMLButtonElement).disabled).toBe(true);
 expect((screen.getByRole('button',{name:'Change game .exe'}) as HTMLButtonElement).disabled).toBe(true);
});
it('shows unreadable and partial log states independently of signatures',async()=>{
 invoke.mockImplementation((name:string)=>Promise.resolve(name==='analyze_windows_game'?{findings:[],logs:[{name:'stderr.log',status:'unreadable',truncated:true}]}:{runtimes:[runtime],games:[game],warnings:[]}));
 await mount();fireEvent.click(screen.getByText('Game settings and logs'));await click('Read runtime logs');expect(screen.getByText(/stderr.log: unreadable/)).toBeTruthy();expect(screen.getByText(/partial log/)).toBeTruthy();
});

it('registers an existing game only after choosing its prefix and approving', async()=>{
 await mount();await click('Add installed game');open.mockResolvedValueOnce('/Games/Wukong/b1.exe');await click('Choose game (.exe)');
 fireEvent.change(screen.getByRole('textbox',{name:'Game name'}),{target:{value:'Black Myth Wukong'}});
 open.mockResolvedValueOnce('/existing/pfx');await click('Choose existing prefix');
 expect((screen.getByRole('button',{name:'Add game',exact:true}) as HTMLButtonElement).disabled).toBe(true);
 fireEvent.click(screen.getByRole('checkbox'));await click('Add game');
 expect(invoke).toHaveBeenCalledWith('register_windows_game',{name:'Black Myth Wukong',path:'/Games/Wukong/b1.exe',prefix:'/existing/pfx',runtimeId:runtime.id,confirmed:true});
});
it('creates a shortcut only after the explicit shortcut click',async()=>{
 invoke.mockImplementation((command:string)=>Promise.resolve(command==='create_windows_game_shortcut'?['/Desktop/game.desktop']:{runtimes:[runtime],games:[{...game,gameExe:'/Games/Wukong/b1.exe'}],warnings:[]}));
 await mount();expect(invoke.mock.calls.every(c=>['list_windows_games','get_windows_game_icon'].includes(c[0]))).toBe(true);
 await click('Create desktop shortcut');expect(invoke).toHaveBeenCalledWith('create_windows_game_shortcut',{id:'123',confirmed:true});
 expect(screen.getByText(/Shortcuts created/)).toBeTruthy();
});
it('finding executables neither selects nor launches one automatically',async()=>{
 invoke.mockImplementation((command:string)=>Promise.resolve(command==='find_windows_game_executables'?{executables:['/Games/Wukong/b1.exe'],truncated:false}:{runtimes:[runtime],games:[game],warnings:[]}));
 await mount();await click('Find game executables');expect(invoke).not.toHaveBeenCalledWith('launch_windows_game',expect.anything());
 expect((screen.getByRole('button',{name:'Use selected executable'}) as HTMLButtonElement).disabled).toBe(true);
 fireEvent.change(screen.getByRole('combobox',{name:'Executable for Fixture'}),{target:{value:'/Games/Wukong/b1.exe'}});
 invoke.mockImplementation((command:string)=>Promise.resolve(command==='select_windows_game_executable'?{...game,gameExe:'/Games/Wukong/b1.exe'}:{runtimes:[runtime],games:[game],warnings:[]}));
 await click('Use selected executable');expect(invoke).toHaveBeenCalledWith('select_windows_game_executable',{id:'123',path:'/Games/Wukong/b1.exe',confirmed:true});
});

it('renames a game from a visible card action',async()=>{
 let current={...game};invoke.mockImplementation((command:string,args:any)=>{if(command==='rename_windows_game'){current={...current,name:args.name};return Promise.resolve(current);}return Promise.resolve({runtimes:[runtime],games:[current],warnings:[]});});
 await mount();await click('Rename Fixture');fireEvent.change(screen.getByRole('textbox',{name:'Display name for Fixture'}),{target:{value:'Black Myth Wukong'}});await click('Save name');
 expect(invoke).toHaveBeenCalledWith('rename_windows_game',{id:'123',name:'Black Myth Wukong',confirmed:true});expect(screen.getByRole('heading',{name:'Black Myth Wukong'})).toBeTruthy();
});
it('removes only after confirmation and restores a removed game',async()=>{
 let hidden=false;invoke.mockImplementation((command:string,args:any)=>{if(command==='set_windows_game_hidden'){hidden=args.hidden;return Promise.resolve();}return Promise.resolve({runtimes:[runtime],games:hidden?[]:[game],removedGames:hidden?[game]:[],warnings:[]});});
 await mount();await click('Remove Fixture from list');expect(invoke).not.toHaveBeenCalledWith('set_windows_game_hidden',expect.anything());
 await click('Cancel');expect(screen.getByRole('heading',{name:'Fixture'})).toBeTruthy();
 await click('Remove Fixture from list');await click('Confirm removal');expect(screen.queryByRole('heading',{name:'Fixture'})).toBeNull();
 fireEvent.click(screen.getByText('Removed games (1)'));await click('Restore Fixture');expect(screen.getByRole('heading',{name:'Fixture'})).toBeTruthy();
 expect(invoke).toHaveBeenCalledWith('set_windows_game_hidden',{id:'123',hidden:false,confirmed:true});
});
it('blocks rename and removal while the game is running',async()=>{
 invoke.mockResolvedValue({runtimes:[runtime],games:[{...game,state:'game-running'}],warnings:[]});await mount();
 expect((screen.getByRole('button',{name:'Rename Fixture'}) as HTMLButtonElement).disabled).toBe(true);expect((screen.getByRole('button',{name:'Remove Fixture from list'}) as HTMLButtonElement).disabled).toBe(true);
});

it('saves launch settings through the native command without launching a game',async()=>{
 let current={...game,gameExe:'/game.exe',launchArguments:[] as string[],workingDirectory:null as string|null};
 invoke.mockImplementation(async(command:string,args:any)=>{if(command==='save_windows_launch_settings'){current={...current,runtimeId:args.value.runtimeId,launchArguments:args.value.arguments,workingDirectory:args.value.workingDirectory};return current;}return {runtimes:[runtime],games:[current],warnings:[]};});
 await mount();fireEvent.click(screen.getByText('Game settings and logs'));await click('Edit launch settings');
 fireEvent.change(screen.getByRole('textbox',{name:'Launch arguments for Fixture'}),{target:{value:'-dx12\ntwo words'}});fireEvent.change(screen.getByRole('textbox',{name:'Working directory for Fixture'}),{target:{value:'/Games/With Spaces'}});
 await click('Save launch settings');expect(invoke).toHaveBeenCalledWith('save_windows_launch_settings',{id:'123',value:{runtimeId:runtime.id,arguments:['-dx12','two words'],workingDirectory:'/Games/With Spaces'},expected:{runtimeId:runtime.id,arguments:[],workingDirectory:null},confirmed:true});
 expect(invoke.mock.calls.some(c=>c[0]==='launch_windows_game')).toBe(false);expect(screen.getByText(/Launch settings saved with a profile backup/)).toBeTruthy();
});
it('keeps a confirmed game when optional shortcut creation fails',async()=>{
 let current={...game};invoke.mockImplementation(async(command:string,args:any)=>{
  if(command==='review_windows_installation')return {candidates:[{path:'/Games/Wukong/b1.exe',suggestedName:'Wukong'}],truncated:false};
  if(command==='confirm_windows_installation'){current={...current,name:args.name,gameExe:args.path,state:'ready-to-test'};return current;}
  if(command==='create_windows_game_shortcut')throw new Error('Desktop unavailable');
  return {runtimes:[runtime],games:[current],warnings:[]};
 });
 await mount();fireEvent.change(screen.getByRole('combobox',{name:'Installed executable for Fixture'}),{target:{value:'/Games/Wukong/b1.exe'}});fireEvent.change(screen.getByRole('textbox',{name:'Installed game name for Fixture'}),{target:{value:'Wukong'}});
 fireEvent.click(screen.getByLabelText('I trust the selected executable and approve saving this game.'));fireEvent.click(screen.getByLabelText('Create desktop and applications-menu shortcuts'));await click('Confirm game');
 expect(invoke).toHaveBeenCalledWith('confirm_windows_installation',{id:'123',name:'Wukong',path:'/Games/Wukong/b1.exe',confirmed:true});expect(screen.getByRole('heading',{name:'Wukong'})).toBeTruthy();expect(screen.getByText(/Game saved, but shortcut creation failed/)).toBeTruthy();expect(invoke.mock.calls.some(c=>c[0]==='launch_windows_game')).toBe(false);
});

it('clears a temporary library error after a successful refresh',async()=>{
 invoke.mockRejectedValueOnce(new Error('Temporary read failure'));
 await mount();
 expect(screen.getByRole('alert').textContent).toContain('Temporary read failure');
 expect(screen.queryByText('Loading Windows games…')).toBeNull();
 await click('Refresh');
 expect(screen.queryByRole('alert')).toBeNull();
 expect(screen.getByText('No Windows games have been added yet.')).toBeTruthy();
});
it('keeps loaded games visible when a later refresh fails',async()=>{
 invoke.mockImplementation((cmd:string)=>Promise.resolve(cmd==='get_windows_game_icon'?null:{runtimes:[runtime],games:[{...game,gameExe:'/game.exe'}],warnings:[]}));
 await mount();
 invoke.mockRejectedValueOnce(new Error('Temporary read failure'));
 await click('Refresh');
 expect(screen.getByRole('heading',{name:'Fixture'})).toBeTruthy();
 expect(screen.getByRole('alert').textContent).toContain('Your saved games are unchanged');
});
it('successful background refreshes do not dismiss operation failures',async()=>{
 await mount();open.mockRejectedValueOnce(new Error('Picker unavailable'));
 await click('Choose installer (.exe)');
 await act(async()=>{await vi.advanceTimersByTimeAsync(5000);});
 expect(screen.getByRole('alert').textContent).toContain('Picker unavailable');
});

it('opens only the selected unified-library game while retaining its controls',async()=>{
 const back=vi.fn();invoke.mockImplementation((command:string)=>Promise.resolve(command==='list_windows_games'?{runtimes:[runtime],games:[{...game,gameExe:'/game.exe'},{...game,id:'456',name:'Other game'}],warnings:[]}:null));
 render(<WindowsGames selectedId="123" onBack={back}/>);await act(async()=>{});
 expect(screen.getByRole('button',{name:'Launch game'})).toBeTruthy();expect(screen.getByText('Launch history')).toBeTruthy();expect(screen.getByRole('button',{name:'Monitor resources'})).toBeTruthy();expect(screen.queryByRole('heading',{name:'Other game'})).toBeNull();expect(screen.queryByRole('region',{name:'Add Windows game'})).toBeNull();
 fireEvent.click(screen.getByRole('button',{name:'Back to My Games'}));expect(back).toHaveBeenCalledTimes(1);
});
it('explains when a selected game was removed from the visible library',async()=>{
 invoke.mockResolvedValue({games:[],runtimes:[runtime],warnings:[]});render(<WindowsGames selectedId="123" onBack={()=>{}}/>);await act(async()=>{});expect(screen.getByText(/no longer in the visible library/)).toBeTruthy();
});

it('clears approval when a refresh replaces the selected runtime',async()=>{
 let runtimes=[runtime];invoke.mockImplementation(async command=>command==='list_windows_games'?{runtimes,games:[],warnings:[]}:null);
 await mount();open.mockResolvedValue('/tmp/setup.exe');await click('Choose installer (.exe)');fireEvent.click(screen.getByRole('checkbox'));
 runtimes=[{...runtime,id:'wine:replacement',name:'Replacement Wine'}];await click('Refresh');
 expect((screen.getByLabelText('Runtime') as HTMLSelectElement).value).toBe('wine:replacement');
 expect((screen.getByRole('checkbox') as HTMLInputElement).checked).toBe(false);
 expect((screen.getByRole('button',{name:'Run installer'}) as HTMLButtonElement).disabled).toBe(true);
 expect(invoke).not.toHaveBeenCalledWith('install_windows_game',expect.anything());
});
it('keeps approval when a refresh retains the selected runtime',async()=>{
 await mount();open.mockResolvedValue('/tmp/setup.exe');await click('Choose installer (.exe)');fireEvent.click(screen.getByRole('checkbox'));await click('Refresh');
 expect((screen.getByRole('checkbox') as HTMLInputElement).checked).toBe(true);
});
it('removes old findings when a new run replaces the log directory',async()=>{
 let current={...game};invoke.mockImplementation(async command=>command==='analyze_windows_game'?{findings:[{code:'old',title:'Old run error',recommendation:'Old recommendation'}],logs:[]}:{runtimes:[runtime],games:[current],warnings:[]});
 await mount();fireEvent.click(screen.getByText('Game settings and logs'));await click('Read runtime logs');expect(screen.getByText('Old run error')).toBeTruthy();
 current={...current,logDirectory:'/managed/123/logs/2',state:'game-running'};await click('Refresh');expect(screen.queryByText('Old run error')).toBeNull();
});
it('ignores a late log response from a previous run',async()=>{
 let current={...game};let resolve!:(value:unknown)=>void;
 invoke.mockImplementation(async command=>command==='analyze_windows_game'?new Promise(r=>{resolve=r;}):{runtimes:[runtime],games:[current],warnings:[]});
 await mount();fireEvent.click(screen.getByText('Game settings and logs'));fireEvent.click(screen.getByRole('button',{name:'Read runtime logs'}));
 current={...current,logDirectory:'/managed/123/logs/2',state:'game-running'};
 await act(async()=>{await vi.advanceTimersByTimeAsync(5000);});
 await act(async()=>{resolve({findings:[{code:'old',title:'Late old error',recommendation:'Old recommendation'}],logs:[]});});
 expect(screen.queryByText('Late old error')).toBeNull();
});
