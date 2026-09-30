# Charlita — agreed product

Native desktop application for Windows and Linux. Free and open source. Intended primarily for a group of streamers, distributable to anyone. Complete product scope, with low resource consumption as a design requirement; no fixed RAM budget and no artificial limit on participant count.

## Connection and control

- Official Discord desktop client only. Browser and alternative clients are excluded by explicit decision.
- Server voice channels, direct calls, and group calls.
- Follow the host's current call automatically; allow pinning a channel.
- The host controls all guest representations. Guests send importable character packages and do not install software.
- Newly discovered guests appear in the panel and require host activation before broadcast. Known guests retain per-profile configuration.
- Speaking/silence events drive the default subtle brightness increase and hop. Configurable effects, state images, expressions, timing, and hotkeys. Volume-driven lip synchronization is outside this agreed behavior.
- On Discord disconnection, keep the composition visible at rest, preserve manual controls, and reconnect automatically.

## Characters, groups, and output

- Imported artwork: PNG, JPEG, WebP, GIF, animated WebP, sprite sheets, and transparent WebM. Character drawing tools are outside scope.
- Reusable library, crop, size, orientation, names, states, custom expressions, and portable packages.
- One usual character per Discord user, with group-specific overrides.
- Group output is primary: horizontal row, vertical column, grid, or free positioning. Each group has a stable local URL. Individual output remains available.
- Automatic layouts reflow on membership/visibility changes, with a preserve-spaces option. Free layout retains positions.
- Native visual editor; keyboard equivalents for canvas operations, alignment, layer order, locked positions, preview/test states.
- Composition edits are saved as draft and reach the output only with Apply. Show/hide/expression live controls act immediately.
- Transparent browser-source output compatible with OBS Studio on Linux and Windows, and Streamlabs Desktop on Windows. Window closure keeps the local engine running from the system tray.

## Delivery and UX

- Sober dark Qt Quick/QML interface; center canvas, guest navigation, separate editing drawers. Avoid OBS-style docks and appearance. Spanish and English, system-language default, manual selection.
- Autosave, profiles, import/export including assets, recovery after restart, actionable errors, diagnostics export.
- Installers and portable distributions for both operating systems. Discord credentials remain machine-local and are never exported.
- Update checks from GitHub Releases for `48hoursnonstop/charlita`; automatic checks, explicit installation, no update during broadcast without user action.
- Reference performance hardware: 8 GB RAM, four-core CPU, integrated GPU. Benchmark growing scenes rather than claiming unlimited scenes have constant cost.
- Rust engine, native Qt 6 Quick/QML editor, SQLite, Tokio/Axum, browser-native overlay renderer. Internal modules, one local engine, event-driven output and paused hidden animations.
- New groups fit their canvas to content by default. Row, column and grid retain
  guest sizes and reserve configured hop and label space. Free layouts include
  negative coordinates and keep their coordinate origin stable on hide/show.
  A fixed width/height remains available per group. Legacy projects retain
  their saved fixed dimensions until auto size is enabled. Canvas changes follow
  the same draft/Apply boundary as the composition.

## External validation gates

No Discord application currently exists. Real call integration requires registering an application, configuring authorization, and obtaining the required RPC voice access. Public RPC access requires Discord approval. Mock protocol tests cannot establish real-call or public-distribution compatibility.

Windows installer, tray, hotkeys, media transparency, and native UI must be verified on Windows; Linux verification alone is insufficient. Release installation requires published build artifacts and checksums in GitHub Releases.
