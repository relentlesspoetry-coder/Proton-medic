import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { ImagePlus } from 'lucide-react';

type Source = { source: 'steam' | 'windows'; id: string; libraryPath: string | null };
type Props = { source: Source; name: string; onOpen: () => void };
const isImage = (value: unknown): value is string => typeof value === 'string' && /^data:image\/(png|jpeg);base64,/.test(value);

export function GameCover({ source, name, onOpen }: Props) {
  const key = JSON.stringify([source.source, source.id, source.libraryPath]);
  const current = useRef(key); current.current = key;
  const mounted = useRef(false);
  const serial = useRef(0);
  const choosing = useRef(false);
  const element = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(() => typeof IntersectionObserver === 'undefined');
  const [image, setImage] = useState<{ key: string; uri: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  useEffect(() => {
    if (visible || !element.current) return;
    const observer = new IntersectionObserver(entries => {
      if (entries.some(entry => entry.isIntersecting)) { setVisible(true); observer.disconnect(); }
    }, { rootMargin: '200px' });
    observer.observe(element.current);
    return () => observer.disconnect();
  }, [visible]);
  useEffect(() => {
    setError('');
    if (!visible) return;
    const request = ++serial.current;
    const [kind, id, libraryPath] = JSON.parse(key);
    void invoke<string | null>('get_game_cover', { source: kind, id, libraryPath }).then(uri => {
      if (mounted.current && current.current === key && request === serial.current && isImage(uri)) setImage({ key, uri });
    }).catch(() => {});
    return () => { ++serial.current; };
  }, [key, visible]);
  const change = async () => {
    if (choosing.current) return;
    choosing.current = true; setBusy(true); setError('');
    try {
      const path = await open({ multiple: false, title: `Choose cover for ${name}`, filters: [{ name: 'Game cover (PNG or JPEG)', extensions: ['png', 'jpg', 'jpeg'] }] });
      if (typeof path !== 'string' || !mounted.current || current.current !== key) return;
      const request = serial.current;
      const uri = await invoke<string>('set_game_cover', { ...source, path, confirmed: true });
      if (!mounted.current || current.current !== key || request !== serial.current) return;
      if (!isImage(uri)) throw new Error('The selected cover could not be displayed.');
      ++serial.current;
      setImage({ key, uri });
    } catch (failure) {
      if (mounted.current && current.current === key) setError(`Could not change cover: ${String(failure)}`);
    } finally {
      choosing.current = false;
      if (mounted.current) setBusy(false);
    }
  };
  const label = source.source === 'steam' ? 'Steam' : 'Windows';
  return <div ref={element} className="pm-cover">
    <button type="button" className="pm-library-art" aria-label={`Open ${name} (${label})`} onClick={onOpen}>
      {image?.key === key ? <img className="pm-game-cover" src={image.uri} alt="" onError={() => setImage(null)}/> : <span className="pm-cover-placeholder" aria-hidden="true"><span>{name}</span><small>Game cover</small></span>}
    </button>
    <button type="button" className="pm-cover-change" aria-label={`Change cover for ${name} (${label})`} disabled={busy} onClick={() => void change()}><ImagePlus size={16}/>{busy ? 'Saving…' : 'Change cover'}</button>
    {error && <p role="alert" className="pm-cover-error">{error}</p>}
  </div>;
}
