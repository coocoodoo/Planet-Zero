# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Flight Club is a Rust 2024 pixel flight game that targets the M5 Cardputer Zero (aarch64 Linux, 320×170 RGB565 framebuffer, evdev keyboard, BMI270 IMU). It also has a desktop build (macroquad) for previewing and debugging. Most docs under `docs/`, code comments, and user-facing error strings are in Chinese.

## Commands

```sh
cargo run --release                          # desktop preview (default feature "desktop")
cargo run --release -- --capture captures    # desktop: export menu/race/combat/boss/cockpit/vehicle PNGs, then exit

cargo fmt --all -- --check                   # CI-enforced
cargo clippy --all-features --all-targets -- -D warnings
cargo test --locked                          # desktop feature set (CI)
cargo test --locked --no-default-features --features cardputer   # device feature set (CI)
cargo test hover                             # run tests whose names match a filter

bash scripts/build-cardputer.sh              # cross-build static aarch64-musl binary -> dist/flight-club-cardputer
python3 scripts/package-cardputer.py         # build dist/flight-club_<version>_arm64.deb (uses dpkg-deb or Docker)
```

The cross build needs `rustup target add aarch64-unknown-linux-musl` and links with the toolchain's bundled `rust-lld`. Building on Linux for desktop needs the X11/xkbcommon dev packages listed in `.github/workflows/ci.yml`.

`[profile.dev]` uses `opt-level = 2`, so debug tests run at near-release speed.

## Features and binaries

- `desktop` (default): enables `macroquad` and (non-Linux only) `cpal`. Required by the `flight-club` bin (`src/main.rs`).
- `cardputer`: enables `memmap2` and the `cardputer` module (fbdev mmap, evdev, IMU, battery/Wi-Fi telemetry). Required by `flight-club-cardputer` (`src/bin/cardputer.rs`).
- `flight-club-audio` (`src/bin/audio.rs`) exports music and benchmarks synthesis without either feature.

Code must compile and pass tests under both feature sets. On Linux, live audio goes through `audio/linux.rs`, which spawns an external process. Elsewhere it uses `cpal` (`audio/desktop.rs`). `audio::backend` picks between them with `cfg`.

The device binary takes many CLI flags for scenarios, benchmarks, and frame captures (`--mode`, `--map`, `--seed`, `--weather`, `--view`, `--frames`, `--benchmark`, `--capture-dir`, `--imu-test`, …). Run `--help` or see the README for the list. Runtime env vars: `FLIGHT_CLUB_FB`, `FLIGHT_CLUB_KEYBOARD`, `FLIGHT_CLUB_IMU`, `FLIGHT_CLUB_SETTINGS`, `FLIGHT_CLUB_RECORDS`.

## Architecture

The library crate (`src/lib.rs`) holds all game logic. Both entry points are thin shells around `app::App`:

- **`app::App`** (`src/app/mod.rs`) is the whole session state machine: `Page` (Menu/Playing/Paused/Settings/Chart/Results/Help/Loading), the current `World`, `Flight`, `Mission`, the portal `Network`, settings, records, and audio events. Drive it with `App::new(World::generate(kind, seed))`, `app.start()`, `app.key(code, 1|0)`, and `app.step(dt, external_controls)`. Then `app.draw()` renders into `app.screen`, an RGBA `Vec<u8>` at `WIDTH=320 × HEIGHT=170`. Everything renders offscreen into memory, so tests and captures don't need a window or GPU.
- **Input** is unified on Linux evdev key codes (`src/keys.rs`). The desktop maps macroquad `KeyCode`s onto them in `main.rs`. The device reads evdev directly, and `keys::cardputer_direction` remaps the physical F/X/Z/C keys (33/45/44/46) to arrow codes. Several constants intentionally share values because number keys mean different things per mode (e.g. `FIRE` = `WEATHER` = 2). When changing a binding, confirm the code the device actually emits.
- **`world/`** handles procedural terrain generation per `MapKind` (12 maps, including the `Moon` atmosphere planet and the `Space` vacuum map) plus the sampling and height bounds used by flight and render. Generation is slow, so `WorldLoader` / `loader::spawn_world` build worlds on a background thread. `world::network::Network` manages the cross-map wormhole graph and caches and preloads destination worlds.
- **`flight/`** covers flight dynamics, aerobatics, formation wingmen, the camera and projection (`camera.rs`), portal transit timing and space transforms (`portals.rs`), terrain-avoidance safety (`safety.rs`), and surface↔space↔orbit transitions (`space.rs`, `orbit.rs`). `app/travel.rs` and `app/orbit.rs` orchestrate map switches at the session level.
- **`gameplay/`** contains `Mission` for the four `Mode`s (Free, Race, Coins, Combat). It includes the phase director, enemies and combat, weapons, objectives, and progression and loadouts.
- **`render/`** is a CPU software renderer (`Renderer`). It draws sprite-stacked and tilted terrain meshes, clouds, weather, aircraft, landmarks, celestial bodies, the cockpit overlay, and portal views. A portal view is a nested `Renderer` that renders the destination world through the wormhole window. The renderer splits frames across worker `Renderer`s on a shared `rayon_core` pool. Render scale is supported (desktop HD multipliers `1/2/4/8`). On desktop, `src/desktop/mod.rs` runs rendering on a dedicated thread and passes `Scene`s and `Frame`s over channels, recycling buffers.
- **`ui/`** (private module) draws the 2D HUD, menus, map chart, FPV instruments, and perf panel into the same pixel buffer, using `font8x8`.
- **`content/`** reads typed TOML from `content/levels`, `content/maps`, and `content/music`, and the binary asset art in `assets/`. All of it is **embedded at compile time** via `include_str!` / `include_bytes!`, so adding a content file means also adding it to the `include_*` list in `content/levels.rs`, `content/maps.rs`, or `audio/score.rs`. `content::tests` validates the embedded data.

Tests live in `tests.rs` submodules: `src/{app,flight,gameplay,world,audio}/tests.rs` and `src/app/orbit/tests.rs`. They include full-level playthroughs, so a single test can run for a while. Map generation is asynchronous, so tests must wait on the loader rather than busy-loop (see recent commit 96f2704).

## Testing workflow

`skills/flight-club-testing/SKILL.md` (Chinese) documents the full method:
- For numeric behavior, write a temporary probe in `examples/<name>.rs` against the library (`cargo run --example <name>`), stepping `Flight`/`App` at `dt = 1/30`. Delete the probe afterwards and turn stable expectations into `#[test]`s.
- For visual checks, call `app.draw()` (full scene) or `app.draw_overlay()` (HUD only), then dump `app.screen` as a PPM/PNG. Keep artifacts under `target/<topic>/`.
- For on-device checks, use SSH with credentials from `/home/dev/projects/.secrets/.device.env` (outside the repo; never copy it in or print the passwords). Deploy to `/usr/share/APPLaunch/bin/`. End-to-end runs stop `APPLaunch.service`, inject keys through a FIFO set as `FLIGHT_CLUB_KEYBOARD`, and read `/dev/fb_lcd`.

## Conventions

- Commits use Conventional Commits with scopes, e.g. `fix(render): …`, `perf(portals): …`, `feat(settings): …`.
- A release bumps the version in `Cargo.toml`, `Cargo.lock`, `app-builder.json`, and the `.deb` filename in both `README.md` and `README_zh.md`, with a commit `chore(release): package vX.Y.Z …`. Pushing a `v*` tag runs CI, creates the GitHub release, and publishes to the Cardputer store (`publish-store.yml`, metadata in `app-builder.json`).
- `README.md` and `README_zh.md` are kept in parallel. Update both.
