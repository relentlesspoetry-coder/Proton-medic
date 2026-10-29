import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Gamepad2 } from 'lucide-react';

type Props = { source: { kind: 'windows' | 'candidate'; id: string; executable: string | null } | { kind: 'steam'; appId: string; libraryPath: string }; className?: string };
export function GameIcon({ source, className = '' }: Props) {
  const key = source.kind !== 'steam' ? JSON.stringify([source.kind, source.id, source.executable]) : JSON.stringify(['steam', source.appId, source.libraryPath]);
  const element = useRef<HTMLSpanElement>(null);
  const [visible, setVisible] = useState(() => typeof IntersectionObserver === 'undefined');
  useEffect(() => {
    if (visible || !element.current) return;
    const observer = new IntersectionObserver(entries => {
      if (entries.some(entry => entry.isIntersecting)) { setVisible(true); observer.disconnect(); }
    }, { rootMargin: '200px' });
    observer.observe(element.current);
    return () => observer.disconnect();
  }, [visible]);
  const [image, setImage] = useState<{ key: string; uri: string } | null>(null);
  useEffect(() => {
    if (!visible) return;
    let stopped = false;
    const selected = JSON.parse(key) as string[];
    if (selected[0] !== 'steam' && !selected[2]) return;
    const command = selected[0] === 'candidate' ? 'get_windows_executable_icon' : selected[0] === 'windows' ? 'get_windows_game_icon' : 'get_steam_game_icon';
    const args = selected[0] === 'candidate' ? { id: selected[1], path: selected[2] } : selected[0] === 'windows' ? { id: selected[1] } : { appId: selected[1], libraryPath: selected[2] };
    void invoke<string | null>(command, args).then(uri => {
      if (!stopped && typeof uri === 'string' && /^data:image\/(png|jpeg);base64,/.test(uri)) setImage({ key, uri });
    }).catch(() => {});
    return () => { stopped = true; };
  }, [key, visible]);
  return <span ref={element} className={`pm-game-image ${className}`} aria-hidden="true">{image?.key === key ? <img src={image.uri} alt="" onError={() => setImage(null)}/> : <Gamepad2 size={30}/>}</span>;
}
