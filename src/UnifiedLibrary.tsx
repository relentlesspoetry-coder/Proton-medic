import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ArrowRight, Search, Star } from 'lucide-react';
import type { Game as SteamGame } from './types';
import type { Game as WindowsGame, Library } from './WindowsGames';
import { GameIcon } from './GameIcon';
import { PagedList } from './PagedList';
export type LibraryFilter='all'|'favorites'|'issues'|'missing'|'prefix';
export type LibrarySource='all'|'steam'|'windows';
export type LibrarySort='name'|'recent';
const FAVORITES='proton-medic.library.favorites.v1';
type Item={key:string;name:string;recent:number;kind:'steam';game:SteamGame}|{key:string;name:string;recent:number;kind:'windows';game:WindowsGame};
export const steamLibraryKey=(game:SteamGame)=>JSON.stringify(['steam',game.libraryPath,game.appId]);
export const windowsLibraryKey=(game:WindowsGame)=>JSON.stringify(['windows',game.id]);
function timestamp(value:number|null|undefined){return typeof value==='number'&&Number.isFinite(value)&&value>0&&value<=Date.now()?value:0;}
export function libraryItems(steam:SteamGame[],windows:WindowsGame[]):Item[]{return [
 ...steam.map(game=>({key:steamLibraryKey(game),name:game.name,recent:timestamp(game.lastPlayedAt),kind:'steam' as const,game})),
 ...windows.map(game=>({key:windowsLibraryKey(game),name:game.name,recent:timestamp(game.lastLaunchedAt),kind:'windows' as const,game})),
];}
function readFavorites(){
 const raw=localStorage.getItem(FAVORITES);if(raw===null)return new Set<string>();
 const parsed:unknown=JSON.parse(raw);
 if(!Array.isArray(parsed)||parsed.length>10000||parsed.some(value=>typeof value!=='string'||value.length>8192))throw new Error('Invalid favorites data');
 return new Set<string>(parsed);
}
const needsReview=(game:WindowsGame)=>['process-failed','start-failed','session-ended','interrupted'].includes(game.state);
function windowsStatus(game:WindowsGame){
 if(game.state==='installer-running')return 'Installing';if(game.state==='game-running')return 'Running';
 if(needsReview(game))return 'Needs review';if(!game.gameExe)return 'Setup needed';return 'Ready to launch';
}
interface Props {
 steam:SteamGame[];steamLoading:boolean;scanAvailable:boolean;refreshKey:number;problemKeys:Set<string>;
 query:string;onQuery:(value:string)=>void;filter:LibraryFilter;onFilter:(value:LibraryFilter)=>void;
 source:LibrarySource;onSource:(value:LibrarySource)=>void;sort:LibrarySort;onSort:(value:LibrarySort)=>void;
 onSteam:(game:SteamGame)=>void;onWindows:(game:WindowsGame)=>void;steamStatus:(game:SteamGame)=>ReactNode;
}
export function UnifiedLibrary(props:Props){
 const [windows,setWindows]=useState<WindowsGame[]>([]);
 const [warnings,setWarnings]=useState<string[]>([]);
 const [loaded,setLoaded]=useState(false);const [error,setError]=useState('');const [retry,setRetry]=useState(0);
 const [favorites,setFavorites]=useState(()=>{try{return readFavorites();}catch{return new Set<string>();}});
 const [favoriteError,setFavoriteError]=useState(()=>{try{readFavorites();return '';}catch{return 'Saved favorites could not be read. They have been left unchanged.';}});
 useEffect(()=>{
  let stopped=false;let timer:ReturnType<typeof setTimeout>;
  const refresh=async()=>{
   try{
    const next=await invoke<Library>('list_windows_games');
    if(!Array.isArray(next?.games))throw new Error('Invalid Windows library response');
    if(!stopped){setWindows(old=>JSON.stringify(old)===JSON.stringify(next.games)?old:next.games);setWarnings(next.warnings??[]);setLoaded(true);setError('');}
   }catch(error){if(!stopped)setError(String(error));}
   finally{if(!stopped)timer=setTimeout(()=>void refresh(),5000);}
  };
  void refresh();return()=>{stopped=true;clearTimeout(timer);};
 },[props.refreshKey,retry]);
 useEffect(()=>{
  const sync=(event:StorageEvent)=>{if(event.key!==FAVORITES&&event.key!==null)return;try{setFavorites(readFavorites());setFavoriteError('');}catch{setFavoriteError('Saved favorites could not be read. They have been left unchanged.');}};
  window.addEventListener('storage',sync);return()=>window.removeEventListener('storage',sync);
 },[]);
 const toggleFavorite=(key:string)=>{
  try{const next=readFavorites();if(next.has(key))next.delete(key);else next.add(key);if(next.size>10000)throw new Error("Favorites limit reached");localStorage.setItem(FAVORITES,JSON.stringify([...next]));setFavorites(next);setFavoriteError('');}
  catch{setFavoriteError('Could not save favorites. Existing saved favorites have been left unchanged.');}
 };
 const items=useMemo(()=>libraryItems(props.steam,windows),[props.steam,windows]);
 const visible=useMemo(()=>items.filter(item=>{
  if(props.source!=='all'&&item.kind!==props.source)return false;
  const identity=item.kind==='steam'?item.game.appId:item.game.id;
  if(!`${item.name} ${item.kind} ${identity}`.toLocaleLowerCase().includes(props.query.trim().toLocaleLowerCase()))return false;
  if(props.filter==='favorites')return favorites.has(item.key);
  if(props.filter==='issues')return item.kind==='steam'?props.problemKeys.has(`${item.game.libraryPath}:${item.game.appId}`):needsReview(item.game);
  if(props.filter==='missing')return item.kind==='steam'&&!item.game.installPathExists;
  if(props.filter==='prefix')return item.kind==='steam'&&item.game.prefixExists;
  return true;
 }).sort((a,b)=>(props.sort==='recent'?b.recent-a.recent:0)||a.name.localeCompare(b.name)||a.key.localeCompare(b.key)),[items,props.source,props.query,props.filter,props.sort,props.problemKeys,favorites]);
 return <>
  <div className="pm-game-toolbar">
   <label className="pm-search"><Search size={19}/><input value={props.query} onChange={event=>props.onQuery(event.target.value)} placeholder="Search your games…" aria-label="Search games"/></label>
   <select aria-label="Game source" value={props.source} onChange={event=>props.onSource(event.target.value as LibrarySource)}><option value="all">Steam + Windows</option><option value="steam">Steam</option><option value="windows">Windows</option></select>
   <select aria-label="Filter games" value={props.filter} onChange={event=>props.onFilter(event.target.value as LibraryFilter)}><option value="all">All games</option><option value="favorites">Favorites</option><option value="issues">Needs attention</option><option value="missing">Missing Steam folder</option><option value="prefix">Steam Proton prefix</option></select>
   <select aria-label="Sort games" value={props.sort} onChange={event=>props.onSort(event.target.value as LibrarySort)}><option value="name">Name A–Z</option><option value="recent">Recently played</option></select>
   <span>{visible.length} games</span>
  </div>
  {favoriteError&&<p role="alert" className="pm-repair-danger">{favoriteError}</p>}
  {error&&<div role="alert" className="pm-repair-danger"><span>Could not refresh Windows games: {error}. Previously loaded games are kept.</span><button className="pm-button pm-button-secondary" onClick={()=>setRetry(value=>value+1)}>Retry Windows library</button></div>}
  {warnings.map(warning=><p role="alert" className="pm-repair-danger" key={warning}>{warning}</p>)}
  {props.sort==='recent'&&<p className="pm-library-note">Uses available Steam last-played dates and recorded Windows game launches. Unknown dates appear last.</p>}
  <section className="pm-section" aria-label="Unified game library">
   {!loaded&&!error&&props.source!=='steam'&&<p role="status">Loading Windows games…</p>}
   {props.steamLoading&&!props.scanAvailable&&props.source!=='windows'&&<p role="status">Loading Steam games…</p>}
   {visible.length?<PagedList key={`${props.query}:${props.filter}:${props.source}:${props.sort}`} items={visible} render={item=>{
    const source=item.kind==='steam'?'Steam':'Windows';const favorite=favorites.has(item.key);
    return <div className="pm-library-entry" key={item.key}>
     <button className="pm-game-row" aria-label={`Open ${item.name} (${source})`} onClick={()=>item.kind==='steam'?props.onSteam(item.game):props.onWindows(item.game)}>
      <GameIcon source={item.kind==='steam'?{kind:'steam',appId:item.game.appId,libraryPath:item.game.libraryPath}:{kind:'windows',id:item.game.id,executable:item.game.gameExe}}/>
      <span className="pm-game-title"><strong>{item.name}</strong><small>{source}{item.kind==='steam'?` · App ${item.game.appId}`:''}{props.sort==='recent'?item.recent?` · ${item.kind==='steam'?'Last played':'Last launched'} ${new Date(item.recent).toLocaleDateString()}`:' · Date unknown':''}</small></span>
      <span className="pm-game-issue">{item.kind==='steam'?props.steamStatus(item.game):<span className={`pm-tag ${needsReview(item.game)?'warning':'neutral'}`}>{windowsStatus(item.game)}</span>}</span><span className="pm-view-game" aria-hidden="true"><ArrowRight size={16}/></span>
     </button>
     <button className="pm-favorite" aria-label={`${favorite?'Remove':'Add'} ${item.name} (${source}) ${favorite?'from':'to'} favorites`} aria-pressed={favorite} onClick={()=>toggleFavorite(item.key)}><Star size={18} fill={favorite?'currentColor':'none'}/></button>
    </div>;
   }}/>:(loaded||props.source==='steam')&&(!props.steamLoading||props.scanAvailable||props.source==='windows')&&<div className="pm-empty"><strong>No matching games</strong><p>Change your search or filters, scan Steam again, or add a game from Windows Games.</p></div>}
  </section>
 </>;
}
