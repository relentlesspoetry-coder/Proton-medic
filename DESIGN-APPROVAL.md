# Interface design

Proton Medic uses a blue glass theme with a landscape backdrop, sidebar navigation, three primary home actions, compact status metrics, and a game list.

The interface displays scanned game names, counts, and diagnostic states. Advanced repair and backup controls remain expandable.

- Base component styles: `src/style.css`
- Theme and responsive styles: `src/theme.css`
- Bundled images: `src/assets/`
- Header updates and version label: `src/AppUpdater.tsx`

Keep theme rules grouped by component, followed by responsive breakpoints. Preserve keyboard focus indicators, reduced-motion and reduced-transparency settings, readable opaque fallbacks, and consent controls.
