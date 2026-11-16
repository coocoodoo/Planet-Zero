use super::{Frame, Mixer};
use std::sync::mpsc::{self, SyncSender};

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod platform;
#[cfg(all(not(target_os = "linux"), feature = "desktop"))]
#[path = "desktop.rs"]
mod platform;

pub struct Output {
    sender: SyncSender<Frame>,
    #[cfg(any(target_os = "linux", feature = "desktop"))]
    _handle: platform::Handle,
}
impl Output {
    pub fn start() -> Result<Self, Box<dyn std::error::Error>> {
        let (sender, receiver) = mpsc::sync_channel::<Frame>(4);
        #[cfg(any(target_os = "linux", feature = "desktop"))]
        {
            let handle = platform::open(receiver)?;
            Ok(Self {
                sender,
                _handle: handle,
            })
        }
        #[cfg(all(not(target_os = "linux"), not(feature = "desktop")))]
        {
            let _ = (sender, receiver, Mixer::new);
            Err("当前构建支持离线音频导出，实时输出需要 desktop 或 Linux".into())
        }
    }
    pub fn update(&self, app: &mut crate::app::App) {
        let _ = self.sender.try_send(app.audio_frame());
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        let frame = Frame {
            settings: super::Settings {
                volume: 0,
                profile: 3,
            },
            ..Frame::default()
        };
        if self.sender.try_send(frame).is_ok() {
            std::thread::sleep(std::time::Duration::from_millis(180));
        }
    }
}
