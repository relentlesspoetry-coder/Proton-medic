import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
interface ControlState { enabled: boolean; revision: number }
const preference = 'proton-medic.auto-log-fixes.enabled';
function checked(value: ControlState): ControlState {
  if (!value || typeof value.enabled !== 'boolean' || !Number.isSafeInteger(value.revision) || value.revision < 0) {
    throw new Error('Automatic-fix state is unavailable');
  }
  return value;
}
const description = (enabled: boolean) => enabled
  ? 'Automatic log fixes enabled. New supported failures will be queued and fixed after Steam closes, with a backup.'
  : 'Automatic fixes disabled. Log monitoring continues read-only.';

export function useAutoFixControl() {
  const [enabled, setEnabled] = useState(false);
  const [pending, setPending] = useState(true);
  const [message, setMessage] = useState<string | null>(null);
  const revision = useRef(0);
  const request = useRef(0);
  const pendingRequest = useRef(true);
  const mounted = useRef(false);
  useEffect(() => {
    mounted.current = true;
    const current = ++request.current;
    pendingRequest.current = true;
    const initialize = async () => {
      try {
        let state = checked(await invoke<ControlState>('get_auto_fix_control'));
        if (!mounted.current || current !== request.current) return;
        revision.current = state.revision;
        setEnabled(state.enabled);
        let saved = false;
        try { saved = localStorage.getItem(preference) === 'true'; } catch { /* explicit toggling remains available */ }
        if (saved !== state.enabled) {
          state = checked(await invoke<ControlState>('set_auto_fix_control', { enabled: saved, revision: ++revision.current }));
        }
        if (!mounted.current || current !== request.current) return;
        revision.current = state.revision;
        setEnabled(state.enabled);
      } catch (error) {
        if (mounted.current && current === request.current) setMessage(`Could not restore automatic-fix settings: ${String(error)}`);
      } finally {
        if (mounted.current && current === request.current) { pendingRequest.current = false; setPending(false); }
      }
    };
    void initialize();
    return () => { mounted.current = false; ++request.current; };
  }, []);
  const toggle = useCallback(async (desired: boolean) => {
    if (pendingRequest.current) return;
    pendingRequest.current = true;
    const current = ++request.current;
    setPending(true);
    setMessage(desired ? 'Enabling automatic fixes…' : 'Stopping automatic fixes; finishing any change already being saved…');
    try {
      const state = checked(await invoke<ControlState>('set_auto_fix_control', { enabled: desired, revision: ++revision.current }));
      if (!mounted.current || current !== request.current) return;
      revision.current = state.revision;
      setEnabled(state.enabled);
      try { localStorage.setItem(preference, String(state.enabled)); } catch { /* backend consent remains authoritative this session */ }
      setMessage(description(state.enabled));
    } catch (error) {
      // Another setting request may have superseded this one; recover the authoritative revision.
      try {
        const state = checked(await invoke<ControlState>('get_auto_fix_control'));
        if (mounted.current && current === request.current) { revision.current = state.revision; setEnabled(state.enabled); }
      } catch { /* retain the last confirmed setting and show the error */ }
      if (mounted.current && current === request.current) setMessage(`Could not change automatic fixes: ${String(error)}`);
    } finally {
      if (mounted.current && current === request.current) { pendingRequest.current = false; setPending(false); }
    }
  }, []);
  return { enabled, pending, message, toggle };
}
