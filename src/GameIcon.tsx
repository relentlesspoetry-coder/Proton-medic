import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Gamepad2 } from 'lucide-react';

type Props = { source: { kind: 'windows'; id: string; executable: string | null } | { kind: 'steam'; appId: string; libraryPath: string }; className?: string };
export function GameIcon({ source, className = '' }: Props) {
  const key = source.kind === 'windows' ? JSON.stringify(['windows', source.id, source.executable]) : JSON.stringify(['steam', source.appId, source.libraryPath]);
  const [image, setImage] = useState<{ key: string; uri: string } | null>(null);
  useEffect(() => {
    let stopped = false;
    const selected = JSON.parse(key) as string[];
    if (selected[0] === 'windows' && !selected[2]) return;
    const command = selected[0] === 'windows' ? 'get_windows_game_icon' : 'get_steam_game_icon';
    const args = selected[0] === 'windows' ? { id: selected[1] } : { appId: selected[1], libraryPath: selected[2] };
    void invoke<string | null>(command, args).then(uri => {
      if (!stopped && typeof uri === 'string' && /^data:image\/(png|jpeg);base64,/.test(uri)) setImage({ key, uri });
    }).catch(() => {});
    return () => { stopped = true; };
  }, [key]);
  return <span className={`pm-game-image ${className}`} aria-hidden="true">{image?.key === key ? <img src={image.uri} alt="" onError={() => setImage(null)}/> : <Gamepad2 size={30}/>}</span>;
}
