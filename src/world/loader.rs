use super::*;
#[derive(Default)]
pub struct WorldLoader {
    job: Option<JoinHandle<World>>,
    queued: Option<(MapKind, u32)>,
    pub pending: Option<MapKind>,
}

impl WorldLoader {
    pub fn start(&mut self, kind: MapKind, seed: u32) {
        self.pending = Some(kind);
        if self.job.is_none() {
            self.job = Some(std::thread::spawn(move || World::generate(kind, seed)));
        } else {
            self.queued = Some((kind, seed));
        }
    }
    pub fn poll(&mut self) -> Option<World> {
        if !self.job.as_ref().is_some_and(JoinHandle::is_finished) {
            return None;
        }
        let world = self.job.take().unwrap().join().expect("地图生成线程失败");
        if let Some((kind, seed)) = self.queued.take()
            && (world.kind != kind || world.seed != seed)
        {
            drop(world);
            self.start(kind, seed);
            return None;
        }
        self.pending = None;
        Some(world)
    }
}
