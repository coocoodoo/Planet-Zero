use super::{Frame, Mixer};
use std::{
    io::Write,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub struct Handle {
    child: Child,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl Drop for Handle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.child.kill();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = self.child.wait();
    }
}
pub fn open(receiver: Receiver<Frame>) -> Result<Handle, Box<dyn std::error::Error>> {
    let mut mixer = Mixer::new(48000);
    let mut child = Command::new("pacat")
        .args([
            "--playback",
            "--raw",
            "--format=s16le",
            "--rate=48000",
            "--channels=2",
            "--latency-msec=60",
            "--process-time-msec=10",
            "--client-name=Flight Club",
            "--stream-name=Flight Club Chiptune",
            "--property=media.role=game",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut input = child.stdin.take().ok_or("音频管道未建立")?;
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let worker = thread::Builder::new()
        .name("chip-audio".into())
        .spawn(move || {
            let mut samples = [0.0_f32; 960];
            let mut bytes = [0_u8; 1920];
            let mut deadline = Instant::now();
            // PCM 以 10 ms 分块，预留 30 ms 输入，后续按采样时钟发送
            for _ in 0..3 {
                mixer.render(&mut samples);
                for (value, bytes) in samples.iter().zip(bytes.chunks_exact_mut(2)) {
                    bytes.copy_from_slice(&((*value * 32767.0) as i16).to_le_bytes());
                }
                if input.write_all(&bytes).is_err() {
                    return;
                }
            }
            let mut last_cue = mixer.cue();
            let mut rendered = 1440_u64;
            eprintln!("音乐：{}", last_cue.name());
            while !stopped.load(Ordering::Relaxed) {
                while let Ok(frame) = receiver.try_recv() {
                    mixer.update(frame);
                }
                mixer.render(&mut samples);
                rendered += 480;
                if mixer.cue() != last_cue {
                    last_cue = mixer.cue();
                    eprintln!(
                        "音乐：{} / {:.3} s",
                        last_cue.name(),
                        rendered as f64 / 48000.0
                    );
                }
                for (value, bytes) in samples.iter().zip(bytes.chunks_exact_mut(2)) {
                    bytes.copy_from_slice(&((*value * 32767.0) as i16).to_le_bytes());
                }
                if let Err(error) = input.write_all(&bytes) {
                    if !stopped.load(Ordering::Relaxed) {
                        eprintln!("音频输出已结束：{error}");
                    }
                    break;
                }
                deadline += Duration::from_millis(10);
                if let Some(wait) = deadline.checked_duration_since(Instant::now()) {
                    thread::sleep(wait);
                } else if deadline.elapsed() > Duration::from_millis(100) {
                    deadline = Instant::now();
                }
            }
        })?;
    eprintln!("音频：48 kHz 立体声 PCM，目标缓冲 60 ms");
    Ok(Handle {
        child,
        stop,
        thread: Some(worker),
    })
}
