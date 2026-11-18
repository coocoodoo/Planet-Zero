use rust_fly::app::{App, HEIGHT, WIDTH};
use rust_fly::flight::Flight;
use rust_fly::gameplay::Mission;
use rust_fly::render::Renderer;
use rust_fly::world::{World, network::Network};
use std::sync::{Arc, mpsc};

pub const SCALES: [usize; 4] = [1, 2, 4, 8];

struct Scene {
    world: Arc<World>,
    flight: Flight,
    mission: Option<Mission>,
    network: Option<Network>,
    theme: usize,
    scale: usize,
}
pub struct Frame {
    pub map: rust_fly::world::network::MapKey,
    pub pixels: Vec<u8>,
    pub scale: usize,
    pub milliseconds: f64,
}
pub struct Drawing {
    send: mpsc::SyncSender<Scene>,
    receive: mpsc::Receiver<Frame>,
    recycle: mpsc::Sender<Vec<u8>>,
    busy: bool,
}
impl Drawing {
    pub fn new() -> Self {
        let (send, scenes) = mpsc::sync_channel::<Scene>(1);
        let (frames, receive) = mpsc::sync_channel(1);
        let (recycle, recycled) = mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
            let mut renderer = Renderer::new();
            let mut scale = 1;
            while let Ok(scene) = scenes.recv() {
                // 回收主线程用过的帧缓冲，避免每帧重新分配
                if let Ok(buffer) = recycled.try_recv() {
                    renderer.rgba = buffer;
                }
                if scale != scene.scale {
                    renderer = Renderer::new();
                    scale = scene.scale;
                }
                let started = std::time::Instant::now();
                renderer.render_connected(
                    &scene.world,
                    &scene.flight,
                    (WIDTH * scale, HEIGHT * scale),
                    scene.theme,
                    scene.mission.as_ref(),
                    scene.network.as_ref(),
                );
                let frame = Frame {
                    map: rust_fly::world::network::MapKey::of(&scene.world),
                    pixels: std::mem::take(&mut renderer.rgba),
                    scale,
                    milliseconds: started.elapsed().as_secs_f64() * 1000.0,
                };
                if frames.send(frame).is_err() {
                    break;
                }
            }
        });
        Self {
            send,
            receive,
            recycle,
            busy: false,
        }
    }
    pub fn poll(&mut self) -> Option<Frame> {
        let frame = self.receive.try_recv().ok()?;
        self.busy = false;
        Some(frame)
    }
    /// 交还已上传的帧缓冲，渲染线程复用它作为下一帧的输出
    pub fn recycle(&mut self, pixels: Vec<u8>) {
        let _ = self.recycle.send(pixels);
    }
    pub fn submit(&mut self, app: &App, scale: usize) {
        if self.busy {
            return;
        }
        let menu = app.scene_is_menu();
        let scene = Scene {
            world: app.world.clone(),
            flight: app.flight.clone(),
            mission: (!menu).then(|| app.mission.clone()),
            network: Some(app.network.snapshot()),
            theme: app.theme,
            scale,
        };
        self.busy = self.send.send(scene).is_ok();
    }
}
