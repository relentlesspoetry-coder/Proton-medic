import type { Runtime } from './WindowsGames';

export const selectableRuntimes = (runtimes: Runtime[]) => runtimes.filter(runtime => runtime.kind === 'proton');
