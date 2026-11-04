import { afterEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import App from '../src/App';
const { invoke }=vi.hoisted(()=>({invoke:vi.fn()}));
vi.mock('@tauri-apps/api/core',()=>({invoke}));
vi.mock('@tauri-apps/plugin-updater',()=>({check:vi.fn()}));
afterEach(cleanup);
it('clears failed diagnostics after a successful Check Logs & System retry',async()=>{
 let count=0;
 invoke.mockImplementation(async(command:string)=>{
  if(command==='scan_steam')return {games:[],installations:[],libraries:[],compatibilityTools:[],warnings:[]};
  if(command==='list_windows_games')return {games:[],runtimes:[],removedGames:[],warnings:[]};
  if(command==='get_auto_fix_control')return {enabled:false,revision:0};
  if(['monitor_game_logs','list_fix_sessions'].includes(command))return [];
  if(command==='analyze_diagnostics'){
   if(++count===1)throw new Error('temporary fixture failure');
   return {games:[],system:{os:'Recovered fixture OS',kernel:'fixture',arch:'x86_64',gpuDevices:[],icdManifests:[],notes:[],vulkaninfoPath:null}};
  }
  if(command==='recommend_proton')return {games:[]};
  return null;
 });
 render(<App/>);
 await screen.findByText('Games were found, but logs could not be analyzed: Error: temporary fixture failure');
 fireEvent.click(screen.getByRole('button',{name:'System Check',exact:true}));
 fireEvent.click(screen.getByRole('button',{name:'Check Logs & System'}));
 await screen.findByText('Recovered fixture OS');
 expect(count).toBe(2);
 expect(screen.queryByText('Games were found, but logs could not be analyzed: Error: temporary fixture failure')).toBeNull();
});
it('preserves selected game when a requested scan fails and allows retry',async()=>{
 let scanCount=0;
 const game={appId:'123',name:'Retained Game',libraryPath:'/fixture',installPath:'/fixture/game',installPathExists:true,manifestPath:'/fixture/manifest',prefixPath:'/fixture/prefix',prefixExists:true,buildId:null};
 invoke.mockImplementation(async(command:string)=>{
  if(command==='scan_steam'){
   if(++scanCount===2)throw new Error('scan fixture failure');
   return {games:[game],installations:[],libraries:[],compatibilityTools:[],warnings:[]};
  }
  if(command==='list_windows_games')return {games:[],runtimes:[],removedGames:[],warnings:[]};
  if(command==='get_auto_fix_control')return {enabled:false,revision:0};
  if(['monitor_game_logs','list_fix_sessions','list_repair_backups','list_game_protons'].includes(command))return [];
  if(command==='analyze_diagnostics'||command==='recommend_proton')return {games:[]};
  return null;
 });
 render(<App/>);
 fireEvent.click(await screen.findByRole('button',{name:'Game settings for Retained Game (Steam)'}));
 fireEvent.click(await screen.findByRole('button',{name:'Scan Again'}));
 await screen.findByText('Could not scan Steam libraries: Error: scan fixture failure');
 expect(screen.getByRole('heading',{name:'Retained Game',exact:true})).toBeTruthy();
 fireEvent.click(screen.getByRole('button',{name:'Scan Again'}));
 await waitFor(()=>expect(scanCount).toBe(3));
 await waitFor(()=>expect(screen.queryByText('Could not scan Steam libraries: Error: scan fixture failure')).toBeNull());
 expect(screen.getByRole('heading',{name:'Retained Game',exact:true})).toBeTruthy();
});
it('preserves a newer advisor error when diagnostics retry succeeds',async()=>{
 let count=0;
 let rejectAdvice!:(e:Error)=>void;
 let finishDiagnostics!:(value:unknown)=>void;
 const recovered={games:[],system:{os:'Recovered with advisor error',kernel:'fixture',arch:'x86_64',gpuDevices:[],icdManifests:[],notes:[],vulkaninfoPath:null}};
 invoke.mockImplementation((command:string)=>{
  if(command==='scan_steam')return Promise.resolve({games:[],installations:[],libraries:[],compatibilityTools:[],warnings:[]});
  if(command==='list_windows_games')return Promise.resolve({games:[],runtimes:[],removedGames:[],warnings:[]});
  if(command==='get_auto_fix_control')return Promise.resolve({enabled:false,revision:0});
  if(['monitor_game_logs','list_fix_sessions'].includes(command))return Promise.resolve([]);
  if(command==='analyze_diagnostics'){
   if(++count===1)return Promise.reject(new Error('temporary diagnostics failure'));
   return new Promise(resolve=>{finishDiagnostics=resolve;});
  }
  if(command==='recommend_proton')return new Promise((_,reject)=>{rejectAdvice=reject;});
  return Promise.resolve(null);
 });
 render(<App/>);
 await screen.findByText('Games were found, but logs could not be analyzed: Error: temporary diagnostics failure');
 fireEvent.click(screen.getByRole('button',{name:'System Check',exact:true}));
 fireEvent.click(screen.getByRole('button',{name:'Check Logs & System'}));
 await waitFor(()=>expect(count).toBe(2));
 rejectAdvice(new Error('advisor fixture failure'));
 await screen.findByText('Could not recommend a Proton version: Error: advisor fixture failure');
 finishDiagnostics(recovered);
 await screen.findByText('Recovered with advisor error');
 expect(screen.getByText('Could not recommend a Proton version: Error: advisor fixture failure')).toBeTruthy();
});
