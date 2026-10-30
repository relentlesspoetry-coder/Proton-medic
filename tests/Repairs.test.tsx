import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { RepairTools } from '../src/Repairs';
const {invoke}=vi.hoisted(()=>({invoke:vi.fn()}));
vi.mock('@tauri-apps/api/core',()=>({invoke}));
const game={appId:'123',name:'Game',libraryPath:'/Steam',installPath:'/game',installPathExists:true,manifestPath:'/manifest',prefixPath:'/prefix',prefixExists:true,buildId:null};
const plan={appId:'123',libraryPath:'/Steam',gameName:'Game',action:'shader-cache',title:'Refresh shader cache',explanation:'Archive cache',sourcePath:'/Steam/shadercache/123',backupRoot:'/Steam/backups',risk:'moderate',canApply:true,blocker:null};
const backup={name:'shader-cache-123-1',appId:'123',libraryPath:'/Steam',action:'shader-cache',backupPath:'/Steam/backups/one',originalPath:'/Steam/shadercache/123',restoredTargetExists:true};
const start=async()=>act(async()=>{render(<RepairTools game={game} onChanged={vi.fn()}/>);});
beforeEach(()=>{invoke.mockReset();invoke.mockResolvedValue([])});afterEach(cleanup);
it('clears a backup-read error after a successful refresh',async()=>{
 invoke.mockRejectedValueOnce('Temporary disk failure');await start();
 expect(screen.getByRole('alert').textContent).toContain('Could not read saved backups');
 fireEvent.click(screen.getByText(/Saved backups/));
 invoke.mockResolvedValueOnce([backup]);
 await act(async()=>{fireEvent.click(screen.getByRole('button',{name:'Refresh backups'}));});
 expect(screen.getByText('shader-cache-123-1')).toBeTruthy();
 expect(screen.queryByRole('alert')).toBeNull();
});
it('does not apply repairs until confirmation and carries exact game identity',async()=>{
 await start();invoke.mockResolvedValueOnce(plan);
 await act(async()=>{fireEvent.click(screen.getByRole('button',{name:/Refresh shader cache/}));});
 const apply=screen.getByRole('button',{name:'Archive & continue'}) as HTMLButtonElement;
 expect(apply.disabled).toBe(true);fireEvent.click(apply);
 expect(invoke.mock.calls.filter(([c])=>c==='apply_repair')).toHaveLength(0);
 fireEvent.click(screen.getByRole('checkbox'));
 invoke.mockResolvedValueOnce({message:'Archived',backup,displacedBackup:null});
 await act(async()=>{fireEvent.click(apply);});
 expect(invoke).toHaveBeenCalledWith('apply_repair',{appId:'123',libraryPath:'/Steam',confirmed:true,action:'shader-cache'});
});
it('keeps a blocked repair disabled',async()=>{
 await start();invoke.mockResolvedValueOnce({...plan,canApply:false,blocker:'Close Steam first'});
 await act(async()=>{fireEvent.click(screen.getByRole('button',{name:/Refresh shader cache/}));});
 expect((screen.getByRole('checkbox') as HTMLInputElement).disabled).toBe(true);
 expect((screen.getByRole('button',{name:'Archive & continue'}) as HTMLButtonElement).disabled).toBe(true);
 expect(screen.getByRole('alert').textContent).toContain('Close Steam first');
});

it('preserves operation errors while recovering from a backup-list failure',async()=>{
 await start();invoke.mockResolvedValueOnce(plan);
 await act(async()=>{fireEvent.click(screen.getByRole('button',{name:/Refresh shader cache/}));});
 fireEvent.click(screen.getByRole('checkbox'));
 invoke.mockRejectedValueOnce('Archive failed');
 await act(async()=>{fireEvent.click(screen.getByRole('button',{name:'Archive & continue'}));});
 expect(screen.getByRole('alert').textContent).toContain('Operation failed');
 fireEvent.click(screen.getByText(/Saved backups/));
 invoke.mockRejectedValueOnce('Temporary disk failure');
 await act(async()=>{fireEvent.click(screen.getByRole('button',{name:'Refresh backups'}));});
 expect(screen.getAllByRole('alert')).toHaveLength(2);
 invoke.mockResolvedValueOnce([backup]);
 await act(async()=>{fireEvent.click(screen.getByRole('button',{name:'Refresh backups'}));});
 expect(screen.getByText('shader-cache-123-1')).toBeTruthy();
 expect(screen.getByRole('alert').textContent).toContain('Archive failed');
 expect(screen.queryByText(/Could not read saved backups/)).toBeNull();
 expect((screen.getByRole('checkbox') as HTMLInputElement).checked).toBe(false);
});
