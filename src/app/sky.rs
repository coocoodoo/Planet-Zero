//! Cardputer Sky glue: connects `crate::sky` (galaxy, survival, inventory…) to the engine's
//! session (`App`), flight model, orbit transitions, wormhole network and renderer.
use super::*;
use crate::audio::Sound;
use crate::flight::heading;
use crate::keys::*;
use crate::sky::galaxy::SystemId;
use crate::sky::inventory::{Item, RECIPES};
use crate::sky::props::{self, PropKind};
use crate::sky::save::{Place, SaveData};
use crate::sky::station::{self, Upgrade};
use crate::sky::survival::Event;
use crate::sky::walker::{WalkInput, Walker};
use crate::sky::{
    LANDING_SECONDS, Location, Notice, Overlay, Parked, Phase, Sky, TAKEOFF_SECONDS, Target,
};
use crate::world::{MapKind, SIZE, network::MapKey, orbit::Body};
use glam::{Vec2, Vec3, vec2, vec3};

/// Highest point above the ground from which you can land.
pub const LAND_ALTITUDE: f32 = 190.0;
pub const BOARD_DISTANCE: f32 = 12.0;
const SHELTER_DISTANCE: f32 = 10.0;
/// Height of the parked ship's centre above the ground.
const SHIP_REST: f32 = 3.0;
pub const DOCK_DISTANCE: f32 = 300.0;
const AUTOSAVE_SECONDS: f32 = 45.0;
pub const CROSSHAIR: Vec2 = Vec2::new(160.0, 71.4);

