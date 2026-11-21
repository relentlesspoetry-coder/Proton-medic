import { CalendarClock, FolderOpen, Gamepad2, Layers3, Play, ShieldCheck } from 'lucide-react';
import type { ReactNode } from 'react';
import { GameCover } from '../GameCover';
import { GameDetailTabs, type GameDetailTab } from '../GameDetailTabs';
import type { Game } from '../types';
import './detail.css';

type CoverSource = Parameters<typeof GameCover>[0]['source'];

export function GameDetailHero({ name, source, subtitle, status, actions, heading = 'h2' }: {
  name: string;
  source: CoverSource;
  subtitle: ReactNode;
  status?: ReactNode;
  actions?: ReactNode;
  heading?: 'h2' | 'h3';
}) {
  const Heading = heading;
  return <header className="gw-detail-hero">
    <div className="gw-detail-hero-art"><GameCover name={name} source={source} onOpen={() => {}} readOnly/></div>
    <div className="gw-detail-hero-content">
      <div className="gw-detail-identity">
        <Heading>{name}</Heading>
        <div className="gw-detail-identity-meta"><span>{subtitle}</span>{status}</div>
      </div>
      {actions && <div className="gw-detail-hero-actions">{actions}</div>}
    </div>
  </header>;
}

export function DetailFacts({ rows }: { rows: Array<[string, ReactNode]> }) {
  return <dl className="gw-detail-facts">{rows.map(([label, value]) =>
    <div key={label}><dt>{label}</dt><dd>{value}</dd></div>
  )}</dl>;
}

function lastPlayed(value?: number | null) {
  if (typeof value !== 'number' || !Number.isFinite(value) || value <= 0 || value > Date.now()) return 'Not recorded';
  return new Date(value).toLocaleDateString();
}

export function SteamGameDetail({ game, status, note, toolbar, tabs }: {
  game: Game;
  status: ReactNode;
  note: string;
  toolbar: ReactNode;
  tabs: GameDetailTab[];
}) {
  return <div className="gw-detail">
    <div className="gw-detail-heading">{toolbar}<h1>Game Details</h1><p>Manage settings, Proton, and tools for this game.</p></div>
    <GameDetailTabs
      renderHero={open => <GameDetailHero name={game.name} source={{source:'steam',id:game.appId,libraryPath:game.libraryPath}}
        subtitle={<><Gamepad2 size={15} aria-hidden="true"/> Steam · {game.configuredTool || 'Steam default'}</>}
        status={status}
        actions={<button type="button" className="pm-button pm-button-primary" disabled={!game.installPathExists} onClick={() => open('launch')}><Play size={17}/>Test Launch</button>}/>}
      tabs={[{id:'overview',label:'Overview',content: <div className="gw-detail-panels">
        <div className="gw-detail-stack">
          <section className="gw-detail-panel"><h3><FolderOpen size={18}/>Game Information</h3>
            <DetailFacts rows={[
              ['Install folder', game.installPath],
              ['Proton prefix', game.prefixExists ? game.prefixPath : 'Not found'],
              ['Steam App ID', game.appId],
              ['Build ID', game.buildId || 'Unknown'],
            ]}/>
          </section>
          <section className="gw-detail-panel"><h3><CalendarClock size={18}/>Activity</h3><DetailFacts rows={[[ 'Last played', lastPlayed(game.lastPlayedAt) ]]}/></section>
        </div>
        <div className="gw-detail-stack">
          <section className="gw-detail-panel"><h3><Layers3 size={18}/>Proton & Compatibility</h3>
            <DetailFacts rows={[
              ['Configured runtime', game.configuredTool || 'Automatic / Steam default'],
              ['Install folder', game.installPathExists ? 'Found' : 'Missing'],
              ['Proton prefix', game.prefixExists ? 'Found' : 'Not found'],
            ]}/>
          </section>
          <section className="gw-detail-panel"><h3><ShieldCheck size={18}/>Game Status</h3>
            {status}<p>{note}</p>
          </section>
        </div>
      </div>}, ...tabs]}/>
  </div>;
}
