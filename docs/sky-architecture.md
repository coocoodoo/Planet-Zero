# Cardputer Sky — Phase 0 audit and Phase 1 architecture

Cardputer Sky is a procedural space exploration and survival mode built on top of the Flight Club
engine in this repository. It is an original game in the space-exploration genre; it reuses only
this repository's engine and art.

## Phase 0 — what the engine really is

`Plan.md` assumed an ESP32-S3 Arduino/PlatformIO project with a 240×135 ST7789 display. The audit found
something different, and every design decision below follows the real code:

| Plan assumption | What the repository actually is |
| --- | --- |
| ESP32-S3, Arduino / ESP-IDF, C++ | Rust 2024 crate, `aarch64-unknown-linux-musl` static binary for the Cardputer Zero (Linux, Cortex-A53) |
| 240×135 ST7789 over SPI | 320×170 RGB565 Linux framebuffer (`/dev/fb_lcd`), written by mmap (`src/cardputer/linux.rs`), ordered dither in `cardputer::rgb565` |
| M5Cardputer keyboard library | evdev key events (`src/keys.rs`); F/X/Z/C are remapped to arrows by `keys::cardputer_direction` |
| Buzzer | Software synthesizer (`src/audio/`) piped to an external player on Linux (`audio/linux.rs`), cpal on desktop |
| SD card / NVS | Normal Linux files (`~/.local/share/flight-club/`) |
| No gyroscope | BMI270 IMU over IIO sysfs (`src/cardputer/imu.rs`): `Sensor` thread + `Fusion` (calibration, roll/pitch, yaw rate) already steer the aircraft in `cardputer::Game::step` |
| Static pools, no heap in the loop | The engine allocates world maps (4096² heights + materials) on a loader thread and reuses its frame buffers; the new code follows the same rule (fixed arrays, reused `Vec`s) |

**Build system.** Cargo with two feature sets: `desktop` (macroquad window, default) and `cardputer`
(fbdev + evdev + IMU). `scripts/build-cardputer.sh` cross-builds the device binary with the toolchain's
`rust-lld`; it builds unchanged on this PC (1.9 MB). The unmodified repository passed all 169 tests.

**Rendering pipeline.** `render::Renderer` is a CPU software renderer into 32-bit pixels with a per-pixel
depth buffer (`depth`, column-major `x * h + y`). Terrain is a sprite-stacked/tilted heightmap mesh drawn
in parallel column strips on a 3-thread `rayon_core` pool; aircraft, mission objects and portals are drawn
afterwards on the main renderer with depth testing; clouds and weather last. Projection is `flight::Camera`
(float math, `q` = perspective strength). The cockpit view renders into a larger rotated canvas and
composites a pixel-art cockpit. HUD and menus (`src/ui/`) are drawn onto the final RGBA frame with
`font8x8`.

**State machine and transitions.** `app::App` holds `Page` (Menu/Playing/Paused/Settings/Chart/Results/
Help/Loading), the current `World`, the `Flight`, the `Mission` and the wormhole `Network`.
Surface → space → planet transitions live in `app/orbit.rs` + `world/orbit.rs`: holding climb above
`ARM_ALTITUDE` ignites the orbital burn, the terrain bends onto a sphere (`SurfaceWarp`), and at orbit
altitude the session swaps to the `Space` map; flying at a body from space reverses it. Exactly two bodies
exist (`Body::Moon`, `Body::Earth`) with fixed centres and radii; the Earth slot shows whichever surface map
`orbit.earth` names. Wormholes (`world/network.rs`) are a ring of links between the 12 stock maps; flying
through one swaps worlds (`app/travel.rs`) with a live preview of the destination through the aperture.

**Memory and performance.** A world is ~16 MB of map data (compacted); the network caches at most two
neighbouring worlds. Frames render at 320×170 on three worker threads; `docs/performance.md` records
~30 fps on the device. New per-frame work must stay small: Cardputer Sky adds at most a few hundred
triangles (props, ship, station, asteroids) and a 2D overlay.

## Phase 1 — architecture

New code lives in `src/sky/` (game logic, no rendering) and `src/ui/sky.rs` + `src/ui/sky_icons.rs`
(drawing), with thin hooks into the engine:

