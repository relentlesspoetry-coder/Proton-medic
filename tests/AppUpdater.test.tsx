import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { AppUpdater } from '../src/AppUpdater';
const { check } = vi.hoisted(() => ({ check: vi.fn() }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check }));
beforeEach(() => { check.mockReset(); });
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
  await screen.findByText(/Update installed. Finish any current work/);expect(value.downloadAndInstall).toHaveBeenCalledTimes(1);
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
  expect(value.downloadAndInstall).toHaveBeenCalledTimes(1);finish();
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