const CYAN: [u8; 3] = [111, 237, 218];
const GOLD: [u8; 3] = [255, 208, 105];
const RED: [u8; 3] = [255, 112, 130];
const WHITE: [u8; 3] = [225, 238, 244];

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl App {
    pub fn explore(&self) -> bool {
        self.mode == Mode::Explore && self.sky.active
    }
    /// Space map of the current session: the star system's own in Cardputer Sky.
    pub(crate) fn space_key(&self) -> MapKey {
        if self.explore() {
            self.sky.system.space_key()
        } else {
            MapKey {
                kind: MapKind::Space,
                seed: MapKind::Space.seed(),
            }
        }
    }
    /// Surface map shown in an orbital body slot in Cardputer Sky.
    pub(crate) fn sky_body_key(&self, body: Body) -> Option<MapKey> {
        if !self.explore() {
            return None;
        }
        Some(match body {
            Body::Earth => self.sky.system.planets[self.sky.course].key(),
            Body::Moon => self.sky.system.moon().key(),
        })
    }
    /// Display name of an orbital body (the star system's planet names in Cardputer Sky).
    pub fn body_name(&self, body: Body) -> String {
        if let Some(i) = self
            .sky_body_key(body)
            .and_then(|key| self.sky.system.planet_of(key))
        {
            return self.sky.system.planets[i].name.clone();
        }
        body.name().to_owned()
    }
    /// Whether the gyroscope should steer right now (only while piloting the ship).
    pub fn sky_motion(&self) -> bool {
        !self.explore()
            || (self.sky.phase == Phase::Flying
                && self.sky.overlay == Overlay::None
                && !self.sky.autodock)
    }

    // ----- session start and end -------------------------------------------------------

    fn sky_target_key(&self) -> MapKey {
        let system = &self.sky.system;
        match self.sky.spawn {
            Some(Place::Space(..)) => system.space_key(),
            Some(Place::Flying(i, ..)) | Some(Place::Landed { planet: i, .. }) => {
                system.planets[i.min(system.planets.len() - 1)].key()
            }
            None => system.planets[0].key(),
        }
    }
    /// Called from `start()` in Explore mode. Returns false while the right world still loads.
    pub(super) fn sky_prepare(&mut self) -> bool {
        if !self.sky.active {
            let mut sky = Sky::default();
            if let Some(save) = SaveData::load() {
                sky.apply_save(&save);
                sky.spawn = Some(save.place);
            }
            sky.active = true;
            self.sky = sky;
            self.renderer.extras.clear();
        }
        self.network.surface_links = false;
        let key = self.sky_target_key();
        if MapKey::of(&self.world) != key {
            if self.loader.pending.is_none() {
                self.loader.start(key.kind, key.seed);
            }
            return false;
        }
        true
    }
    /// Called at the end of `start()` once the world is right: place the player.
    pub(super) fn sky_enter(&mut self) {
        self.auto = false;
        self.flight.set_demo(false);
        self.flight.hover = false;
        self.flight.eye = None;
        self.flight.wing_count = 0;
        // chase view shows the ship landing and taking off
        self.flight.snap_view(1);
        self.sky.phase = Phase::Flying;
        self.sky.overlay = Overlay::None;
        self.sky.autodock = false;
        self.sky.location = Location::Elsewhere;
        self.sky_sync_location();
        let world = self.world.clone();
        match self.sky.spawn.take() {
            Some(Place::Landed { planet, ship, yaw })
                if self.sky.location == Location::Planet(planet) =>
            {
                let ground = crate::sky::walker::ground(&world, ship.x, ship.y);
                self.park(planet, vec3(ship.x, ship.y, ground + SHIP_REST), yaw);
            }
            Some(Place::Flying(_, p, yaw)) => {
                let ground = world.height(p.x, p.y);
                let p = vec3(p.x, p.y, p.z.max(ground + 60.0));
                self.flight.translate(p - self.flight.position);
                self.flight.yaw = yaw;
            }
            Some(Place::Space(p, _)) => {
                self.flight.translate(p - self.flight.position);
            }
            _ => {
                // start well inside the terrain map, heading for its middle
                if self.sky.location != Location::Space {
                    let middle = SIZE as f32 * 0.5;
                    let p = self.flight.position;
                    let inside = vec2(
                        p.x.clamp(700.0, SIZE as f32 - 700.0),
                        p.y.clamp(700.0, SIZE as f32 - 700.0),
                    );
                    let ground = world.height(inside.x, inside.y);
                    let target = inside.extend(p.z.max(ground + 120.0));
                    self.flight.translate(target - p);
                    let d = vec2(middle, middle) - inside;
                    if d.length() > 1.0 {
                        self.flight.yaw = d.x.atan2(-d.y);
                    }
                }
                let name = self.sky.system.name.clone();
                self.sky.notice(format!("WELCOME TO {name}"), CYAN);
                self.sky.notice("6 LANDS NEAR THE GROUND", WHITE);
            }
        }
        self.flight.follow = self.flight.position;
    }
    /// Parked ship plus the walker standing beside it.
    fn park(&mut self, planet: usize, position: Vec3, yaw: f32) {
        let world = self.world.clone();
        let mut ship = self.flight.clone();
        ship.wingmen.clear();
        ship.trails.clear();
        ship.eye = None;
        ship.space_attitude = None;
        ship.position = position;
        ship.follow = position;
        ship.yaw = yaw;
        ship.pitch = 0.0;
        ship.bank = 0.0;
        ship.spin = 0.0;
        ship.speed = 0.0;
        ship.boost = 0.0;
        self.sky.ship = Some(ship);
        self.sky.parked = Some(Parked {
            planet,
            position,
            yaw,
        });
        let (_, right) = heading(yaw);
        let spot = position.truncate() + right.truncate() * 7.0;
        self.sky.walker = Walker::new(&world, spot.x, spot.y, yaw - 0.5);
        self.sky.phase = Phase::OnFoot;
        self.sky_recenter();
        self.flight.eye = Some(self.foot_eye());
        self.flight.speed = 0.0;
        self.flight.boost = 0.0;
        self.flight.position = position;
        self.flight.follow = position;
        self.flight.trails.clear();
    }
    fn sky_place(&self) -> Place {
        match (self.sky.location, self.sky.parked) {
            (Location::Planet(i), Some(p)) if p.planet == i => Place::Landed {
                planet: i,
                ship: p.position,
                yaw: p.yaw,
            },
            (Location::Planet(i), _) => Place::Flying(i, self.flight.position, self.flight.yaw),
            _ => Place::Space(self.flight.position, self.flight.yaw),
        }
    }
    /// Write the save file (skipped mid-transition, when positions are in flux).
    pub fn sky_save(&mut self) {
        if !self.explore()
            || self.orbit.transition.is_some()
            || self.sky.location == Location::Elsewhere
            || self.travel.is_some()
        {
            return;
        }
        let place = match self.sky.phase {
            Phase::Docking { .. } | Phase::Docked | Phase::Station => {
                Place::Space(self.sky.system.station + vec3(420.0, 0.0, 0.0), 0.0)
            }
            _ => self.sky_place(),
        };
        self.sky.save_error = self.sky.to_save(place).store().is_err();
        self.sky.save_timer = 0.0;
    }
    /// Leaving Cardputer Sky (main menu): save and restore engine defaults.
    pub(super) fn sky_leave(&mut self) {
        if self.mode == Mode::Explore && self.sky.active {
            self.sky_save();
        }
        self.sky.active = false;
        self.sky.overlay = Overlay::None;
        self.network.surface_links = true;
        self.network.close_warp();
        self.flight.eye = None;
        self.renderer.extras.clear();
    }

    // ----- location tracking ------------------------------------------------------------

    pub(super) fn sky_sync_location(&mut self) {
        let key = MapKey::of(&self.world);
        let mut arrived_system = None;
        let location = if key.kind == MapKind::Space {
            if key.seed == self.sky.system.space_seed {
                Location::Space
            } else if let Some(id) = self.sky.warp_target
                && self.sky.galaxy.system(id).space_seed == key.seed
            {
                arrived_system = Some(id);
                Location::Space
            } else {
                Location::Elsewhere
            }
        } else {
            self.sky
                .system
                .planet_of(key)
                .map_or(Location::Elsewhere, Location::Planet)
        };
        if let Some(id) = arrived_system {
            let new = self.sky.enter_system(id);
            self.mission_event(crate::sky::missions::Event::Arrived { new });
            self.network.close_warp();
            self.flight.planet_maps = [None, None];
            let system = &self.sky.system;
            let text = format!("{} / {} STAR", system.name, system.class.name());
            let bodies = format!("{} PLANETS + 1 MOON", system.atmospheric());
            self.sky.notice(text, CYAN);
            self.sky.notice(bodies, WHITE);
            if new {
                self.sky.credits += 200;
                self.sky.data += 3;
                self.sky.notice("NEW SYSTEM +200 CR", GOLD);
            }
            self.sky.location = Location::Space;
            self.sky_save();
            return;
        }
        if location == self.sky.location {
            return;
        }
        self.sky.location = location;
        self.sky.clear_combat();
        if let Location::Planet(i) = location {
            let planet = self.sky.system.planets[i].clone();
            let (hazard, _) = planet.biome.hazard();
            self.sky.notice(planet.name.clone(), CYAN);
            self.sky.notice(
                format!("{} / {}", planet.biome.name(), hazard.name()),
                if hazard == crate::sky::biome::Hazard::Safe {
                    WHITE
                } else {
                    GOLD
                },
            );
            self.sky.props_planet = None;
        }
        if location == Location::Space {
            self.sky.parked = None;
            self.sky.ship = None;
        }
    }

    // ----- keys -------------------------------------------------------------------------

    /// Cardputer Sky key presses while playing. Returns true when the key was used.
    pub(super) fn sky_key(&mut self, code: u16) -> bool {
        if matches!(code, HELP | SETTINGS | SOUND | PERF | HUD) {
            return false;
        }
        if code == KEY_H {
            self.help();
            return true;
        }
        match self.sky.overlay {
            Overlay::Galaxy => return self.galaxy_key(code),
            Overlay::System => return self.system_key(code),
            Overlay::Inventory => return self.inventory_key(code),
            Overlay::Station => return self.station_key(code),
            Overlay::Talk => return self.talk_key(code),
            Overlay::Missions => return self.missions_key(code),
            Overlay::Lore => {
                if matches!(code, KEY8 | ENTER | KEY4 | KEY_E) {
                    self.sky.overlay = Overlay::None;
                }
                return !matches!(code, PAUSE);
            }
            Overlay::None => {}
        }
        if code == RETRY {
            return true;
        }
        match self.sky.phase {
            Phase::OnFoot => {
                match code {
                    KEY4 | KEY_V => self.sky_scan(),
                    KEY7 | KEY_E => self.sky_interact(),
                    ROLL_LEFT => self.swap_weapon(1),
                    KEY8 | TAB => self.open_overlay(Overlay::Inventory),
                    KEY_G => self.open_overlay(Overlay::Galaxy),
                    DOWN => {
                        // double tap back turns around
                        if self.sky.time - self.sky.back_tap < 0.3 {
                            self.sky.walker.turn_around();
                        }
                        self.sky.back_tap = self.sky.time;
                    }
                    PAUSE | CHART => return false,
                    _ => {}
                }
                // movement, look, jetpack and mining are read as held keys in sky_step
                true
            }
            Phase::Flying => match code {
                KEY4 | KEY_V => {
                    self.sky_scan();
                    true
                }
                KEY5 => true,
                KEY6 | KEY_L => {
                    if self.sky.location == Location::Space {
                        self.sky.autodock = !self.sky.autodock;
                        let text = if self.sky.autodock {
                            "AUTOPILOT TO STATION"
                        } else {
                            "AUTOPILOT OFF"
                        };
                        self.sky.notice(text, CYAN);
                        self.sounds.push(Sound::Toggle(self.sky.autodock));
                    } else {
                        self.sky_land();
                    }
                    true
                }
                KEY7 | KEY_G => {
                    self.open_overlay(Overlay::Galaxy);
                    true
                }
                KEY8 | TAB => {
                    self.open_overlay(Overlay::Inventory);
                    true
                }
                UP | DOWN | LEFT | RIGHT if self.sky.autodock => {
                    self.sky.autodock = false;
                    self.sky.notice("AUTOPILOT OFF", CYAN);
                    false
                }
                _ => false,
            },
            Phase::Docked => true,
            Phase::Station => {
                match code {
                    KEY7 | KEY_E => self.sky_interact(),
                    KEY4 => {
                        self.open_overlay(Overlay::Missions);
                        self.sky.mission_tab = 1;
                        self.sky.mission_select = 0;
                    }
                    KEY8 | TAB => self.open_overlay(Overlay::Inventory),
                    KEY_G => self.open_overlay(Overlay::Galaxy),
                    PAUSE | CHART => return false,
                    _ => {}
                }
                true
            }
            _ => !matches!(code, PAUSE),
        }
    }
    pub(super) fn open_overlay(&mut self, overlay: Overlay) {
        self.sky.overlay = overlay;
        self.sky.mining = false;
        self.sky.beam = None;
        self.sounds.push(Sound::Confirm);
        match overlay {
            Overlay::Galaxy => self.sky.refresh_neighbors(),
            Overlay::System => {
                self.sky.system_select = self.sky.course;
            }
            Overlay::Inventory => {
                self.sky.crafting = false;
                self.sky.arms = false;
            }
            _ => {}
        }
    }
    /// ESC with a panel open closes it (undocks at a station).
    pub(super) fn sky_back(&mut self) -> bool {
        if !self.explore() || self.page != Page::Playing {
            return false;
        }
        match self.sky.overlay {
            Overlay::None => false,
            Overlay::Station if self.sky.phase != Phase::Station => {
                self.sky_undock();
                true
            }
            _ => {
                self.sky.overlay = Overlay::None;
                true
            }
        }
    }
    fn galaxy_key(&mut self, code: u16) -> bool {
        let n = self.sky.neighbors.len().max(1);
        match code {
            LEFT | KEY4 => self.sky.map_select = (self.sky.map_select + n - 1) % n,
            RIGHT | KEY5 => self.sky.map_select = (self.sky.map_select + 1) % n,
            UP => self.sky.map_tilt = (self.sky.map_tilt + 0.15).min(1.4),
            DOWN => self.sky.map_tilt = (self.sky.map_tilt - 0.15).max(-0.2),
            KEY6 | ENTER => self.sky_warp(),
            KEY7 => self.open_overlay(Overlay::System),
            KEY8 | KEY_G | TAB => self.sky.overlay = Overlay::None,
            _ => return !matches!(code, PAUSE),
        }
        if matches!(code, LEFT | RIGHT | KEY4 | KEY5 | UP | DOWN) {
            self.sounds.push(Sound::Navigate);
        }
        true
    }
    fn system_key(&mut self, code: u16) -> bool {
        let n = self.sky.system.planets.len();
        match code {
            UP | LEFT | KEY4 => self.sky.system_select = (self.sky.system_select + n - 1) % n,
            DOWN | RIGHT | KEY5 => self.sky.system_select = (self.sky.system_select + 1) % n,
            KEY6 | ENTER => self.set_course(self.sky.system_select),
            KEY7 => self.open_overlay(Overlay::Galaxy),
            KEY8 | KEY_G | TAB => self.sky.overlay = Overlay::None,
            _ => return !matches!(code, PAUSE),
        }
        if matches!(code, LEFT | RIGHT | KEY4 | KEY5 | UP | DOWN) {
            self.sounds.push(Sound::Navigate);
        }
        true
    }
    fn inventory_key(&mut self, code: u16) -> bool {
        if self.sky.arms {
            return self.arms_key(code);
        }
        let sky = &mut self.sky;
        if sky.crafting {
            let n = RECIPES.len();
            match code {
                UP | LEFT => sky.craft_select = (sky.craft_select + n - 1) % n,
                DOWN | RIGHT => sky.craft_select = (sky.craft_select + 1) % n,
                KEY6 | ENTER => self.sky_craft(),
                KEY5 => sky.crafting = false,
                KEY8 | TAB => sky.overlay = Overlay::None,
                _ => return !matches!(code, PAUSE),
            }
        } else {
            let s = sky.inventory_select;
            match code {
                UP => sky.inventory_select = (s + 12) % 16,
                DOWN => sky.inventory_select = (s + 4) % 16,
                LEFT => sky.inventory_select = (s / 4) * 4 + (s % 4 + 3) % 4,
                RIGHT => sky.inventory_select = (s / 4) * 4 + (s % 4 + 1) % 4,
                KEY4 | ENTER => self.sky_use(),
                KEY5 => sky.crafting = true,
                KEY6 => {
                    sky.arms = true;
                    sky.arms_select = sky.loadout.active;
                }
                KEY7 => self.open_overlay(Overlay::Galaxy),
                KEY8 | TAB => sky.overlay = Overlay::None,
                _ => return !matches!(code, PAUSE),
            }
        }
        if matches!(code, LEFT | RIGHT | UP | DOWN) {
            self.sounds.push(Sound::Navigate);
        }
        true
    }
    fn station_key(&mut self, code: u16) -> bool {
        let rows = self.station_rows();
        match code {
            KEY4 => self.sky.station_tab = 0,
            KEY5 => self.sky.station_tab = 1,
            KEY6 => self.sky.station_tab = 2,
            UP => self.sky.station_select = (self.sky.station_select + rows - 1) % rows,
            DOWN => self.sky.station_select = (self.sky.station_select + 1) % rows,
            KEY7 | ENTER => self.station_confirm(),
            KEY8 if self.sky.phase == Phase::Station => self.sky.overlay = Overlay::None,
            KEY8 => self.sky_undock(),
            _ => return !matches!(code, PAUSE),
        }
        if matches!(code, KEY4 | KEY5 | KEY6) {
            self.sky.station_select = 0;
            self.sounds.push(Sound::Navigate);
        }
        if matches!(code, UP | DOWN) {
            self.sounds.push(Sound::Navigate);
        }
        true
    }

    // ----- per-frame ----------------------------------------------------------------------

    /// Explore-mode frame. Returns true when it handled the whole frame (the engine's flight
    /// step is skipped): panels open, landing, on foot, take-off, docking.
    pub(super) fn sky_step(&mut self, dt: f32) -> bool {
        self.sky.tick(dt);
        let hud = self.sky.overlay == Overlay::None
            && matches!(self.sky.phase, Phase::Flying | Phase::OnFoot);
        let held = !self.keys.is_empty();
        self.sky.bar.step(dt, hud, held);
        self.sky_sync_location();
        if self.sky.overlay != Overlay::None {
            return true;
        }
        match self.sky.phase {
            Phase::Flying => false,
            Phase::Landing { t, from, to } => {
                self.step_landing(dt, t, from, to);
                true
            }
            Phase::Takeoff { t, from } => {
                self.step_takeoff(dt, t, from);
                true
            }
            Phase::OnFoot => {
                self.step_on_foot(dt);
                true
            }
            Phase::Docking { t, from } => {
                self.step_docking(dt, t, from);
                true
            }
            Phase::Docked => true,
            Phase::Station => {
                self.step_station(dt);
                true
            }
        }
    }
    /// Adjust flight controls before the engine's flight step.
    pub(super) fn sky_controls(&mut self, controls: &mut Controls) {
        if self.sky.vitals.pulse <= 0.0 {
            controls.boost = false;
        }
        if self.sky.autodock && self.sky.location == Location::Space {
            let delta = self.sky.system.station - self.flight.position;
            controls.direction = Some(autopilot_heading(
                self.flight.position,
                self.sky.system.station,
            ));
            controls.boost = delta.length() > 1500.0 && self.sky.vitals.pulse > 0.2;
            controls.manual = false;
        }
    }
    /// Explore-mode bookkeeping after the engine's flight step.
    pub(super) fn sky_after_flight(&mut self, dt: f32) {
        self.sky_sync_location();
        let boosting = self.flight.boost > 0.5 && self.sky.location == Location::Space;
        self.sky.vitals.step_ship(dt, boosting);
        let drive = self.sky.upgrades.drive();
        if boosting && drive > 1.0 {
            let forward = self.flight.axes()[1];
            self.flight
                .translate(forward * self.flight.speed * (drive - 1.0) * dt);
        }
        match self.sky.location {
            Location::Space => self.space_step(dt),
            Location::Planet(_) => {
                let low = self.flight.position.z
                    - self
                        .world
                        .height(self.flight.position.x, self.flight.position.y)
                    < 400.0;
                if low && self.orbit.transition.is_none() {
                    self.refresh_props(self.flight.position.truncate());
                }
                self.sky.mining = false;
                self.sky.beam = None;
            }
            Location::Elsewhere => {}
        }
        self.autosave(dt);
    }
    fn autosave(&mut self, dt: f32) {
        self.sky.save_timer += dt;
        if self.sky.save_timer > AUTOSAVE_SECONDS {
            self.sky_save();
        }
    }
    fn space_step(&mut self, dt: f32) {
        let position = self.flight.position;
        // asteroid collisions
        for i in 0..self.sky.system.asteroid_count {
            if self.sky.destroyed.contains(&i) {
                continue;
            }
            let (center, radius) = self.sky.system.asteroid(i);
            let delta = position - center;
            let reach = radius + 12.0;
            if delta.length_squared() < reach * reach {
                let normal = delta.try_normalize().unwrap_or(Vec3::Z);
                self.flight.translate(center + normal * reach - position);
                self.flight.speed *= 0.4;
                self.sky.vitals.shield = (self.sky.vitals.shield - 0.18).max(0.0);
                self.sounds.push(Sound::Damage);
                self.sky.notice("HULL IMPACT", RED);
                break;
            }
        }
        // key 5: mine the asteroid under the crosshair, otherwise fire the cannons
        if self.keys.contains(&KEY5) && self.space_aim_rock() {
            self.mine_asteroid(dt);
        } else {
            self.sky.mining = false;
            self.sky.beam = None;
            if self.keys.contains(&KEY5) {
                self.fire_cannon(dt);
            } else {
                self.sky.vitals.cool(dt);
            }
        }
        self.pirate_step(dt);
        self.regrow_asteroids(dt);
        // docking
        let distance = position.distance(self.sky.system.station);
        if distance < DOCK_DISTANCE && self.orbit.transition.is_none() {
            self.sky.phase = Phase::Docking {
                t: 0.0,
                from: position,
            };
            self.sky.autodock = false;
            self.sky.notice("DOCKING CLEARANCE GRANTED", CYAN);
            self.sounds.push(Sound::Dock);
        }
    }
    fn step_landing(&mut self, dt: f32, t: f32, from: Vec3, to: Vec3) {
        let t = t + dt / LANDING_SECONDS;
        let s = smooth(t);
        let height = from.z + (to.z - from.z) * (1.0 - (1.0 - s).powi(2));
        let p = from.truncate().lerp(to.truncate(), s).extend(height);
        self.flight.position = p;
        self.flight.pitch *= 1.0 - (4.0 * dt).min(1.0);
        self.flight.bank *= 1.0 - (4.0 * dt).min(1.0);
        self.flight.speed *= 1.0 - (2.5 * dt).min(1.0);
        self.flight.time += dt;
        let world = self.world.clone();
        self.flight.update_visuals(dt, &world);
        if t >= 1.0 {
            let Location::Planet(planet) = self.sky.location else {
                self.sky.phase = Phase::Flying;
                return;
            };
            self.park(planet, to, self.flight.yaw);
            self.sounds.push(Sound::Landing);
            self.mission_event(crate::sky::missions::Event::Landed);
            let hash = self.sky.system.seed ^ ((planet as u64 + 1) * 0x9e37);
            if self.sky.scanned.insert(hash) {
                self.sky.credits += 300;
                self.sky.discoveries += 1;
                let name = self.sky.system.planets[planet].name.clone();
                self.sky.notice(format!("FIRST LANDING {name} +300"), GOLD);
            }
            self.sky.notice("7 OR E BOARDS THE SHIP", WHITE);
            self.sky_save();
        } else {
            self.sky.phase = Phase::Landing { t, from, to };
        }
    }
    fn step_takeoff(&mut self, dt: f32, t: f32, from: Vec3) {
        let t = t + dt / TAKEOFF_SECONDS;
        let s = smooth(t);
        let (forward, _) = heading(self.flight.yaw);
        self.flight.position = from + Vec3::Z * 46.0 * s + forward * 30.0 * s * s;
        self.flight.pitch = 0.18 * s;
        self.flight.speed = self.flight.vehicle.speed() * s * 0.6;
        self.flight.time += dt;
        let world = self.world.clone();
        self.flight.update_visuals(dt, &world);
        if t >= 1.0 {
            self.sky.phase = Phase::Flying;
            self.sky.parked = None;
            self.sky.ship = None;
            self.sky_save();
        } else {
            self.sky.phase = Phase::Takeoff { t, from };
        }
    }
    fn step_docking(&mut self, dt: f32, t: f32, from: Vec3) {
        let t = t + dt / crate::sky::DOCKING_SECONDS;
        let bay = self.sky.system.station + vec3(58.0, 0.0, 0.0);
        let p = from.lerp(bay, smooth(t));
        self.flight.translate(p - self.flight.position);
        self.flight.speed *= 1.0 - (2.0 * dt).min(1.0);
        self.flight.time += dt;
        let world = self.world.clone();
        self.flight.update_visuals(dt, &world);
        if t >= 1.0 {
            self.sky.station_tab = 0;
            self.sky.station_select = 0;
            self.enter_station();
            let v = &mut self.sky.vitals;
            v.pulse = 1.0;
            v.life = 1.0;
            v.hazard = 1.0;
            self.sounds.push(Sound::Dock);
            self.sky_save();
        } else {
            self.sky.phase = Phase::Docking { t, from };
        }
    }
    pub(super) fn sky_undock(&mut self) {
        self.sky.overlay = Overlay::None;
        self.sky.phase = Phase::Flying;
        self.flight.eye = None;
        self.sky.ship = None;
        let exit = self.sky.system.station + vec3(380.0, 0.0, 0.0);
        self.flight.translate(exit - self.flight.position);
        // face away from the station along +X
        self.flight.space_attitude = Some(glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
        self.flight.speed = 60.0;
        self.sky.notice("UNDOCKED", CYAN);
        self.sounds.push(Sound::Dock);
        self.sky_save();
    }
    fn walk_input(&self) -> WalkInput {
        let held = |key| f32::from(self.keys.contains(&key));
        WalkInput {
            forward: held(UP) - held(DOWN),
            strafe: held(LOOK_RIGHT) - held(LOOK_LEFT),
            turn: held(RIGHT) - held(LEFT),
            look: held(LOOK_UP) - held(LOOK_DOWN),
            jet: self.keys.contains(&KEY6) || self.keys.contains(&VIEW),
            run: self.keys.contains(&BOOST) || self.keys.contains(&BOOST_RIGHT),
            level: !self.keys.contains(&KEY5),
        }
    }
    fn step_on_foot(&mut self, dt: f32) {
        let world = self.world.clone();
        let input = self.walk_input();
        let fuel = self.sky.vitals.jetpack > 0.0;
        let before = self.sky.walker.position;
        let jetting = self.sky.walker.step(&world, input, dt, fuel);
        // a burst when the jetpack lights, then a puff six times a second
        if jetting && (!self.sky.jetting || (self.sky.time * 6.0).fract() < dt * 6.0) {
            self.sounds.push(Sound::Jetpack);
        }
        self.sky.jetting = jetting;
        let moved = (self.sky.walker.position - before).truncate().length();
        self.sky.stride += moved * 0.9;
        let mut eye = self.foot_eye();
        if self.sky.walker.grounded {
            eye.position.z += crate::sky::rng::fast_sin(self.sky.stride) * 0.07;
            self.sky.footstep += moved;
            if self.sky.footstep > 2.6 {
                self.sky.footstep = 0.0;
                self.sounds.push(Sound::Footstep);
            }
        }
        self.flight.eye = Some(eye);
        self.flight.time += dt;
        self.flight.weather.step(dt, world.kind);
        // survival
        let sheltered = self
            .sky
            .parked
            .is_some_and(|p| p.position.distance(self.sky.walker.position) < SHELTER_DISTANCE);
        let hazard = match (self.sky.planet(), sheltered) {
            (Some(planet), false) => planet.biome.hazard(),
            _ => (crate::sky::biome::Hazard::Safe, 0.0),
        };
        if sheltered {
            let v = &mut self.sky.vitals;
            v.hazard = (v.hazard + dt / 20.0).min(1.0);
        }
        let upgrades = self.sky.upgrades;
        match self.sky.vitals.step_suit(dt, hazard, &upgrades, jetting) {
            Some(Event::Warning) => {
                self.sounds.push(Sound::Hazard);
                let text = if self.sky.vitals.life < 0.25 {
                    "LIFE SUPPORT LOW / USE OXYGEN"
                } else {
                    "HAZARD PROTECTION LOW / SODIUM"
                };
                self.sky.notice(text, RED);
            }
            Some(Event::Collapse) => self.collapse(),
            _ => {}
        }
        // weapons and drones
        if self.keys.contains(&KEY5) {
            self.fire_weapon(dt);
        } else {
            self.sky.mining = false;
            self.sky.beam = None;
        }
        self.combat_step(dt);
        self.refresh_props(self.sky.walker.position.truncate());
        self.autosave(dt);
    }
    /// Life support ran out: wake up at the ship, having dropped a third of your elements.
    fn collapse(&mut self) {
        for item in Item::ELEMENTS {
            let lost = self.sky.inventory.count(item) / 3;
            self.sky.inventory.take(item, lost);
        }
        let v = &mut self.sky.vitals;
        v.life = 0.6;
        v.hazard = 0.6;
        v.suit = 1.0;
        self.wake_at_ship();
        self.sky.notice("SUIT FAILURE / ELEMENTS LOST", RED);
        self.sounds.push(Sound::Destroyed);
    }
    fn refresh_props(&mut self, center: Vec2) {
        let Location::Planet(i) = self.sky.location else {
            return;
        };
        let here = Some((self.sky.system.id, i));
        if self.sky.props_planet == here && self.sky.props_center.distance(center) < 16.0 {
            return;
        }
        let planet = &self.sky.system.planets[i];
        props::nearby(
            &self.world,
            planet.seed,
            planet.biome,
            center,
            props::DRAW_DISTANCE + 24.0,
            &self.sky.harvested,
            &mut self.sky.props,
        );
        self.sky.props_center = center;
        self.sky.props_planet = here;
    }

    // ----- actions ------------------------------------------------------------------------

    fn sky_land(&mut self) {
        let Location::Planet(_) = self.sky.location else {
            self.sky.notice("FLY DOWN TO A PLANET TO LAND", WHITE);
            return;
        };
        if self.orbit.transition.is_some() || self.flight.surface.is_some_and(|s| s.bend > 0.02) {
            self.sky.notice("TOO HIGH TO LAND", GOLD);
            return;
        }
        let p = self.flight.position;
        let above = p.z - self.world.height(p.x, p.y);
        let edge = p.x.min(p.y).min(SIZE as f32 - p.x).min(SIZE as f32 - p.y);
        if edge < 300.0 {
            self.sky.notice("TOO CLOSE TO THE EDGE OF THE MAP", GOLD);
            return;
        }
        if above > LAND_ALTITUDE {
            self.sky.notice(
                format!("DESCEND {:.0} TO LAND", above - LAND_ALTITUDE),
                GOLD,
            );
            return;
        }
        // search a fan of spots ahead of the ship, nearest and straightest first
        for distance in [45.0, 70.0, 100.0, 25.0, 140.0, 190.0] {
            for turn in [0.0, 0.35, -0.35, 0.75, -0.75, 1.2, -1.2] {
                let (forward, _) = heading(self.flight.yaw + turn);
                let spot = p.truncate() + forward.truncate() * distance;
                if !self.flat_ground(spot) {
                    continue;
                }
                let ground = crate::sky::walker::ground(&self.world, spot.x, spot.y);
                self.sky.phase = Phase::Landing {
                    t: 0.0,
                    from: p,
                    to: spot.extend(ground + SHIP_REST),
                };
                self.flight.hover = false;
                self.flight.yaw += turn;
                self.sky.notice("LANDING", CYAN);
                self.sounds.push(Sound::Toggle(true));
                return;
            }
        }
        self.sky.notice("NO FLAT DRY GROUND AHEAD", GOLD);
    }
    fn flat_ground(&self, p: Vec2) -> bool {
        // keep away from the edge of the terrain map, where the world ends
        let margin = 260.0;
        if p.x < margin || p.y < margin || p.x > SIZE as f32 - margin || p.y > SIZE as f32 - margin
        {
            return false;
        }
        let world = &self.world;
        if !world.kind.vacuum()
            && world.materials[crate::world::index(p.x as i32, p.y as i32, SIZE)] < 4
        {
            return false;
        }
        let ground = |x: f32, y: f32| crate::sky::walker::ground(world, x, y);
        let h = ground(p.x, p.y);
        [
            vec2(5.0, 0.0),
            vec2(-5.0, 0.0),
            vec2(0.0, 5.0),
            vec2(0.0, -5.0),
        ]
        .iter()
        .all(|o| (ground(p.x + o.x, p.y + o.y) - h).abs() <= 2.5)
    }
    pub(super) fn sky_board_ship(&mut self) {
        let Some(parked) = self.sky.parked else {
            return;
        };
        let distance = parked.position.distance(self.sky.walker.position);
        if distance > BOARD_DISTANCE {
            self.sky
                .notice(format!("SHIP IS {distance:.0} M AWAY"), WHITE);
            return;
        }
        if self.sky.vitals.lift < 0.25 {
            self.sky.notice("LIFT FUEL LOW / USE TRITIUM", RED);
            self.sounds.push(Sound::Hazard);
            return;
        }
        self.sky.vitals.lift -= 0.25;
        self.flight.eye = None;
        self.flight.position = parked.position;
        self.flight.follow = parked.position;
        self.flight.yaw = parked.yaw;
        self.flight.pitch = 0.0;
        self.flight.bank = 0.0;
        self.flight.speed = 0.0;
        self.flight.trails.clear();
        self.sky.ship = None;
        self.sky.parked = None;
        self.sky.beam = None;
        self.sky.clear_combat();
        self.sky.phase = Phase::Takeoff {
            t: 0.0,
            from: parked.position,
        };
        self.sounds.push(Sound::BoostStart);
    }
    fn sky_scan(&mut self) {
        if self.sky.scan_age < 1.2 {
            return;
        }
        self.sky.scan_age = 0.0;
        self.sounds.push(Sound::Scan);
        match self.sky.location {
            Location::Planet(i) => {
                let planet = self.sky.system.planets[i].clone();
                let on_foot = self.sky.phase == Phase::OnFoot;
                let (center, radius) = if on_foot {
                    (self.sky.walker.position.truncate(), 45.0)
                } else {
                    (self.flight.position.truncate(), 420.0)
                };
                props::nearby(
                    &self.world,
                    planet.seed,
                    planet.biome,
                    center,
                    radius,
                    &self.sky.harvested,
                    &mut self.sky.revealed,
                );
                let mut credits = 0;
                let mut found = 0;
                let mut first = None;
                if !on_foot {
                    // from the air the scanner only picks out points of interest
                    self.sky.revealed.retain(|p| p.kind.is_poi());
                }
                let mut data = 0;
                let mut flora = 0;
                let mut minerals = 0;
                let revealed = std::mem::take(&mut self.sky.revealed);
                for prop in &revealed {
                    if self.sky.scanned.insert(prop.id) {
                        credits += prop.kind.discovery();
                        found += 1;
                        flora += u16::from(prop.kind == PropKind::Flora);
                        minerals +=
                            u16::from(matches!(prop.kind, PropKind::Rock | PropKind::Crystal));
                        if prop.kind == PropKind::Flora || prop.kind.is_poi() {
                            data += 1;
                        }
                        if first.is_none() || prop.kind.is_poi() {
                            first = Some(*prop);
                        }
                    }
                }
                self.sky.revealed = revealed;
                self.mission_event(crate::sky::missions::Event::Scanned { flora, minerals });
                if found > 0 {
                    self.sky.credits += credits;
                    self.sky.data += data;
                    self.sky.discoveries += found;
                    if let Some(prop) = first {
                        self.sky
                            .notice(format!("{} {}", prop.kind.label(), prop.name()), CYAN);
                    }
                    self.sky.notice(format!("{found} NEW +{credits} CR"), GOLD);
                    self.sounds.push(Sound::Coin(found.min(9) as u8));
                } else if self.sky.revealed.is_empty() {
                    let text = if on_foot {
                        "NOTHING NEW NEARBY"
                    } else {
                        "NO POINTS OF INTEREST"
                    };
                    self.sky.notice(text, WHITE);
                } else {
                    let n = self.sky.revealed.len();
                    self.sky.notice(format!("{n} KNOWN OBJECTS NEARBY"), WHITE);
                }
            }
            Location::Space => {
                let hash = self.sky.system.seed ^ 0x5ca7;
                if self.sky.scanned.insert(hash) {
                    self.sky.credits += 100;
                    self.sky.data += 2;
                    self.sky.notice("SYSTEM SURVEYED +100 CR", GOLD);
                }
                let station = self.flight.position.distance(self.sky.system.station);
                self.sky.notice(
                    format!("STATION {:.1}K / 6 AUTOPILOT", station / 1000.0),
                    CYAN,
                );
            }
            Location::Elsewhere => {}
        }
    }
    /// Prop under the crosshair (on foot), if any, within mining range.
    pub fn aimed_prop(&self) -> Option<props::Prop> {
        let camera = crate::flight::Camera::new(&self.flight, WIDTH, HEIGHT);
        let eye = self.sky.walker.eye().position;
        let mut best: Option<(f32, props::Prop)> = None;
        for prop in &self.sky.props {
            let distance = prop.center().distance(eye);
            if distance > 32.0 {
                continue;
            }
            let Some((point, depth, scale)) = camera.project(prop.center()) else {
                continue;
            };
            if depth < -3.0 {
                continue;
            }
            if !(0.0..320.0).contains(&point.x) || !(0.0..131.0).contains(&point.y) {
                continue;
            }
            // tolerance grows with the prop's size on screen, but never beyond 36 px
            let reach = (prop.half_extent().max_element() * scale).clamp(10.0, 32.0) + 4.0;
            let off = point.distance(CROSSHAIR);
            if off < reach && best.is_none_or(|(d, _)| off < d) {
                best = Some((off, *prop));
            }
        }
        best.map(|(_, p)| p)
    }
    fn mine_asteroid(&mut self, dt: f32) {
        if !self.sky.vitals.tool_ready() {
            self.sky.mining = false;
            self.sky.beam = None;
            self.sky.vitals.cool(dt);
            return;
        }
        let camera = crate::flight::Camera::new(&self.flight, WIDTH, HEIGHT);
        self.sky.mining = true;
        if (self.sky.time * 8.0).fract() < dt * 8.0 {
            self.sounds.push(Sound::Mine);
        }
        self.sky.vitals.fire_tool(dt * 0.6);
        let mut best: Option<(f32, u32)> = None;
        for i in 0..self.sky.system.asteroid_count {
            if self.sky.destroyed.contains(&i) {
                continue;
            }
            let (center, radius) = self.sky.system.asteroid(i);
            if center.distance(self.flight.position) > 900.0 {
                continue;
            }
            let Some((point, depth, scale)) = camera.project(center) else {
                continue;
            };
            if depth < 0.0 {
                continue;
            }
            let off = point.distance(vec2(160.0, 73.0));
            if off < radius * scale + 14.0 && best.is_none_or(|(d, _)| off < d) {
                best = Some((off, i));
            }
        }
        let Some((_, i)) = best else {
            self.sky.target = None;
            self.sky.beam = Some(self.flight.position + self.flight.axes()[1] * 600.0);
            return;
        };
        let (center, radius) = self.sky.system.asteroid(i);
        if self.sky.target.map(|t| t.id()) != Some(Target::Asteroid(i).id()) {
            self.sky.target = Some(Target::Asteroid(i));
            self.sky.mine_progress = 0.0;
        }
        self.sky.beam = Some(center);
        self.sky.mine_progress += dt * self.sky.upgrades.beam_rate();
        if self.sky.mine_progress >= radius / 12.0 {
            self.sky.destroyed.push(i);
            self.sky.target = None;
            self.sky.mine_progress = 0.0;
            self.sounds.push(Sound::Explosion(false));
            self.rock_yield(i, radius);
        }
    }
    fn set_course(&mut self, index: usize) {
        let planet = self.sky.system.planets[index].clone();
        if planet.is_moon() {
            self.sky.notice("THE MOON IS ALWAYS IN REACH", WHITE);
            return;
        }
        if self.sky.location != Location::Space || self.orbit.transition.is_some() {
            self.sky.notice("LEAVE THE PLANET TO CHANGE COURSE", GOLD);
            return;
        }
        if index == self.sky.course {
            self.sky.overlay = Overlay::None;
            return;
        }
        self.sky.course = index;
        self.flight.planet_maps[Body::Earth.index()] = None;
        self.orbit.earth = Some(planet.key());
        self.network.request(planet.key());
        self.flight.warp_flash = 0.8;
        self.sounds.push(Sound::Warp);
        self.sky.overlay = Overlay::None;
        self.sky
            .notice(format!("COURSE SET: {}", planet.name), CYAN);
        self.sky.notice(
            format!(
                "{} / {}",
                planet.biome.name(),
                planet.biome.hazard().0.name()
            ),
            WHITE,
        );
    }
    fn sky_warp(&mut self) {
        let Some(&id) = self.sky.neighbors.get(self.sky.map_select) else {
            return;
        };
        let reason = if self.world.kind != MapKind::Space {
            Some("LEAVE THE ATMOSPHERE TO WARP")
        } else if self.flight.orbit_quiet || self.orbit.transition.is_some() {
            Some("TOO CLOSE TO A PLANET")
        } else if !self.sky.in_range(id) {
            Some("OUT OF RANGE / UPGRADE WARP")
        } else if self.sky.inventory.count(Item::JumpCell) == 0 {
            Some("NEED A JUMP CELL / CRAFT ONE")
        } else {
            None
        };
        if let Some(reason) = reason {
            self.sky.notice(reason, RED);
            self.sounds.push(Sound::Hazard);
            return;
        }
        self.sky.inventory.take(Item::JumpCell, 1);
        let target = self.sky.galaxy.system(id);
        let world = self.world.clone();
        self.network.open_warp(
            &world,
            self.flight.position,
            self.flight.axes()[1],
            target.space_key(),
        );
        self.sky.warp_target = Some(id);
        self.sky.overlay = Overlay::None;
        self.sky.autodock = false;
        self.sky
            .notice(format!("WARP TUNNEL TO {}", target.name), CYAN);
        self.sky.notice("FLY INTO THE TUNNEL AHEAD", WHITE);
        self.sounds.push(Sound::Warp);
    }
    fn sky_use(&mut self) {
        let Some((item, count)) = self.sky.inventory.slots()[self.sky.inventory_select] else {
            return;
        };
        match item {
            Item::JumpCell => {
                self.sky.notice("JUMP CELLS POWER WARPS (7 MAP)", WHITE);
                return;
            }
            Item::DroneCore => {
                self.sky.notice("CRAFT A REROLL CELL OR SELL IT", WHITE);
                return;
            }
            Item::RerollCell => {
                if self.reroll_weapon() {
                    self.sky.inventory.take(Item::RerollCell, 1);
                }
                return;
            }
            _ => {}
        }
        let total = self.sky.inventory.count(item).max(count);
        let used = self.sky.vitals.recharge(item, total);
        if used == 0 {
            self.sky.notice("ALREADY FULL", WHITE);
            return;
        }
        self.sky.inventory.take(item, used);
        self.sky.notice(
            format!("{} -{used} {}", item.use_hint(), item.symbol()),
            item.color(),
        );
        self.sounds.push(Sound::Pickup);
    }
    fn sky_craft(&mut self) {
        let recipe = &RECIPES[self.sky.craft_select];
        if recipe.craft(&mut self.sky.inventory) {
            self.sky
                .notice(format!("CRAFTED {}", recipe.output.name()), GOLD);
            self.sounds.push(Sound::Craft);
        } else {
            self.sky.notice("MISSING ELEMENTS OR SPACE", RED);
            self.sounds.push(Sound::Hazard);
        }
    }
    /// Rows of the current station tab.
    pub fn station_rows(&self) -> usize {
        match self.sky.station_tab {
            0 => 1 + Item::ALL.len(),
            1 => 3 + STATION_STOCK.len() + 1,
            _ => Upgrade::ALL.len(),
        }
    }
    fn station_confirm(&mut self) {
        let market = self.sky.system.market;
        let row = self.sky.station_select;
        match self.sky.station_tab {
            0 => {
                if row == 0 {
                    if self.sky.data == 0 {
                        self.sky.notice("NO SURVEY DATA", WHITE);
                        return;
                    }
                    let credits = self.sky.data * station::DATA_PRICE;
                    self.sky.credits += credits;
                    self.sky.data = 0;
                    self.sky.notice(format!("DATA SOLD +{credits} CR"), GOLD);
                    self.sounds.push(Sound::Coin(5));
                    return;
                }
                let item = Item::ALL[row - 1];
                let n = self
                    .sky
                    .inventory
                    .count(item)
                    .min(if item.is_element() { 25 } else { 1 });
                if n == 0 {
                    self.sky.notice(format!("NO {}", item.name()), WHITE);
                    return;
                }
                let (_, sell) = station::prices(item, market);
                self.sky.inventory.take(item, n);
                self.sky.credits += sell * n as u32;
                self.sky.notice(
                    format!("SOLD {n} {} +{}", item.symbol(), sell * n as u32),
                    GOLD,
                );
                self.sounds.push(Sound::Coin(3));
            }
            1 if row <= 2 => self.buy_armory(row),
            1 => {
                let row = row - 3;
                if row == STATION_STOCK.len() {
                    // shield repair service
                    let cost = ((1.0 - self.sky.vitals.shield) * 400.0).ceil() as u32;
                    if cost == 0 {
                        self.sky.notice("SHIELD ALREADY FULL", WHITE);
                    } else if self.sky.credits < cost {
                        self.sky.notice("NOT ENOUGH CREDITS", RED);
                    } else {
                        self.sky.credits -= cost;
                        self.sky.vitals.shield = 1.0;
                        self.sky.notice(format!("SHIELD REPAIRED -{cost}"), CYAN);
                        self.sounds.push(Sound::Craft);
                    }
                    return;
                }
                let (item, n) = STATION_STOCK[row];
                let (buy, _) = station::prices(item, market);
                let cost = buy * n as u32;
                if self.sky.credits < cost {
                    self.sky.notice("NOT ENOUGH CREDITS", RED);
                    self.sounds.push(Sound::Hazard);
                    return;
                }
                let added = self.sky.inventory.add(item, n);
                if added == 0 {
                    self.sky.notice("INVENTORY FULL", RED);
                    return;
                }
                let cost = buy * added as u32;
                self.sky.credits -= cost;
                self.sky
                    .notice(format!("BOUGHT {added} {} -{cost}", item.symbol()), CYAN);
                self.sounds.push(Sound::Craft);
            }
            _ => {
                let upgrade = Upgrade::ALL[row];
                let level = self.sky.upgrades.level(upgrade);
                if level >= Upgrade::MAX {
                    self.sky.notice("FULLY UPGRADED", WHITE);
                    return;
                }
                let cost = upgrade.cost(level);
                if self.sky.credits < cost {
                    self.sky.notice("NOT ENOUGH CREDITS", RED);
                    self.sounds.push(Sound::Hazard);
                    return;
                }
                self.sky.credits -= cost;
                self.sky.upgrades.levels[upgrade as usize] += 1;
                self.sky
                    .notice(format!("{} MK{}", upgrade.name(), level + 2), GOLD);
                self.sounds.push(Sound::Upgrade);
            }
        }
        self.sky_save();
    }

    // ----- scene --------------------------------------------------------------------------

    /// Fill the renderer's extras (props, parked ship, station, asteroids, colour grade).
    pub fn sky_scene(&mut self) {
        let active = self.explore() && !self.scene_is_menu();
        let extras = &mut self.renderer.extras;
        extras.clear();
        if !active {
            return;
        }
        let sky = &self.sky;
        let star = sky.system.class.grade();
        match sky.location {
            Location::Planet(i) => {
                let planet = &sky.system.planets[i];
                let bend = self.flight.surface.map_or(0.0, |s| s.bend);
                extras.grade = crate::sky::biome::compose(&star, &planet.biome.grade(planet.seed));
                if bend < 0.02 && self.orbit.transition.is_none() {
                    let eye = self.flight.eye.map_or(self.flight.position, |e| e.position);
                    let palette = planet.biome.palette(planet.seed);
                    for prop in &sky.props {
                        let d = prop.position.distance(eye);
                        if d < props::DRAW_DISTANCE {
                            props::mesh(
                                prop,
                                palette,
                                sky.time,
                                d < props::DETAIL_DISTANCE,
                                &mut extras.triangles,
                            );
                        }
                    }
                    if let Some(ship) = &sky.ship
                        && sky.parked.is_some_and(|p| p.planet == i)
                    {
                        extras.ships.push(ship.clone());
                    }
                    self.combat_scene();
                }
            }
            Location::Space => {
                extras.grade = star;
                let p = self.flight.position;
                if sky.system.station.distance(p) < 9000.0 {
                    props::station(sky.system.station, sky.time, &mut extras.triangles);
                }
                for i in 0..sky.system.asteroid_count {
                    if sky.destroyed.contains(&i) {
                        continue;
                    }
                    let (center, radius) = sky.system.asteroid(i);
                    if center.distance(p) < 3200.0 {
                        props::asteroid(
                            center,
                            radius,
                            sky.system.seed ^ i as u64,
                            &mut extras.triangles,
                        );
                    }
                }
            }
            Location::Elsewhere => {}
        }
        if self.sky.location == Location::Space {
            if self.sky.phase == Phase::Station {
                self.renderer.extras.triangles.clear();
                self.renderer.extras.ships.clear();
                self.station_scene();
            } else {
                self.space_scene();
            }
        }
    }
    /// Whether a surface/space transition is in progress (for tools and tests).
    pub fn orbit_transition(&self) -> bool {
        self.orbit.transition.is_some()
    }
    /// Notices for the HUD (newest last).
    pub fn sky_notices(&self) -> &[Notice] {
        &self.sky.notices
    }
    /// Systems in the galaxy map with whether each is in range and visited.
    pub fn sky_neighbors(&self) -> Vec<(SystemId, bool, bool)> {
        self.sky
            .neighbors
            .iter()
            .map(|&id| (id, self.sky.in_range(id), self.sky.visited.contains(&id)))
            .collect()
    }
}

/// Autopilot heading from `from` to `to` that goes around a planet's capture sphere (where the
/// engine would start a descent) whenever the straight line crosses it.
pub fn autopilot_heading(from: Vec3, to: Vec3) -> Vec3 {
    let delta = to - from;
    let length = delta.length().max(0.001);
    let direction = delta / length;
    for body in Body::ALL {
        let center = body.center();
        let capture = body.radius() + body.orbit_altitude() + 250.0;
        let along = (center - from).dot(direction).clamp(0.0, length);
        if (from + direction * along).distance(center) >= capture {
            continue;
        }
        // the straight line would cross the capture sphere: skim around it at a safe height,
        // moving along the tangent toward the target and holding ~600 units above the sphere
        let radial = (from - center).try_normalize().unwrap_or(Vec3::Z);
        let tangent = (direction - radial * direction.dot(radial))
            .try_normalize()
            .unwrap_or_else(|| radial.any_orthonormal_vector());
        let height = from.distance(center) - capture;
        let climb = ((600.0 - height) / 400.0).clamp(-0.4, 1.5);
        return (tangent + radial * climb).normalize_or_zero();
    }
    direction
}

/// What stations sell: (item, units per purchase). The last row is shield repair.
pub const STATION_STOCK: [(Item, u16); 8] = [
    (Item::Tritium, 20),
    (Item::Oxygen, 20),
    (Item::Sodium, 20),
    (Item::Carbon, 20),
    (Item::Iron, 20),
    (Item::JumpCell, 1),
    (Item::LiftCell, 1),
    (Item::HullPatch, 1),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::World;

    fn tap(app: &mut App, code: u16) {
        app.key(code, 1);
        app.key(code, 0);
    }
    /// An Explore session that starts on the first planet of the starting system.
    fn session() -> App {
        let key = Sky::default().system.planets[0].key();
        let mut app = App::new(World::generate(key.kind, key.seed));
        app.mode = Mode::Explore;
        app.start();
        assert_eq!(app.page, Page::Playing);
        assert!(app.explore());
        assert_eq!(app.sky.location, Location::Planet(0));
        app
    }

    #[test]
    fn explore_session_lands_walks_scans_mines_crafts_and_takes_off() {
        let mut app = session();
        // fly low and land with key 6, retrying as a player would over water or slopes
        let mut landed = false;
        for _ in 0..40 {
            let p = app.flight.position;
            let ground = app.world.height(p.x, p.y);
            app.flight.translate(vec3(0.0, 0.0, ground + 110.0 - p.z));
            tap(&mut app, KEY6);
            app.advance(110);
            if app.sky.phase == Phase::OnFoot {
                landed = true;
                break;
            }
        }
        assert!(landed, "could not land");
        let parked = app.sky.parked.expect("ship parked");
        assert!(app.flight.eye.is_some());
        app.draw();
        assert!(
            !app.renderer.extras.ships.is_empty(),
            "parked ship is drawn"
        );
        // walking moves the walker and keeps it on the ground
        let start = app.sky.walker.position;
        app.key(UP, 1);
        app.advance(45);
        app.key(UP, 0);
        assert!(app.sky.walker.position.distance(start) > 3.0);
        // scanning on foot finds props and pays for them
        let credits = app.sky.credits;
        tap(&mut app, KEY4);
        app.advance(2);
        assert!(app.sky.discoveries > 0);
        assert!(app.sky.credits > credits);
        // mining: put the nearest prop under the crosshair and hold key 5
        let eye = app.sky.walker.eye().position;
        let prop = *app
            .sky
            .props
            .iter()
            .filter(|p| p.harvest(crate::sky::biome::Biome::Lush).is_some())
            .min_by(|a, b| {
                a.center()
                    .distance(eye)
                    .total_cmp(&b.center().distance(eye))
            })
            .expect("props nearby");
        let back = (eye - prop.center()).truncate().normalize() * 7.0;
        let world = app.world.clone();
        app.sky.walker = Walker::new(
            &world,
            prop.position.x + back.x,
            prop.position.y + back.y,
            0.0,
        );
        let to = prop.center() - app.sky.walker.eye().position;
        app.sky.walker.yaw = to.x.atan2(-to.y);
        app.sky.walker.pitch = (to.z / to.truncate().length()).atan();
        app.advance(1);
        assert_eq!(app.aimed_prop().map(|p| p.id), Some(prop.id));
        let before = app.sky.inventory;
        app.key(KEY5, 1);
        app.advance(150);
        app.key(KEY5, 0);
        assert!(app.sky.harvested.contains(&prop.id));
        assert_ne!(app.sky.inventory, before, "mining yields elements");
        // inventory and crafting through keys 8, 5, 6
        app.sky.inventory.add(Item::Tritium, 40);
        app.sky.inventory.add(Item::Iron, 20);
        tap(&mut app, KEY8);
        assert_eq!(app.sky.overlay, Overlay::Inventory);
        tap(&mut app, KEY5);
        tap(&mut app, KEY6);
        tap(&mut app, KEY8);
        assert_eq!(app.sky.overlay, Overlay::None);
        assert_eq!(app.sky.inventory.count(Item::JumpCell), 1);
        // board from beside the ship (key 7) and lift off, spending lift fuel
        app.sky.walker = Walker::new(&world, parked.position.x + 4.0, parked.position.y, 0.0);
        let lift = app.sky.vitals.lift;
        tap(&mut app, KEY7);
        assert!(matches!(app.sky.phase, Phase::Takeoff { .. }));
        app.advance(90);
        assert_eq!(app.sky.phase, Phase::Flying);
        assert!(app.flight.eye.is_none());
        assert!(app.sky.vitals.lift < lift);
        assert!(app.flight.position.z > parked.position.z + 30.0);
    }

    #[test]
    fn the_gyroscope_only_steers_while_piloting() {
        let mut app = session();
        assert!(app.sky_motion());
        tap(&mut app, KEY8);
        assert!(!app.sky_motion(), "not with a panel open");
        tap(&mut app, KEY8);
        app.sky.phase = Phase::OnFoot;
        assert!(!app.sky_motion(), "not on foot");
        app.mode = Mode::Free;
        assert!(app.sky_motion(), "other modes are unaffected");
    }

    #[test]
    fn autopilot_goes_around_planets_to_the_station() {
        let system = Sky::default().system;
        let earth = Body::Earth;
        // start on the far side of the planet from the station
        let away = (system.station - earth.center()).normalize();
        let mut p = earth.center() - away * (earth.radius() + earth.orbit_altitude() + 400.0);
        let capture = earth.radius() + earth.orbit_altitude();
        for _ in 0..4000 {
            p += autopilot_heading(p, system.station) * 10.0;
            assert!(
                p.distance(earth.center()) > capture,
                "entered the capture zone"
            );
            if p.distance(system.station) < DOCK_DISTANCE {
                return;
            }
        }
        panic!("never reached the station");
    }

    #[test]
    fn warping_needs_space_and_a_jump_cell() {
        let mut app = session();
        app.open_overlay(Overlay::Galaxy);
        assert!(!app.sky.neighbors.is_empty());
        tap(&mut app, KEY6);
        assert!(app.sky.warp_target.is_none(), "no warp from a planet");
        let space = app.sky.system.space_key();
        let world = std::sync::Arc::new(World::generate(space.kind, space.seed));
        app.flight.change_world(&world);
        app.world = world;
        app.flight
            .translate(app.sky.system.station + vec3(600.0, 0.0, 0.0) - app.flight.position);
        app.sky_sync_location();
        assert_eq!(app.sky.location, Location::Space);
        app.open_overlay(Overlay::Galaxy);
        // pick a star in range
        let select = app
            .sky
            .neighbors
            .iter()
            .position(|&id| app.sky.in_range(id))
            .expect("a star in range");
        app.sky.map_select = select;
        app.sky.inventory.take(Item::JumpCell, 9);
        tap(&mut app, KEY6);
        assert!(app.sky.warp_target.is_none(), "no warp without a jump cell");
        app.sky.inventory.add(Item::JumpCell, 1);
        tap(&mut app, KEY6);
        assert_eq!(app.sky.warp_target, Some(app.sky.neighbors[select]));
        assert!(app.network.warp_open());
        assert_eq!(app.sky.inventory.count(Item::JumpCell), 0);
        assert_eq!(
            app.network.routes(&app.world).len(),
            1,
            "only the warp tunnel exists"
        );
    }
}