| Module | Responsibility |
| --- | --- |
| `sky/rng.rs` | `splitmix64`, `xorshift32`, coordinate hashing, sine lookup table |
| `sky/names.rs` | Syllable name generator seeded by id |
| `sky/galaxy.rs` | Galaxy grid `(gx, gy, gz)`, star systems (star class, 2–5 bodies, station, asteroid field), neighbours and warp range |
| `sky/biome.rs` | Seven biomes: terrain archetype, colour grade (sky/fog/ground tint), hazard, resource table |
| `sky/props.rs` | Chunk-hashed flora, rocks, crystals and points of interest; low-poly meshes with distance LOD |
| `sky/inventory.rs` | 16-slot inventory, elements, crafting recipes, recharge rules |
| `sky/survival.rs` | Life support, hazard protection, jetpack, tool heat and charge, ship shield, pulse drive |
| `sky/walker.rs` | On-foot first-person movement over the heightmap: gravity, slope limit, jetpack |
| `sky/station.rs` | Station trading prices and upgrade costs |
| `sky/save.rs` | Plain-text save file (location, inventory, credits, upgrades, discovered systems) |
| `sky/mod.rs` | `Sky` session state and the explore state machine (Flying → Landing → OnFoot → Takeoff, Docking → Docked, Warping) |
| `app/sky.rs` | Glue: keys, per-frame step, binding star-system planets to the engine's two orbital body slots |
| `ui/sky.rs` | Flight and on-foot HUDs, 4–8 key bar, galaxy and system maps, inventory, station terminal |

**Engine hooks (kept minimal):**

* `gameplay::Mode::Explore` — a fifth mode that behaves like Free Flight for orbit and physics.
* `flight::Flight::eye` — optional first-person camera used on foot (`Camera::new` honours it; the renderer
  then hides the player's aircraft, its shadow and the cockpit).
* `render::Renderer::extras` — extra lit triangles and parked aircraft drawn with depth after the terrain.
* `world::network::Network` — surface wormholes can be switched off, and a one-way warp tunnel to another
  system's space map can be opened. Warps reuse the engine's own wormhole transit.
* `app/orbit.rs` — the space map key and the two body slots come from the current star system.
* A per-planet colour grade applied to the finished 3D frame (before the HUD) gives each biome and star its
  own sky, fog and ground colours without touching the terrain generator.

**Planets and the two body slots.** Every star system has 1–4 atmospheric planets plus one airless moon.
The engine's `Earth` slot shows the planet the player has set course for (each planet is a surface map:
biome archetype + planet seed), the `Moon` slot shows the system's moon (a `Moon` map with the moon's seed).
Choosing another planet in the system map performs a short in-system jump that rebinds the slot. This keeps
the engine's tested orbit, curvature and landing code unchanged.

**Controls** follow M5Stack's Recommended Key Mapping Design: the most frequent actions are on keys 4–8
with 22×22 icons centred at x = 49, 105, 161, 217, 273, y = 153 and a tick pointing at each key; F/Z/X/C are
always arrows (no text input anywhere); other keys are listed only in the help overlay and never drawn in
columns that line up with keys 4–8. The ship is piloted with the gyroscope (tilt to bank and climb), as in
Flight Club; arrow keys override it.

## Status: phases 1–5 implemented

| Phase | What exists | Where |
| --- | --- | --- |
| 2 Galaxy and biomes | 14,260 systems on a 97×97×13 grid, star classes with light tint, 2–5 bodies, names, 7 biomes, galaxy and system maps, warp through the engine's wormhole | `sky/galaxy.rs`, `sky/biome.rs`, `sky/names.rs`, `app/sky.rs` (`sky_warp`, `set_course`), `world/network.rs` (`open_warp`) |
| 3 Landing and on foot | Landing on flat dry ground, parked ship with beacon, first-person walking on the rendered mesh, jetpack, take-off using lift fuel, chunk-hashed flora, rocks, crystals, monoliths, pods and deposits with distance LOD | `sky/walker.rs`, `sky/props.rs`, `flight/camera.rs` (`walking`), `render/extras.rs` |
| 4 Tools and survival | Scanner with wireframe frames and discoveries, mining beam on props and asteroids, tool heat and charge, life support, hazard protection, 16-slot inventory, crafting | `sky/survival.rs`, `sky/inventory.rs`, `app/sky.rs`, `ui/sky.rs` |
| 5 Station, audio, save | Autopilot docking around planets, station trading, repairs and upgrades, six synthesized effects, text save with atomic writes, prop cost trimmed after profiling | `sky/station.rs`, `audio/effects.rs`, `sky/save.rs` |

