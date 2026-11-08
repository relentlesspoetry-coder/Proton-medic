import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { ResourceMonitor } from '../src/ResourceMonitor';
import type { Game } from '../src/WindowsGames';
const {invoke}=vi.hoisted(()=>({invoke:vi.fn()}));
vi.mock('@tauri-apps/api/core',()=>({invoke}));
const game:Game={id:'123',name:'Fixture',runtimeId:'wine',gameExe:'/game.exe',state:'game-running',exitCode:null,message:'Running',directory:'/managed/123',windowsDrive:'/prefix/drive_c',prefixDirectory:'/prefix',logDirectory:'/logs/1-100'};
const sample={runId:'1-100',capturedAt:1700000000000,cpuPercent:150,memoryBytes:16*1024**2,peakMemoryBytes:20*1024**2,processCount:2,partial:false};
beforeEach(()=>{vi.useFakeTimers();vi.setSystemTime(sample.capturedAt);invoke.mockReset();invoke.mockImplementation(async()=>({status:'live',sample:{...sample,capturedAt:Date.now()},message:''}));});
afterEach(()=>{cleanup();vi.useRealTimers();Object.defineProperty(document,'hidden',{configurable:true,value:false});});
it('is off by default and samples only when enabled',async()=>{
 const toggle=vi.fn();render(<ResourceMonitor game={game} enabled={false} onToggle={toggle}/>);expect(invoke).not.toHaveBeenCalled();fireEvent.click(screen.getByRole('button',{name:'Monitor resources'}));expect(toggle).toHaveBeenCalledWith(true);
});
it('shows CPU above one core, memory totals, peak and child-process count',async()=>{
 render(<ResourceMonitor game={game} enabled onToggle={()=>{}}/>);await act(async()=>{});expect(screen.getByText('150.0%')).toBeTruthy();expect(screen.getByText('16.0 MiB')).toBeTruthy();expect(screen.getByText('20.0 MiB')).toBeTruthy();expect(screen.getByText('2')).toBeTruthy();
});
it('serializes polls and releases the request when disabled',async()=>{
 let resolve!:(value:unknown)=>void;invoke.mockImplementationOnce(()=>new Promise(done=>{resolve=done;}));const view=render(<ResourceMonitor game={game} enabled onToggle={()=>{}}/>);
 await act(async()=>{vi.advanceTimersByTime(6000);});expect(invoke).toHaveBeenCalledTimes(1);
 view.rerender(<ResourceMonitor game={game} enabled={false} onToggle={()=>{}}/>);await act(async()=>resolve({status:'live',sample,message:''}));
 expect(invoke).toHaveBeenCalledWith('stop_windows_resource_monitor',{id:'123',runId:'1-100'});expect(screen.queryByText('150.0%')).toBeNull();
 await act(async()=>{vi.advanceTimersByTime(10000);});expect(invoke.mock.calls.filter(c=>c[0]==='sample_windows_resources')).toHaveLength(1);
});
it('replaces stale readings with unavailable values instead of zero or old data',async()=>{
 render(<ResourceMonitor game={game} enabled onToggle={()=>{}}/>);await act(async()=>{});invoke.mockResolvedValueOnce({status:'stale',sample:null,message:'Waiting for the launcher'});
 await act(async()=>{vi.advanceTimersByTime(2000);});expect(screen.queryByText('150.0%')).toBeNull();expect(screen.getByText('Waiting for the launcher')).toBeTruthy();expect(screen.getAllByText('—')).toHaveLength(4);
});
it('pauses when hidden and resumes when visible',async()=>{
 render(<ResourceMonitor game={game} enabled onToggle={()=>{}}/>);await act(async()=>{});
 await act(async()=>{Object.defineProperty(document,'hidden',{configurable:true,value:true});document.dispatchEvent(new Event('visibilitychange'));});
 expect(screen.getByText(/Monitoring paused/)).toBeTruthy();invoke.mockClear();await act(async()=>{vi.advanceTimersByTime(8000);});expect(invoke).not.toHaveBeenCalled();
 await act(async()=>{Object.defineProperty(document,'hidden',{configurable:true,value:false});document.dispatchEvent(new Event('visibilitychange'));});expect(invoke).toHaveBeenCalledWith('sample_windows_resources',{id:'123',runId:'1-100'});
});
it('stops after game completion and never treats it as zero usage',async()=>{
 const view=render(<ResourceMonitor game={game} enabled onToggle={()=>{}}/>);await act(async()=>{});view.rerender(<ResourceMonitor game={{...game,state:'game-exited'}} enabled onToggle={()=>{}}/>);await act(async()=>{});
 expect(screen.getByText('Run ended. Monitoring stopped.')).toBeTruthy();expect(screen.queryByText('150.0%')).toBeNull();expect(invoke).toHaveBeenCalledWith('stop_windows_resource_monitor',{id:'123',runId:'1-100'});
});
it('shows partial readings and renders a bounded memory trend',async()=>{
 invoke.mockResolvedValue({status:'live',sample:{...sample,partial:true},message:''});render(<ResourceMonitor game={game} enabled onToggle={()=>{}}/>);await act(async()=>{});
 expect(screen.getByText(/totals are incomplete/)).toBeTruthy();
 for(let i=0;i<65;i++){invoke.mockImplementation(async()=>({status:'live',sample:{...sample,capturedAt:Date.now(),memoryBytes:sample.memoryBytes+i},message:''}));await act(async()=>{vi.advanceTimersByTime(2000);});}
 expect(screen.getByRole('img',{name:/Recent RAM usage/})).toBeTruthy();expect(screen.getByText(/last 60 samples/)).toBeTruthy();
});
it('does not retain old readings after a sample error and can recover',async()=>{
 render(<ResourceMonitor game={game} enabled onToggle={()=>{}}/>);await act(async()=>{});invoke.mockRejectedValueOnce(new Error('cannot read process'));
 await act(async()=>{vi.advanceTimersByTime(2000);});expect(screen.getByText(/cannot read process/)).toBeTruthy();expect(screen.queryByText('150.0%')).toBeNull();
 await act(async()=>{vi.advanceTimersByTime(2000);});expect(screen.getByText('150.0%')).toBeTruthy();
});

