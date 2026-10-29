import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { useAutoFixControl } from '../src/useAutoFixControl';
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
const key='proton-medic.auto-log-fixes.enabled';
function Harness() {
 const control=useAutoFixControl();
 return <><input type="checkbox" aria-label="Automatic fixes" checked={control.enabled} disabled={control.pending} onChange={e=>void control.toggle(e.target.checked)}/><p>{control.message}</p></>;
}
beforeEach(()=>{invoke.mockReset();localStorage.clear();});
afterEach(cleanup);
it('waits for startup restoration before accepting toggle requests',async()=>{
 localStorage.setItem(key,'true');let finish!:(value:unknown)=>void;
 invoke.mockImplementation((command:string)=>command==='get_auto_fix_control'?Promise.resolve({enabled:false,revision:8}):new Promise(r=>{finish=r;}));
 render(<Harness/>);await act(async()=>{});
 const checkbox=screen.getByRole('checkbox') as HTMLInputElement;
 expect(checkbox.disabled).toBe(true);
 expect(invoke).toHaveBeenCalledWith('set_auto_fix_control',{enabled:true,revision:9});
 fireEvent.click(checkbox);
 expect(invoke.mock.calls.filter(call=>call[0]==='set_auto_fix_control')).toHaveLength(1);
 await act(async()=>finish({enabled:true,revision:9}));
 expect(checkbox.checked).toBe(true);expect(checkbox.disabled).toBe(false);
});
it('only announces read-only after the backend finishes an in-flight write',async()=>{
 localStorage.setItem(key,'true');let finish!:(value:unknown)=>void;
 invoke.mockImplementation((command:string)=>command==='get_auto_fix_control'?Promise.resolve({enabled:true,revision:4}):new Promise(r=>{finish=r;}));
 render(<Harness/>);await act(async()=>{});
 const checkbox=screen.getByRole('checkbox') as HTMLInputElement;
 await act(async()=>fireEvent.click(checkbox));
 expect(invoke).toHaveBeenCalledWith('set_auto_fix_control',{enabled:false,revision:5});
 expect(checkbox.checked).toBe(true);expect(checkbox.disabled).toBe(true);
 expect(screen.queryByText(/continues read-only/)).toBeNull();
 expect(localStorage.getItem(key)).toBe('true');
 expect(screen.getByText(/finishing any change already being saved/)).toBeTruthy();
 await act(async()=>finish({enabled:false,revision:5}));
 expect(checkbox.checked).toBe(false);expect(checkbox.disabled).toBe(false);
 expect(screen.getByText(/continues read-only/)).toBeTruthy();
 expect(localStorage.getItem(key)).toBe('false');
});
it('recovers the backend revision after a superseded setting request',async()=>{
 let reads=0;let writes=0;
 invoke.mockImplementation(async(command:string,args?:{enabled:boolean;revision:number})=>{
  if(command==='get_auto_fix_control')return {enabled:false,revision:reads++===0?0:7};
  if(++writes===1)throw new Error('Setting changed');
  return args;
 });
 render(<Harness/>);await act(async()=>{});
 const checkbox=screen.getByRole('checkbox') as HTMLInputElement;
 await act(async()=>fireEvent.click(checkbox));
 expect(checkbox.checked).toBe(false);
 expect(screen.getByText(/Could not change automatic fixes/)).toBeTruthy();
 await act(async()=>fireEvent.click(checkbox));
 expect(invoke).toHaveBeenLastCalledWith('set_auto_fix_control',{enabled:true,revision:8});
 expect(checkbox.checked).toBe(true);
});
