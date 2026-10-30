import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import App from '../src/App';
import { version } from '../package.json';

const { check, invoke } = vi.hoisted(() => ({ check: vi.fn(), invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
beforeEach(() => {
  check.mockReset(); invoke.mockReset(); localStorage.clear();
  invoke.mockImplementation(async (command: string) => {
    if (command === 'scan_steam') return { games: [], installations: [], libraries: [], compatibilityTools: [], warnings: [] };
    if (command === 'monitor_game_logs') return [];
    if (command === 'list_windows_games') return {games:[],runtimes:[],removedGames:[],warnings:[]};
    if (command === 'get_auto_fix_control') return {enabled:false,revision:0};
    return { games: [] };
  });
});
afterEach(cleanup);

it('shows the installed version and keeps a header update check alive across navigation', async () => {
  let finish!: (value: null) => void;
  check.mockImplementation(() => new Promise<null>(resolve => { finish = resolve; }));
  render(<App/>);
  expect(screen.getByText('Version ' + version)).toBeTruthy();
  expect(check).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', {name: 'Check for updates'}));
  await waitFor(() => expect(check).toHaveBeenCalledTimes(1));
  fireEvent.click(screen.getByRole('button', {name: 'Settings', exact: true}));
  expect((screen.getByRole('button', {name: 'Check for updates'}) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole('button', {name: 'Home', exact: true}));
  finish(null);
  await screen.findByText('You are up to date.');
  expect(screen.getAllByRole('button', {name: 'Check for updates'})).toHaveLength(1);
  expect(check).toHaveBeenCalledTimes(1);
});
it('places automatic fix controls before privacy and technical sections in Settings',async()=>{
 render(<App/>);fireEvent.click(screen.getByRole('button',{name:'Settings',exact:true}));
 const monitor=screen.getByRole('region',{name:'Automatic game log reader'});
 expect(monitor.compareDocumentPosition(screen.getByText('Privacy & safety')) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
 expect(screen.getAllByRole('checkbox')).toHaveLength(1);
});

it('opens the library by default and keeps every tool reachable', async () => {
 render(<App/>);
 expect(screen.getByRole('heading', {name:'My Games',level:1})).toBeTruthy();
 expect(screen.getByRole('button', {name:'Library',exact:true}).getAttribute('aria-current')).toBe('page');
 await screen.findByRole('button', {name:'Add game',exact:true});
 fireEvent.click(screen.getByRole('button', {name:'Add game',exact:true}));
 expect(await screen.findByRole('heading', {name:'Windows Games',level:1})).toBeTruthy();
 fireEvent.click(screen.getByRole('button', {name:'Library',exact:true}));
 expect(screen.getByRole('heading', {name:'My Games',level:1})).toBeTruthy();
 for(const label of ['Home','Windows Games','Proton versions','Diagnostics','System Check','Settings']) {
  expect(screen.getByRole('button', {name:label,exact:true})).toBeTruthy();
 }
});

it('retains the library page across game details and resets it when searching', async () => {
  const games=Array.from({length:95},(_,i)=>({appId:String(i),name:`Game ${String(i).padStart(3,'0')}`,libraryPath:'/fixture',installPath:'/fixture/game',installPathExists:true,manifestPath:'/fixture/manifest',prefixPath:'/fixture/prefix',prefixExists:true,buildId:null}));
  invoke.mockImplementation(async(command:string)=>{
    if(command==='scan_steam')return {games,installations:[],libraries:[],compatibilityTools:[],warnings:[]};
    if(command==='list_windows_games')return {games:[],runtimes:[],removedGames:[],warnings:[]};
    if(command==='get_auto_fix_control')return {enabled:false,revision:0};
    if(['monitor_game_logs','list_fix_sessions','list_repair_backups','list_game_protons'].includes(command))return [];
    if(command==='analyze_diagnostics'||command==='recommend_proton')return {games:[]};
    return null;
  });
  render(<App/>);
  fireEvent.click(await screen.findByRole('button',{name:'Next',exact:true}));
  fireEvent.click(screen.getByRole('button',{name:'Game settings for Game 040 (Steam)'}));
  fireEvent.click(screen.getByRole('button',{name:'Back to My Games'}));
  expect(await screen.findByRole('button',{name:'Open Game 040 (Steam)'})).toBeTruthy();
  expect(screen.getByText('Page 2 of 3')).toBeTruthy();
  fireEvent.change(screen.getByRole('textbox',{name:'Search games'}),{target:{value:'Game 0'}});
  expect(screen.getByText('Page 1 of 3')).toBeTruthy();
  expect(screen.getByRole('button',{name:'Open Game 000 (Steam)'})).toBeTruthy();
});
