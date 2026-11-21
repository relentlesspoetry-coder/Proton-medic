# Gameweave — staged rebrand

Gameweave is a **working name** pending brand clearance. This branch starts
the UI redesign without changing the Proton Medic product identifier,
package IDs, saved game profiles, Proton prefixes, updater signatures,
update endpoints, release metadata or user data.

## Design contract

- The approved Library layout is the reference.
- Exactly four color themes: Carbon Green, Blue & Purple, Red & Orange,
  and Teal & Green.
- Glass is actual backdrop blur with translucent surfaces, not an opaque
  gradient; reduced motion and strong-focus treatments remain available.
- Four primary navigation destinations: Library, Install, Tools, Settings.
- Game health claims must describe verified evidence rather than assume
  a successful launch or a Proton compatibility guarantee.

## Implementation sequence

1. Visual preferences, shared glass tokens and the four-item shell.
2. Integrate the four *original supplied* logo images (no CSS recreation).
3. Library contextual game panel, preserving existing launch paths.
4. EXE installation wizard backed by existing safe Rust operations.
5. Game Details, Tools, Settings and Steam Deck responsive refinements.
6. Regression, accessibility, native smoke tests and installer packages.

All new visuals are separated from existing game-management operations;
the eventual package rename and updater migration require independent review.

## Original logo assets

The four original user-supplied logos are packaged in `public/branding/`
as lossless WebP images. Theme IDs match the filenames; the source PNG
pixels, including transparency, are preserved.

`GameweaveBrand` maps each theme to
`/branding/{carbon-green|cyan-purple|red-orange-gold|teal-green-gold}.webp`
and falls back to a text mark if loading fails.
