import { Check, Layers, Monitor, Palette, Play, Settings2 } from 'lucide-react';
import { DEFAULT, THEMES, type Appearance, type Glass } from './appearance';
import './appearance-settings.css';

interface Props { value: Appearance; onChange: (value: Appearance) => void; saveError?: string }
const GLASS_LEVELS: Glass[] = ['subtle', 'balanced', 'strong'];
const label = (name: Glass) => name[0].toUpperCase() + name.slice(1);

export function AppearanceSettings({ value, onChange, saveError }: Props) {
  const set = (patch: Partial<Appearance>) => onChange({ ...value, ...patch });
  const themeIndex = THEMES.findIndex(theme => theme.id === value.theme);
  const activeTheme = THEMES[themeIndex] ?? THEMES[0];
  const glassIndex = GLASS_LEVELS.indexOf(value.glass);
  return <section className="gw-settings-appearance" aria-label="Appearance">
    <nav className="gw-settings-preferences" aria-label="Appearance preferences">
      <span className="gw-settings-eyebrow">Preferences</span>
      <a className="gw-settings-nav-active" href="#gw-themes"><Palette size={18} aria-hidden="true"/> Appearance</a>
      <a href="#gw-effects"><Layers size={18} aria-hidden="true"/> Glass &amp; effects</a>
      <a href="#gw-preview"><Monitor size={18} aria-hidden="true"/> Live preview</a>
      <p>Appearance changes apply across Library, Install, Tools, and Settings.</p>
    </nav>
    <div className="gw-settings-content">
      <section id="gw-themes" className="gw-settings-pane gw-settings-themes" aria-labelledby="gw-appearance-heading">
        <div className="gw-settings-heading">
          <div><h2 id="gw-appearance-heading">Choose Your Theme</h2>
            <p>Four distinct visual styles. One consistent Gameweave layout.</p></div>
          <span className="gw-active-theme"><i aria-hidden="true"/>{activeTheme.label} active</span>
        </div>
        <div className="gw-settings-theme-grid" role="radiogroup" aria-label="Color theme">
          {THEMES.map((theme, index) => <button key={theme.id} id={'gw-theme-' + theme.id} type="button" role="radio"
            aria-checked={value.theme === theme.id} tabIndex={themeIndex === index ? 0 : -1}
            className="gw-settings-theme-tile"
            style={{ backgroundImage: 'radial-gradient(ellipse at 65% 60%, ' + theme.colors[1] + '50, transparent 79%), linear-gradient(145deg, ' + theme.colors[0] + ', #0b1a19)' }}
            onClick={() => set({ theme: theme.id })}
            onKeyDown={event => {
              const direction = event.key === 'ArrowRight' || event.key === 'ArrowDown' ? 1
                : event.key === 'ArrowLeft' || event.key === 'ArrowUp' ? -1 : 0;
              if (!direction && event.key !== 'Home' && event.key !== 'End') return;
              event.preventDefault();
              const next = event.key === 'Home' ? 0 : event.key === 'End' ? THEMES.length - 1
                : (index + direction + THEMES.length) % THEMES.length;
              const option = THEMES[next];
              set({ theme: option.id });
              event.currentTarget.ownerDocument.getElementById('gw-theme-' + option.id)?.focus();
            }}>
            <img src={'/branding/' + theme.id + '.webp'} alt="" width="240" height="112" loading="eager"/>
            <strong>{theme.label}</strong>{value.theme === theme.id && <span className="gw-settings-check"><Check size={12} aria-hidden="true"/></span>}
          </button>)}
        </div>
      </section>
      <div className="gw-settings-detail-grid">
        <section id="gw-effects" className="gw-settings-pane" aria-labelledby="gw-effects-heading">
          <div className="gw-settings-heading"><div><h2 id="gw-effects-heading">Glass &amp; Effects</h2>
            <p>True background blur with translucent, tinted controls.</p></div></div>
          <div className="gw-effect-note"><span>Frosted-glass buttons<small>Blur the background directly behind a control.</small></span>
            <span className="gw-blur-badge"><Check size={13} aria-hidden="true"/> Enabled</span></div>
          <label className="gw-motion-control">
            <span>Reduce motion<small>Minimize decorative transitions and movement.</small></span>
            <span className="gw-motion-switch">
              <input type="checkbox" checked={value.reduceMotion}
                onChange={event => set({ reduceMotion: event.target.checked })}/>
              <span aria-hidden="true"/>
            </span>
          </label>
          <div className="gw-glass-control">
            <div><label htmlFor="gw-glass">Glass intensity</label><span aria-hidden="true">{label(value.glass)}</span></div>
            <input id="gw-glass" type="range" min={0} max={2} step={1} value={glassIndex}
              aria-valuetext={label(value.glass)} onChange={event => set({ glass: GLASS_LEVELS[Number(event.target.value)] })}/>
            <small>Adjust the strength of the background blur.</small>
          </div>
        </section>
        <section id="gw-preview" className="gw-settings-pane" aria-labelledby="gw-preview-heading">
          <div className="gw-settings-heading"><div><h2 id="gw-preview-heading">Live Appearance Preview</h2>
            <p>Translucent buttons reveal blurred colors beneath.</p></div></div>
          <div className="gw-settings-demo">
            <div className="gw-settings-demo-window" aria-hidden="true">
              <strong>My Library</strong><small>Gameweave · {activeTheme.label}</small>
              <div className="gw-settings-demo-filters"><span>All Games</span><span>Steam</span><span>Windows</span></div>
              <div className="gw-settings-demo-divider"/>
              <div className="gw-settings-demo-actions"><span><Play size={13} fill="currentColor"/> Play Game</span>
                <span><Settings2 size={13}/> Configure</span></div>
            </div>
          </div>
          <footer className="gw-settings-preview-note">
            <span><Check size={15} aria-hidden="true"/> Blur enabled</span><span>Glass intensity: {label(value.glass)}</span>
          </footer>
        </section>
      </div>
      <footer className="gw-settings-footer">
        <p>Appearance changes apply immediately. Game data and Proton settings stay unchanged.</p>
        <button type="button" className="pm-button pm-button-secondary" onClick={() => onChange({ ...DEFAULT })}>Reset to Default</button>
      </footer>
      {saveError && <p role="alert" className="pm-error">{saveError}</p>}
    </div>
  </section>;
}