Verification: 25 new unit and integration tests (`cargo test` in both feature sets: 194 and 205
tests pass), clippy clean with `-D warnings` for both feature sets, the aarch64 device binary
cross-builds, and the PC emulator (`src/bin/emulator.rs`) plays a full session through the device's
own key and gyroscope code with 17 checks plus a save-and-resume script. See `docs/cardputer-sky.md`.

## Version 2: first person, weapons, guard drones

| Part | What exists | Where |
| --- | --- | --- |
| First person | Turn ramp with tap nudge, double-tap turn-around, auto-level, tilt fine-aim from the BMI270 fusion (via `cardputer::Game::step`), aim assist, context key 7 | `sky/walker.rs`, `app/sky_foot.rs`, `cardputer/mod.rs` |
| Arsenal | 7 archetypes, 4 rarities, 16 perks, deterministic from `(archetype, seed, rarity)`; loadout of 3 plus a locker of 6, per-slot heat | `sky/arsenal.rs`, `sky/mod.rs` (`Loadout`) |
| Combat | Beams, bolts, lances, ballistic mortar, chain arcs; projectiles with splash, ricochet, homing and pierce; a hit trace that honours the aimed prop | `sky/combat.rs`, `app/sky_foot.rs` |
| Guard drones | Scout, warden, hive; alert level, patrols, line-of-sight fire with a telegraph, suit shield, drops | `sky/drones.rs`, `sky/survival.rs` |
| Content | Weapon caches, ruins, beacons, lore, station armory, drone cores, reroll and shield cells, suit and jetpack upgrades | `sky/props.rs`, `sky/lore.rs`, `sky/inventory.rs`, `sky/station.rs` |
| UI | Weapon card, radar and alert meter, drone health, hit markers, damage arcs, weapons view, inscription panel, 8 new icons, settings rows | `ui/sky.rs`, `ui/sky_icons.rs`, `ui/menus.rs`, `app/settings.rs` |
| Save | Version 2 (weapons, locker, counters); version 1 files still load | `sky/save.rs` |

Verification: 213 tests (desktop) and 224 (device feature) pass, clippy is clean for both, the device
binary cross-builds, and three emulator scripts pass (`tour.txt`, `continue.txt`, `combat.txt`).

## Version 3: stations aboard, missions, pirates, ship mining

| Part | What exists | Where |
| --- | --- | --- |
| Station interior | `Phase::Station`: docking ends on foot in a hangar built on the space map's flat zero plane at `interior::ORIGIN`, far from every body. The walker uses `Walker::step_on` with flat ground and `interior::clamp`; the ship is drawn on the pad as a `Flight` clone. Five people (agent, trader, outfitter, technician, a walking traveller) of one of three original species, drawn as extra triangles | `sky/interior.rs`, `app/sky_space.rs`, `sky/walker.rs` |
| Missions | 16 kinds, tiers, rewards and prizes; a board of six per visit, three active; event-driven progress (`missions::Event` from scans, kills, landings, arrivals, docking and more); guild standing and ranks. Replaces the v2 single contract | `sky/missions.rs`, `app/sky_space.rs` |
| Pirates | `Pirate::step` (attack and break-off with a limited turn rate, lead-aimed bolts); random raids every 30 min to 3 h of play (`pirates::RAID_MIN/MAX`, timer saved), bounty and ace waves; drawn as engine aircraft through `render::Extras` ships; ship shield damage and towing | `sky/pirates.rs`, `app/sky_space.rs` |
| Ship weapons and mining | Key 5 mines the asteroid under the crosshair, otherwise fires cannons with lead and a 20 degree assist; rock kinds (common, rich, rare) with veins; belts out past the station so the flight there stays clear of the atmosphere; regrowth every 300 s | `app/sky_space.rs`, `sky/galaxy.rs` |
| Voxel portraits | Each person's bust is a 16×12×22 voxel grid built from species and role (`sky::bust`), ray-traced into the talk panel with per-face light, glowing cubes and seams between cubes (`ui::sky_bust`); about 1 ms a frame on a PC, only while talking | `sky/bust.rs`, `ui/sky_bust.rs` |
| UI | Hangar HUD, talk panel, mission board and log, mission tracker, pirate markers and health, TALK icon, two help pages | `ui/sky.rs`, `ui/sky_icons.rs`, `ui/menus.rs` |
| Save | Version 3 (missions, standing, missions done, raid timer); versions 1 and 2 still load | `sky/save.rs` |

Emulator: new `talk`, `accept`, `claim`, `dogfight` and `mine-asteroids` commands, and
`scripts/emulator/station.txt` for the whole loop.
