# liteweight-fps-wasm

The source code in this project is available under the terms of either the
[MIT License](LICENSE-MIT) or the [Apache License, Version 2.0](LICENSE-APACHE).

`liteweight-fps-wasm` is the WebAssembly port of
[liteweight-fps](https://github.com/issy20/liteweight-fps), an offline,
single-player FPS for practicing against BOTs. It runs in a WebGPU-capable
browser and shares the simulation, physics, arena, UI mesh generation, and
renderer with the native game; only the platform layer is web-specific. It
needs no installer and keeps working offline after the first visit.

## Game features

- **Free Training** and **Tracking Aim** practice modes.
- First-person mouse look, WASD movement, sprinting, jumping, and collision
  against arena walls, cover, stairs, and ramps using AABB approximations.
- Three weapons with hitscan firing, ADS, recoil, reloads, and weapon switching:

  | Weapon | Magazine | Fire rate | Body damage | Reload |
  | --- | ---: | ---: | ---: | ---: |
  | Pistol | 7 | 180 RPM | 50 | 1.8 s |
  | Assault rifle | 30 | 600 RPM | 25 | 2.0 s |
  | Sniper rifle | 5 | 50 RPM | 100 | 2.8 s |

  Headshots deal twice the body damage. The sniper rifle's ADS-in and ADS-out
  times are 400 ms and 200 ms.
- BOT behaviors: Static, Strafe, Chase, and Mixed. BOT decisions run at 20 Hz;
  movement uses the fixed simulation timestep. Tracking Aim uses strafing
  targets that move across the player's view.
- Separate head, body, arm, leg, and foot hitboxes. A defeated BOT respawns after
  a configurable delay at a free location away from the player.
- Session tracking for kills, accuracy, and headshot rate, with a result screen
  and per-mode best records.
- Three independent profiles for settings and best records.
- Settings for mouse and ADS sensitivity, FOV, vertical inversion, remappable
  gameplay keys, internal render resolution, fullscreen, FPS limit, infinite
  ammo, audio volume, hit-marker strength, and BOT parameters.
- Crosshair customization: color, size, gap, thickness, and center dot.
- Slim, faceless mannequin BOTs with rounded low-poly body parts and an orange
  forehead marker.
- A HUD with ammo, reticle, hit markers, accuracy, and headshot rate, along with
  hit reactions, muzzle flashes, impact effects, simple weapon models, a sniper
  scope, and synthesized sound effects.
- A **CREDITS** screen and a bundled [credits page](credits.html) that list the
  game, font, engine, privacy, and license information.
- Installable as a PWA: the app shell and hashed assets are cached, so the game
  starts without a network after the first successful load.

The page requires a WebGPU-capable desktop browser with a keyboard and mouse.
Touch devices show a desktop-only notice. WebGPU requires a secure context
(`https://` or `http://localhost`).

## Controls

| Input | Action |
| --- | --- |
| Up / Down, Enter | Navigate and select menu items |
| Mouse movement | Look around while the pointer is locked |
| W / A / S / D | Move forward / left / backward / right (rebindable) |
| Left Shift | Sprint while held (rebindable) |
| Space | Jump (rebindable) |
| Left mouse button | Fire; hold for automatic fire with the assault rifle |
| Right mouse button | Aim down sights while held |
| 1 / 2 / 3 | Select pistol / assault rifle / sniper rifle (rebindable) |
| R | Reload; reload takes priority over firing and is ignored when full (rebindable) |
| Esc | Pause and release the pointer (rebindable) |
| Left / Right, Enter | Adjust the selected setting / confirm a menu item |
| Home | Reset the current settings to their defaults |

Click the canvas to lock the pointer and start or resume play. While the pointer
is not locked, the game shows **CLICK TO PLAY** or **PAUSED** and freezes the
world. Browsers consume the first Esc press to release the pointer, so releasing
the lock opens the pause menu directly.

The canvas always fills the viewport. Rendering pauses while the page is hidden,
occluded, or has zero size, and resumes when it becomes visible again. Browsers
may throttle background tabs.

## Play a deployed build

`.github/workflows/deploy.yml` runs on pushes to `main`, pull requests, and
manual dispatch. It verifies the project, builds `dist/` with Trunk, and deploys
it to the Cloudflare Pages project `liteweight-fps-wasm`; pull requests deploy a
preview branch. Deployments require the `CLOUDFLARE_API_TOKEN` and
`CLOUDFLARE_ACCOUNT_ID` repository secrets.

To run the release build locally instead:

```sh
trunk serve --release
# open http://127.0.0.1:8080/
```

## Build from source

### Requirements

- Rust stable (via [rustup](https://rustup.rs)).
- The `wasm32-unknown-unknown` target.
- [Trunk](https://trunkrs.dev).
- [Binaryen](https://github.com/WebAssembly/binaryen) providing `wasm-opt`. The
  build enables bulk-memory and non-trapping-float-to-int so that `wasm-opt -Oz`
  accepts the generated module; Binaryen 123 is the version this project uses.
- A WebGPU-capable browser.

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
```

The game itself needs no native C toolchain or system audio library: it uses the
browser's Web Audio API and `localStorage`.

### 1. Development server

```sh
git clone --depth 1 https://github.com/issy20/liteweight-fps-wasm.git
cd liteweight-fps-wasm
trunk serve
```

Trunk rebuilds on changes and serves the game at `http://127.0.0.1:8080/`.
The host settings live in `Trunk.toml`.

### 2. Release build

```sh
trunk build --release
```

The result is a static site in `dist/`. Serve that directory over HTTP or HTTPS
and open it in a WebGPU-capable browser. The release profile uses thin LTO,
`opt-level = "s"`, stripping, and `wasm-opt -Oz`.

### Browser requirements

- Chrome or Edge 113+, or another browser with WebGPU enabled. When WebGPU is
  unavailable, the page shows a notice and does not start the game.
- A desktop-sized display and a pointing device. Pointer lock is required for
  mouse look and is not supported on touch-only devices.
- WebGPU is unavailable in insecure contexts except `localhost`.

### Troubleshooting

- **"WebGPU が必要です"** — update the browser to a WebGPU-capable version, or
  enable WebGPU in the browser's experimental flags and reload.
- **The pointer does not lock** — click inside the canvas. Browsers require a
  user gesture, and some block pointer lock after switching tabs; click again.
- **No sound** — audio starts only after a user gesture because of browser
  autoplay policies. If the audio context cannot be created, the game reports
  that it will run silently.
- **The screen is black or the game does not start** — check the browser console.
  The host page hides the loading overlay and shows an error message when
  renderer initialization fails.
- **`wasm-opt` rejects an option** — install Binaryen 123 or newer and make sure
  it is on `PATH`.
- **An old build is still served** — the service worker prefers the network for
  documents but caches hashed assets. Reload once, or clear the site data.

## Build and test

Run formatting, Clippy, unit tests, and the WebAssembly type check:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo check --locked --target wasm32-unknown-unknown
```

The unit tests target pure Rust logic and run on the host; the `wasm32` check
verifies the browser-only code paths compile. Browser checks against real WebGPU
hardware remain manual, and CI compilation or tests do not replace device
testing.

The deploy workflow runs `cargo test` and the `wasm32` check before each
deployment. Run the full command list locally before opening a pull request.

## Where data is stored

Settings, profiles, best records, and the last seen version are stored in this
browser profile's `localStorage`:

| Key | Contents |
| --- | --- |
| `liteweight-fps/settings-{n}.json` | Settings for profile `n` (0–2) |
| `liteweight-fps/records-{n}.json` | Best records for profile `n` |
| `liteweight-fps/profile` | Active profile number |
| `liteweight-fps/version` | Last version shown in the update notice |

Nothing is sent to a server, and the game performs no analytics or tracking.
Clearing the site data removes all saved settings and records.

## Source layout

- `src/lib.rs` — Crate entry point, module declarations, and the wasm-bindgen
  start function.
- `src/app.rs` — Web event loop and game progression.
- `src/platform.rs` — Time (`web_time`) and random-seed platform abstraction.
- `src/persistence.rs` — Settings and records in `localStorage` or files.
- `src/world/` — Arena and tracking-range meshes and colliders.
- `src/engine/layout.rs` — Internal resolution and display layout.
- `src/engine/renderer.rs` — Scene, HUD, text, and presentation rendering.
- `src/engine/text.rs`, `src/engine/text_render.rs` — Font atlas and text
  pipeline.
- `src/engine/window.rs` — Surface setup, resizing, and presentation.
- `src/engine/audio.rs` — Synthesized effects and Web Audio playback.
- `src/simulation/time.rs` — Fixed-timestep simulation.
- `src/simulation/player.rs` — First-person view, movement, jumping, gravity,
  and collision.
- `src/simulation/weapon.rs` — Weapon data, loadout, firing, ADS, recoil,
  reload, and bolt state.
- `src/simulation/bot.rs` — BOT behaviors, hitboxes, health, and respawning.
- `src/simulation/combat.rs` — Hitscan, damage, statistics, and safe respawn
  selection.
- `src/ui/` — HUD, menus, result and credits screens, and BOT mesh generation.
- `index.html` — Host page, canvas, loading and fallback overlays.
- `credits.html` — Full credits and license page.
- `assets/fonts/` — Embedded Cantarell font and its OFL license.
- `assets/shaders/` — WGSL scene, HUD, text, and blit shaders.
- `manifest.webmanifest`, `sw.js`, `icon-*.png` — PWA metadata and offline
  assets.

## Differences from the native game

- Removed: the `--benchmark`, `--smoke`, and `--fps-log` options, the `bench`
  feature, Miniquad, and native packaging.
- Removed: display resolution, VSync, and quit-game settings. The canvas always
  fills the viewport, and presentation is fixed to `Fifo`; the displayed frame
  rate is capped by the display's refresh rate. Users close the tab or return to
  the title screen instead of quitting.
- Replaced: `rodio` with Web Audio, file-backed settings with `localStorage`,
  `std::time` with `web_time`, and the process ID with a random seed.
- Added: loading and WebGPU fallback overlays, the desktop-only notice, the
  click-to-play/paused overlay, fullscreen through the Fullscreen API, and
  profiles.
- Fullscreen cannot be restored automatically on load because browsers require a
  user gesture. The pointer is locked through the Pointer Lock API; the cursor is
  hidden while captured and visible in menus.
- Background tabs may be throttled by the browser. Audio may start suspended
  until the first interaction.

## Release status

The project is distributed as source and as a static web build. Gameplay
performance has not been measured on representative low-end hardware, and
runtime behavior has not been verified on every browser, GPU, or operating
system. WebGPU support and performance vary by browser and driver. Do not treat
the build as a guarantee of performance or verified compatibility. CI
compilation and automated tests do not replace testing on real devices.

## License

The source code is available under the terms of either the
[MIT License](LICENSE-MIT) or the [Apache License, Version 2.0](LICENSE-APACHE),
at your option. Copyright 2026 issy20.

This project is a WebAssembly port of
[liteweight-fps](https://github.com/issy20/liteweight-fps) by issy20. The
simulation, physics, world, UI mesh generation, and renderer come from that
project and are used under the same dual license.

The bundled **Cantarell** font is distributed under the SIL Open Font License
1.1. The license text and copyright notice are in
[assets/fonts/OFL.txt](assets/fonts/OFL.txt); the release build copies it to
`OFL.txt`. See [assets/fonts/README.md](assets/fonts/README.md) for details.

Third-party Rust crates remain under their own licenses. The in-game **CREDITS**
screen summarizes them, and [credits.html](credits.html) lists the main
dependencies and links to the bundled MIT, Apache-2.0, and OFL license texts.
Refer to each upstream project for the full license and copyright notice of its
crate.

## Additional references

- [Specification](SPECIFICATION.md)
- [Credits and license page](credits.html)
- [Bundled font notes](assets/fonts/README.md)
- [Original native project](https://github.com/issy20/liteweight-fps)

Primary API references: [wgpu 24](https://docs.rs/wgpu/24.0.5/wgpu/),
[winit 0.30](https://docs.rs/winit/0.30.13/winit/), and
[wasm-bindgen](https://docs.rs/wasm-bindgen/).
