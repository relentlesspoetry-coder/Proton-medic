import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import type { Game, Runtime } from './WindowsGames';
export interface LaunchSettings { runtimeId: string; arguments: string[]; workingDirectory: string | null }
export const settingsFor=(game:Game):LaunchSettings=>({runtimeId:game.runtimeId,arguments:game.launchArguments??[],workingDirectory:game.workingDirectory??null});
type Props={game:Game;runtimes:Runtime[];busy:boolean;run:(task:()=>Promise<void>)=>Promise<void>;onSave:(value:LaunchSettings,expected:LaunchSettings)=>Promise<void>;onReset:(expected:LaunchSettings)=>Promise<void>};
export function WindowsLaunchSettings({game,runtimes,busy,run,onSave,onReset}:Props){
 const [editing,setEditing]=useState(false);
 const [baseline,setBaseline]=useState(()=>settingsFor(game));
 const [runtime,setRuntime]=useState(game.runtimeId);
 const [args,setArgs]=useState('');
 const [working,setWorking]=useState('');
 const disabled=busy||game.state.endsWith('running')||game.state==='session-ended';
 const edit=()=>{const value=settingsFor(game);setBaseline(value);setRuntime(value.runtimeId);setArgs(value.arguments.join('\n'));setWorking(value.workingDirectory??'');setEditing(true);};
 return <div className="pm-launch-settings"><h4>Launch settings</h4>
 {!editing?<><dl className="pm-windows-paths"><dt>Runtime</dt><dd>{runtimes.find(r=>r.id===game.runtimeId)?.name??game.runtimeId}</dd><dt>Arguments</dt><dd>{game.launchArguments?.length?<code>{JSON.stringify(game.launchArguments)}</code>:'None'}</dd><dt>Working directory</dt><dd>{game.workingDirectory??'Game executable folder'}</dd></dl><button className="pm-button pm-button-secondary" disabled={disabled} onClick={edit}>Edit launch settings</button></>:<form onSubmit={event=>{event.preventDefault();void run(async()=>{await onSave({runtimeId:runtime,arguments:args.split(/\r?\n/).filter(value=>value!==''),workingDirectory:working||null},baseline);setEditing(false);});}}>
 <div className="pm-form-grid"><label>Game runtime<select aria-label={'Game runtime for '+game.name} value={runtime} disabled={disabled} onChange={event=>setRuntime(event.target.value)}>
 {!runtimes.some(r=>r.id===runtime)&&<option value={runtime}>Unavailable: {runtime}</option>}
 {runtimes.map(r=><option key={r.id} value={r.id}>{r.name} ({r.kind})</option>)}</select></label>
 <label>Working directory<input aria-label={'Working directory for '+game.name} value={working} disabled={disabled} placeholder="Default: game executable folder" onChange={event=>setWorking(event.target.value)}/></label></div>
 <button type="button" className="pm-subtle-button" disabled={disabled} onClick={()=>void run(async()=>{const directory=await open({directory:true,multiple:false,title:'Choose the game working directory',defaultPath:working||undefined});if(typeof directory==='string')setWorking(directory);})}>Choose working directory</button>
 <label className="pm-argument-field">Launch arguments<textarea aria-label={'Launch arguments for '+game.name} value={args} disabled={disabled} maxLength={16384} rows={3} placeholder={'-dx12\n-windowed'} onChange={event=>setArgs(event.target.value)}/><small>One argument per line. Spaces within a line are preserved. Blank lines are ignored; shell commands and variables are not expanded.</small></label>
 <p className="pm-settings-help">Changing the runtime keeps the existing prefix and game files. Each saved change is backed up.</p>
 <div className="pm-button-row"><button type="submit" className="pm-button pm-button-primary" disabled={disabled||!runtimes.some(r=>r.id===runtime)}>Save launch settings</button><button type="button" className="pm-button pm-button-secondary" disabled={disabled} onClick={()=>{if(window.confirm('Restore the original runtime, clear launch arguments, and use the game executable folder? Current settings will be backed up.'))void run(async()=>{await onReset(baseline);setEditing(false);});}}>Reset launch settings</button><button type="button" className="pm-button pm-button-secondary" disabled={busy} onClick={()=>setEditing(false)}>Cancel editing</button></div>
 </form>}</div>;
}
