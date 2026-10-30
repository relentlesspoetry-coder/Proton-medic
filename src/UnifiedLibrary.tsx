import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ArrowRight, Play, Plus, RefreshCw, Search, Star, X } from 'lucide-react';
import type { Game as SteamGame } from './types';
import type { Game as WindowsGame, Library } from './WindowsGames';
import { GameIcon } from './GameIcon';
import { GameCover } from './GameCover';
import { LaunchTest } from './LaunchTest';
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
 page?:number;onPageChange?:(page:number)=>void;
 onAddGame?:()=>void;onScan?:()=>void;scanning?:boolean;
 onSteam:(game:SteamGame)=>void;onWindows:(game:WindowsGame)=>void;steamStatus:(game:SteamGame)=>ReactNode;
}
export function UnifiedLibrary(props:Props){
 const [steamLaunching,setSteamLaunching]=useState(false);
 const [launchGame,setLaunchGame]=useState<SteamGame|null>(null);
 const [launching,setLaunching]=useState<string|null>(null);
 const launchLock=useRef(false);
 const launchPanel=useRef<HTMLElement>(null);
 const [launchError,setLaunchError]=useState('');
 const [runtimes,setRuntimes]=useState<Library['runtimes']>([]);
 useEffect(()=>{if(launchGame)launchPanel.current?.focus();},[launchGame]);
 const [windowVisible,setWindowVisible]=useState(!document.hidden);
 const [windows,setWindows]=useState<WindowsGame[]>([]);
 const [warnings,setWarnings]=useState<string[]>([]);
 const [loaded,setLoaded]=useState(false);const [error,setError]=useState('');const [retry,setRetry]=useState(0);
 const [favorites,setFavorites]=useState(()=>{try{return readFavorites();}catch{return new Set<string>();}});
 const [favoriteError,setFavoriteError]=useState(()=>{try{readFavorites();return '';}catch{return 'Saved favorites could not be read. They have been left unchanged.';}});
 useEffect(()=>{
  const change=()=>setWindowVisible(!document.hidden);
  document.addEventListener('visibilitychange',change);
  return()=>document.removeEventListener('visibilitychange',change);
 },[]);
 useEffect(()=>{
  if(!windowVisible)return;
  let stopped=false;let timer:ReturnType<typeof setTimeout>;
  const refresh=async()=>{
   try{
    const next=await invoke<Library>('list_windows_games');
    if(!Array.isArray(next?.games))throw new Error('Invalid Windows library response');
    if(!stopped){setWindows(old=>JSON.stringify(old)===JSON.stringify(next.games)?old:next.games);setWarnings(next.warnings??[]);setRuntimes(next.runtimes??[]);setLoaded(true);setError('');}
   }catch(error){if(!stopped)setError(String(error));}
   finally{if(!stopped)timer=setTimeout(()=>void refresh(),5000);}
  };
  void refresh();return()=>{stopped=true;clearTimeout(timer);};
 },[windowVisible,props.refreshKey,retry]);
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
 useEffect(()=>{
  if((loaded||props.source==='steam')&&visible.length===0&&props.page)props.onPageChange?.(0);
 },[loaded,props.source,visible.length,props.page,props.onPageChange]);
 const recent=useMemo(()=>visible.filter(item=>item.recent>0).sort((a,b)=>b.recent-a.recent||a.key.localeCompare(b.key)).slice(0,3),[visible]);
 const openItem=(item:Item)=>item.kind==='steam'?props.onSteam(item.game):props.onWindows(item.game);
 const sourceName=(item:Item)=>item.kind==='steam'?'Steam':'Windows';
 const iconSource=(item:Item)=>item.kind==='steam'?{kind:'steam' as const,appId:item.game.appId,libraryPath:item.game.libraryPath}:{kind:'windows' as const,id:item.game.id,executable:item.game.gameExe};
 const runtimeName=(item:Item)=>item.kind==='steam'?(item.game.configuredTool||'Steam default'):(runtimes.find(runtime=>runtime.id===item.game.runtimeId)?.name||'Saved runtime');
 const cannotPlay=(item:Item)=>steamLaunching||!!launching||(item.kind==='steam'?!item.game.installPathExists:!item.game.gameExe||item.game.state.endsWith('running')||item.game.state==='session-ended');
 const play=async(item:Item)=>{
  if(launchLock.current||cannotPlay(item))return;
  setLaunchError('');
  if(item.kind==='steam'){setLaunchGame(item.game);return;}
  launchLock.current=true;setLaunching(item.key);
  try{
   const next=await invoke<WindowsGame>('launch_windows_game',{id:item.game.id,confirmed:true});
   setWindows(games=>games.map(game=>game.id===next.id?next:game));setRetry(value=>value+1);
  }catch(error){setLaunchError(`Could not launch ${item.name}: ${String(error)}`);}
  finally{launchLock.current=false;setLaunching(null);}
 };
 return <>
  <div className="pm-library-toolbar">
   <label className="pm-search"><Search size={19}/><input value={props.query} onChange={event=>props.onQuery(event.target.value)} placeholder="Search games…" aria-label="Search games"/></label>
   <div className="pm-library-sources" role="group" aria-label="Game source">{(['all','steam','windows'] as const).map(source=><button key={source} type="button" aria-pressed={props.source===source} onClick={()=>props.onSource(source)}>{source==='all'?'All':source==='steam'?'Steam':'Windows'}</button>)}</div>
   {props.onAddGame&&<button className="pm-button pm-button-primary" onClick={props.onAddGame}><Plus size={18}/>Add game</button>}
  </div>
  <div className="pm-library-options">
   <select aria-label="Filter games" value={props.filter} onChange={event=>props.onFilter(event.target.value as LibraryFilter)}><option value="all">All games</option><option value="favorites">Favorites</option><option value="issues">Needs attention</option><option value="missing">Missing Steam folder</option><option value="prefix">Steam Proton prefix</option></select>
   <select aria-label="Sort games" value={props.sort} onChange={event=>props.onSort(event.target.value as LibrarySort)}><option value="name">Name A–Z</option><option value="recent">Recently played</option></select>
   <span>{visible.length} games</span>
   {props.onScan&&<button className="pm-button pm-button-secondary" disabled={props.scanning} onClick={props.onScan}><RefreshCw size={16}/>{props.scanning?'Scanning…':'Scan library'}</button>}
  </div>
  {favoriteError&&<p role="alert" className="pm-repair-danger">{favoriteError}</p>}
  {error&&<div role="alert" className="pm-repair-danger"><span>Could not refresh Windows games: {error}. Previously loaded games are kept.</span><button className="pm-button pm-button-secondary" onClick={()=>setRetry(value=>value+1)}>Retry Windows library</button></div>}
  {warnings.map(warning=><p role="alert" className="pm-repair-danger" key={warning}>{warning}</p>)}
  {launchError&&<p role="alert" className="pm-repair-danger">{launchError}</p>}
  {launchGame&&<section ref={launchPanel} tabIndex={-1} className="pm-section pm-library-launch" aria-label={`Launch ${launchGame.name}`}><div className="pm-section-heading"><h2>Launch {launchGame.name}</h2><button className="pm-button pm-button-secondary" aria-label="Close launch options" disabled={steamLaunching} onClick={()=>setLaunchGame(null)}><X size={18}/></button></div><LaunchTest key={steamLibraryKey(launchGame)} game={launchGame} onBusyChange={setSteamLaunching}/></section>}
  {props.sort==='recent'&&<p className="pm-library-note">Uses available Steam last-played dates and recorded Windows game launches. Unknown dates appear last.</p>}
  <section className="pm-library-grid-section" aria-label="Unified game library">
   {!loaded&&!error&&props.source!=='steam'&&<p role="status">Loading Windows games…</p>}
   {props.steamLoading&&!props.scanAvailable&&props.source!=='windows'&&<p role="status">Loading Steam games…</p>}
   {visible.length?<PagedList page={props.page} onPageChange={props.onPageChange} clampPage={loaded||props.source==='steam'} className="pm-library-grid" key={`${props.query}:${props.filter}:${props.source}:${props.sort}`} items={visible} render={item=>{
    const source=sourceName(item);const favorite=favorites.has(item.key);
    return <article className="pm-library-card" key={item.key}>
     <GameCover name={item.name} source={{source:item.kind,id:item.kind==='steam'?item.game.appId:item.game.id,libraryPath:item.kind==='steam'?item.game.libraryPath:null}} onOpen={()=>openItem(item)}/>
     <div className="pm-library-card-body">
      <strong className="pm-library-card-title">{item.name}</strong>
      <div className="pm-library-card-meta"><span>{source} · {runtimeName(item)}</span>{props.sort==='recent'&&<small>{item.recent?`${item.kind==='steam'?'Last played':'Last launched'} ${new Date(item.recent).toLocaleDateString()}`:'Date unknown'}</small>}</div>
      <div className="pm-library-card-status">{item.kind==='steam'?props.steamStatus(item.game):<span className={`pm-tag ${needsReview(item.game)?'warning':'neutral'}`}>{windowsStatus(item.game)}</span>}</div>
      <div className="pm-library-card-actions">
       <button className="pm-button pm-button-primary" aria-label={`Play ${item.name} (${source})`} disabled={cannotPlay(item)} onClick={()=>void play(item)}><Play size={17}/>{launching===item.key?'Launching…':'Play'}</button>
       <button className="pm-favorite" aria-label={`${favorite?'Remove':'Add'} ${item.name} (${source}) ${favorite?'from':'to'} favorites`} aria-pressed={favorite} onClick={()=>toggleFavorite(item.key)}><Star size={18} fill={favorite?'currentColor':'none'}/></button>
       <button className="pm-button pm-button-secondary" aria-label={`Game settings for ${item.name} (${source})`} onClick={()=>openItem(item)}><ArrowRight size={17}/></button>
      </div>
     </div>
    </article>;
   }}/>:(loaded||props.source==='steam')&&(!props.steamLoading||props.scanAvailable||props.source==='windows')&&<div className="pm-empty"><strong>No matching games</strong><p>Change your search or filters, scan Steam again, or add a game from Windows Games.</p></div>}
  </section>
  {recent.length>0&&<section className="pm-section pm-library-recent" aria-label="Recently played"><div className="pm-section-heading"><h2>Recently played</h2><button className="pm-button pm-button-secondary" onClick={()=>props.onSort('recent')}>View all<ArrowRight size={16}/></button></div><div className="pm-library-recent-list">{recent.map(item=><div key={item.key} className="pm-library-recent-row"><GameIcon source={iconSource(item)}/><div className="pm-game-title"><strong>{item.name}</strong><small>{sourceName(item)} · {runtimeName(item)}</small></div><time dateTime={new Date(item.recent).toISOString()}>{new Date(item.recent).toLocaleDateString()}</time><button className="pm-button pm-button-secondary" aria-label={`View recent ${item.name} (${sourceName(item)})`} onClick={()=>openItem(item)}><ArrowRight size={17}/></button></div>)}</div></section>}
 </>;
}
