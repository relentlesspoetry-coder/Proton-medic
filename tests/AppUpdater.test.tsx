import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { AppUpdater } from '../src/AppUpdater';
const { check, invoke } = vi.hoisted(() => ({ check: vi.fn(), invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check }));
beforeEach(() => { check.mockReset(); invoke.mockReset(); invoke.mockResolvedValue([]); });
beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute('open', ''); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute('open'); };
});
afterEach(cleanup);
function update() { return {version:'0.6.6',currentVersion:'0.6.5',body:'Release notes',close:vi.fn().mockResolvedValue(undefined),downloadAndInstall:vi.fn().mockResolvedValue(undefined)}; }
async function findUpdate(value = update()) {
  check.mockResolvedValue(value); render(<AppUpdater/>);
  fireEvent.click(screen.getByRole('button',{name:'Check for updates'}));
  await screen.findByText(/Version 0.6.6 is available/); return value;
}
it('never checks automatically and reports no update accurately',async()=>{
  check.mockResolvedValue(null);render(<AppUpdater/>);expect(check).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button',{name:'Check for updates'}));
  await screen.findByText('You are up to date.');expect(check).toHaveBeenCalledWith({timeout:20000});
});
it('requires approval before downloading or installing',async()=>{
  const value=await findUpdate();expect(value.downloadAndInstall).not.toHaveBeenCalled();
  expect((screen.getByRole('button',{name:'Download & install 0.6.6'}) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole('checkbox'));fireEvent.click(screen.getByRole('button',{name:'Download & install 0.6.6'}));
  await screen.findByText(/Update installed. Reopening Proton Medic/);expect(value.downloadAndInstall).toHaveBeenCalledTimes(1);
  await waitFor(()=>expect(invoke).toHaveBeenCalledWith('restart_after_update',{version:'0.6.6'}));
});
it('does not call a failed check up to date',async()=>{
  check.mockRejectedValue(new Error('404'));render(<AppUpdater/>);
  fireEvent.click(screen.getByRole('button',{name:'Check for updates'}));
  expect(await screen.findByRole('alert')).toHaveProperty('textContent',expect.stringContaining('Could not check for updates'));
  expect(screen.queryByText('You are up to date.')).toBeNull();
});
it('shows a signature or installer failure without a success claim',async()=>{
  const value=update();value.downloadAndInstall.mockRejectedValue(new Error('Signature verification failed'));
  await findUpdate(value);fireEvent.click(screen.getByRole('checkbox'));fireEvent.click(screen.getByRole('button',{name:'Download & install 0.6.6'}));
  expect(await screen.findByRole('alert')).toHaveProperty('textContent',expect.stringContaining('Signature verification failed'));
  expect(screen.queryByText(/Update installed\./)).toBeNull();expect((screen.getByRole('checkbox') as HTMLInputElement).checked).toBe(false);
});
it('prevents duplicate downloads while an install is pending',async()=>{
  const value=update();let finish!:()=>void;
  value.downloadAndInstall.mockImplementation(()=>new Promise<void>(resolve=>{finish=resolve;}));
  await findUpdate(value);fireEvent.click(screen.getByRole('checkbox'));
  const button=screen.getByRole('button',{name:'Download & install 0.6.6'});
  fireEvent.click(button);fireEvent.click(button);
  await waitFor(()=>expect(value.downloadAndInstall).toHaveBeenCalledTimes(1));finish();
  await screen.findByText(/Update installed\./);
});
it('releases the previous update handle before rechecking',async()=>{
  const value=await findUpdate();check.mockResolvedValue(null);
  fireEvent.click(screen.getByRole('button',{name:'Check for updates'}));
  await waitFor(()=>expect(value.close).toHaveBeenCalledTimes(1));
  await screen.findByText('You are up to date.');expect(screen.queryByRole('checkbox')).toBeNull();
});

it('keeps raw update errors in a collapsed technical-details disclosure',async()=>{
  check.mockRejectedValue(new Error('Could not fetch a valid release JSON from the remote'));
  render(<AppUpdater/>);fireEvent.click(screen.getByRole('button',{name:'Check for updates'}));
  await screen.findByRole('alert');
  expect(screen.getByText('Could not check for updates. Please try again later.')).toBeTruthy();
  const details=screen.getByText('Technical details').closest('details');
  expect(details?.open).toBe(false);
  expect(details?.textContent).toContain('Could not fetch a valid release JSON');
});

it('opens update help without contacting GitHub',()=>{
  render(<AppUpdater/>);
  expect(screen.queryByRole('region',{name:'App updates'})).toBeNull();
  fireEvent.click(screen.getByRole('button',{name:'About app updates'}));
  expect(screen.getByText('How updates work')).toBeTruthy();
  expect(check).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button',{name:'Close update details'}));
  expect(screen.queryByRole('region',{name:'App updates'})).toBeNull();
});

it('retains an active install when update details are closed and reopened',async()=>{
  const value=update();let finish!:()=>void;
  value.downloadAndInstall.mockImplementation(()=>new Promise<void>(resolve=>{finish=resolve;}));
  await findUpdate(value);fireEvent.click(screen.getByRole('checkbox'));
  fireEvent.click(screen.getByRole('button',{name:'Download & install 0.6.6'}));
  fireEvent.click(screen.getByRole('button',{name:'Close update details'}));
  fireEvent.click(screen.getByRole('button',{name:'About app updates'}));
  expect((screen.getByRole('button',{name:'Download & install 0.6.6'}) as HTMLButtonElement).disabled).toBe(true);
  await waitFor(()=>expect(value.downloadAndInstall).toHaveBeenCalledTimes(1));
  finish();await screen.findByText(/Update installed\./);
});

it('does not install or restart when active work blocks the update',async()=>{
 invoke.mockRejectedValue(new Error('Finish Windows games and installers before updating.'));
 const value=await findUpdate();fireEvent.click(screen.getByRole('checkbox'));fireEvent.click(screen.getByRole('button',{name:'Download & install 0.6.6'}));
 expect(await screen.findByRole('alert')).toHaveProperty('textContent',expect.stringContaining('Finish Windows games and installers before updating.'));
 expect(value.downloadAndInstall).not.toHaveBeenCalled();expect(invoke).toHaveBeenCalledTimes(1);
});
it('releases the update guard after installer failure and never restarts',async()=>{
 const value=update();value.downloadAndInstall.mockRejectedValue(new Error('Installer cancelled'));
 await findUpdate(value);fireEvent.click(screen.getByRole('checkbox'));fireEvent.click(screen.getByRole('button',{name:'Download & install 0.6.6'}));
 await screen.findByRole('alert');await waitFor(()=>expect(invoke).toHaveBeenCalledWith('cancel_app_update'));
 expect(invoke.mock.calls.some(c=>c[0]==='restart_after_update')).toBe(false);
});
it('reports relaunch failure without offering another action button or reinstalling',async()=>{
 invoke.mockImplementation(async(command:string)=>{if(command==='restart_after_update')throw new Error('Spawn failed');return [];});
 const value=await findUpdate();fireEvent.click(screen.getByRole('checkbox'));fireEvent.click(screen.getByRole('button',{name:'Download & install 0.6.6'}));
 await screen.findByText(/The update installed, but Proton Medic could not open/);
 expect(value.downloadAndInstall).toHaveBeenCalledTimes(1);
 expect(screen.queryByRole('button',{name:/reopen|relaunch|restart/i})).toBeNull();
 expect(invoke.mock.calls.some(c=>c[0]==='cancel_app_update')).toBe(false);
});
it('waits for successful installation before requesting relaunch',async()=>{
 const value=update();let finish!:()=>void;value.downloadAndInstall.mockImplementation(()=>new Promise<void>(resolve=>{finish=resolve;}));
 await findUpdate(value);fireEvent.click(screen.getByRole('checkbox'));fireEvent.click(screen.getByRole('button',{name:'Download & install 0.6.6'}));
 await waitFor(()=>expect(value.downloadAndInstall).toHaveBeenCalledTimes(1));
 expect(invoke.mock.calls.some(c=>c[0]==='restart_after_update')).toBe(false);
 finish();await waitFor(()=>expect(invoke).toHaveBeenCalledWith('restart_after_update',{version:'0.6.6'}));
});

const running = [{id:'1-2',runId:'1-3',name:'Wukong',kind:'game',canStop:true},{id:'4-5',runId:'4-6',name:'Setup',kind:'installer',canStop:true}];
async function blockedUpdate() {
 invoke.mockImplementation(async command => command === 'active_update_sessions' ? running : undefined);
 const value=await findUpdate();fireEvent.click(screen.getByRole('checkbox'));
 fireEvent.click(screen.getByRole('button',{name:'Download & install 0.6.6'}));
 await screen.findByRole('dialog');return value;
}
it('lists active sessions and cancellation never stops or downloads',async()=>{
 const value=await blockedUpdate();expect(screen.getByText('Wukong')).toBeTruthy();expect(screen.getByText('Setup')).toBeTruthy();
 fireEvent.click(screen.getByRole('button',{name:'Keep running'}));
 expect(screen.queryByRole('dialog')).toBeNull();expect(value.downloadAndInstall).not.toHaveBeenCalled();
 expect(invoke.mock.calls.map(c=>c[0])).toEqual(['active_update_sessions']);
});
it('stops only the reviewed sessions and waits before acquiring the update gate',async()=>{
 const value=await blockedUpdate();let finish!:()=>void;
 invoke.mockImplementation(async command => {if(command==='stop_update_sessions')await new Promise<void>(resolve=>{finish=resolve;});});
 fireEvent.click(screen.getByRole('button',{name:'Stop and update'}));
 await waitFor(()=>expect(invoke).toHaveBeenCalledWith('stop_update_sessions',{sessions:running,confirmed:true}));
 expect(value.downloadAndInstall).not.toHaveBeenCalled();expect(invoke.mock.calls.some(c=>c[0]==='begin_app_update')).toBe(false);
 finish();await waitFor(()=>expect(value.downloadAndInstall).toHaveBeenCalledTimes(1));
 expect(invoke.mock.calls.map(c=>c[0])).toEqual(['active_update_sessions','stop_update_sessions','begin_app_update','restart_after_update']);
});
it('never installs after a stop timeout',async()=>{
 const value=await blockedUpdate();invoke.mockRejectedValue(new Error('Still closing'));
 fireEvent.click(screen.getByRole('button',{name:'Stop and update'}));await screen.findByRole('alert');
 expect(value.downloadAndInstall).not.toHaveBeenCalled();expect(invoke.mock.calls.some(c=>c[0]==='begin_app_update')).toBe(false);
});
it('keeps older supervisors running and asks for manual closure',async()=>{
 invoke.mockResolvedValue([{...running[0],canStop:false}]);const value=await findUpdate();
 fireEvent.click(screen.getByRole('checkbox'));fireEvent.click(screen.getByRole('button',{name:'Download & install 0.6.6'}));
 await screen.findByRole('dialog');expect((screen.getByRole('button',{name:'Stop and update'}) as HTMLButtonElement).disabled).toBe(true);
 expect(value.downloadAndInstall).not.toHaveBeenCalled();
});
