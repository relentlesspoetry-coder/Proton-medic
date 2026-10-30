import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { GameIcon } from './GameIcon';
import type { Game } from './WindowsGames';
interface Review { candidates:{path:string;suggestedName:string}[];truncated:boolean }
type Props={game:Game;busy:boolean;run:(task:()=>Promise<void>)=>Promise<void>;onConfirm:(name:string,path:string,shortcut:boolean)=>Promise<void>};
export function InstallationAssistant({game,busy,run,onConfirm}:Props){
 const [review,setReview]=useState<Review|null>(null);
 const [error,setError]=useState('');
 const [selected,setSelected]=useState('');
 const [name,setName]=useState(game.name);
 const [nameEdited,setNameEdited]=useState(false);
 const [approved,setApproved]=useState(false);
 const [shortcut,setShortcut]=useState(false);
 useEffect(()=>{
  let stopped=false;setReview(null);setError('');setSelected('');setApproved(false);
  void invoke<Review>('review_windows_installation',{id:game.id}).then(result=>{if(!stopped&&Array.isArray(result?.candidates))setReview(result);}).catch(error=>{if(!stopped)setError(String(error));});
  return ()=>{stopped=true;};
 },[game.id,game.logDirectory]);
 return <section className="pm-install-review" aria-label={'Finish installation of '+game.name}>
 <h4>Finish adding your game</h4><p>Setup has closed. Confirm the executable that starts your game; finding a file does not verify the installation.</p>
 {!review&&!error&&<p role="status">Looking for game executables…</p>}
 {error&&<p role="status">Automatic discovery is unavailable. Use Select installed game .exe below to choose it manually.</p>}
 {review&&(!review.candidates.length?<p>No likely game executable found. Choose it manually below, including files on other drives.</p>:<form onSubmit={event=>{event.preventDefault();if(selected&&name.trim()&&approved)void run(()=>onConfirm(name,selected,shortcut));}}>
 <div className="pm-install-choice"><GameIcon source={{kind:'candidate',id:game.id,executable:selected||null}}/><label>Game executable<select aria-label={'Installed executable for '+game.name} value={selected} disabled={busy} onChange={event=>{const path=event.target.value;setSelected(path);setApproved(false);const candidate=review.candidates.find(c=>c.path===path);if(candidate&&/setup|install/i.test(game.name)&&!nameEdited)setName(candidate.suggestedName);}}><option value="">Choose a game executable…</option>{review.candidates.map(candidate=><option key={candidate.path} value={candidate.path}>{candidate.path}</option>)}</select></label></div>
 <label className="pm-review-name">Game name<input aria-label={'Installed game name for '+game.name} value={name} maxLength={120} disabled={busy} onChange={event=>{setName(event.target.value);setNameEdited(true);setApproved(false);}}/></label>
 <label className="pm-launch-consent"><input type="checkbox" checked={approved} disabled={busy} onChange={event=>setApproved(event.target.checked)}/><span>I trust the selected executable and approve saving this game.</span></label>
 <label className="pm-launch-consent"><input type="checkbox" checked={shortcut} disabled={busy} onChange={event=>setShortcut(event.target.checked)}/><span>Create desktop and applications-menu shortcuts</span></label>
 <button type="submit" className="pm-button pm-button-primary" disabled={busy||!selected||!name.trim()||!approved}>Confirm game</button></form>)}
 {review?.truncated&&<p>The search reached its limit. You can select another executable manually.</p>}
 </section>;
}