it('expires a displayed sample even while the next request is stalled',async()=>{
 render(<ResourceMonitor game={game} enabled onToggle={()=>{}}/>);await act(async()=>{});invoke.mockImplementationOnce(()=>new Promise(()=>{}));
 await act(async()=>{vi.advanceTimersByTime(5000);});expect(screen.queryByText('150.0%')).toBeNull();expect(screen.getByText('Waiting for a fresh resource reading…')).toBeTruthy();
});

it('shows a fresh first sample before a CPU delta is available',async()=>{
 invoke.mockResolvedValueOnce({status:'live',sample:{...sample,cpuPercent:null},message:''});
 render(<ResourceMonitor game={game} enabled onToggle={()=>{}}/>);await act(async()=>{});
 expect(screen.getByText('16.0 MiB')).toBeTruthy();expect(screen.getByText('20.0 MiB')).toBeTruthy();expect(screen.getAllByText('—')).toHaveLength(1);
});

it.each([
 ['expired',{capturedAt:sample.capturedAt-4001}],
 ['at the expiry boundary',{capturedAt:sample.capturedAt-4000}],
 ['future',{capturedAt:sample.capturedAt+1}],
 ['wrong run',{runId:'1-101'}],
 ['invalid timestamp',{capturedAt:NaN}],
])('does not show a %s sample reported as live',async(_description,changes)=>{
 invoke.mockResolvedValueOnce({status:'live',sample:{...sample,...changes},message:''});
 render(<ResourceMonitor game={game} enabled onToggle={()=>{}}/>);await act(async()=>{});
 expect(screen.queryByText('150.0%')).toBeNull();expect(screen.getAllByText('—')).toHaveLength(4);expect(screen.getByText('Waiting for a fresh resource reading…')).toBeTruthy();
 await act(async()=>{vi.advanceTimersByTime(2000);});expect(screen.getByText('150.0%')).toBeTruthy();
});

it('rejects a formerly live reply that arrives after its sample expires',async()=>{
 let resolve!:(value:unknown)=>void;invoke.mockImplementationOnce(()=>new Promise(done=>{resolve=done;}));
 render(<ResourceMonitor game={game} enabled onToggle={()=>{}}/>);
 await act(async()=>{vi.advanceTimersByTime(12000);resolve({status:'live',sample,message:''});});
 expect(screen.queryByText('150.0%')).toBeNull();expect(screen.getAllByText('—')).toHaveLength(4);
});

it('expires a delayed but fresh reply after only its remaining lifetime',async()=>{
 let resolve!:(value:unknown)=>void;invoke.mockImplementationOnce(()=>new Promise(done=>{resolve=done;}));
 render(<ResourceMonitor game={game} enabled onToggle={()=>{}}/>);
 await act(async()=>{vi.advanceTimersByTime(3500);resolve({status:'live',sample,message:''});});
 expect(screen.getByText('150.0%')).toBeTruthy();
 await act(async()=>{vi.advanceTimersByTime(499);});expect(screen.getByText('150.0%')).toBeTruthy();
 await act(async()=>{vi.advanceTimersByTime(1);});expect(screen.queryByText('150.0%')).toBeNull();
});

it('does not extend the deadline or duplicate the trend for a repeated sample',async()=>{
 invoke.mockResolvedValueOnce({status:'live',sample,message:''}).mockResolvedValueOnce({status:'live',sample,message:''}).mockImplementationOnce(()=>new Promise(()=>{}));
 render(<ResourceMonitor game={game} enabled onToggle={()=>{}}/>);await act(async()=>{});
 await act(async()=>{vi.advanceTimersByTime(2000);});expect(screen.getByText('150.0%')).toBeTruthy();expect(screen.queryByRole('img',{name:/Recent RAM usage/})).toBeNull();
 await act(async()=>{vi.advanceTimersByTime(1999);});expect(screen.getByText('150.0%')).toBeTruthy();
 await act(async()=>{vi.advanceTimersByTime(1);});expect(screen.queryByText('150.0%')).toBeNull();
});

it('ignores a pending old-run reply after monitoring switches to a new run',async()=>{
 let resolve!:(value:unknown)=>void;invoke.mockImplementationOnce(()=>new Promise(done=>{resolve=done;}));
 const view=render(<ResourceMonitor game={game} enabled onToggle={()=>{}}/>);
 invoke.mockResolvedValue({status:'waiting',sample:null,message:'Waiting for the new run'});
 view.rerender(<ResourceMonitor game={{...game,logDirectory:'/logs/1-101'}} enabled onToggle={()=>{}}/>);
 await act(async()=>{resolve({status:'live',sample,message:''});});
 expect(screen.queryByText('150.0%')).toBeNull();expect(screen.getByText('Waiting for the new run')).toBeTruthy();
 expect(invoke).toHaveBeenCalledWith('sample_windows_resources',{id:'123',runId:'1-101'});
});
