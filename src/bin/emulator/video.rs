//! Video capture for the emulator: framed device frames scaled 2× (sharp pixels) and encoded by
//! ffmpeg, plus the game's own music and effects rendered offline by its synthesizer, in sync
//! (1600 stereo samples per 1/30 s frame). A caption strip under the device names what is
//! happening and which keys are held.
use flight_club::audio::{Frame, Mixer};
use font8x8::UnicodeFonts;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};

/// Frame size before the 2× scale: the 631×408 bezel padded to even width, plus a caption strip.
pub const WIDTH: usize = 632;
pub const HEIGHT: usize = 448;
const STRIP: usize = 408;
const RATE: u32 = 48_000;
const SAMPLES_PER_FRAME: usize = (RATE / 30) as usize;

pub struct Video {
    child: Child,
    stdin: Option<ChildStdin>,
    mixer: Mixer,
    audio: Vec<i16>,
    pub recording: bool,
    output: PathBuf,
    silent: PathBuf,
    buffer: Vec<u8>,
    pub frames: u64,
}

impl Video {
    pub fn start(output: &Path) -> io::Result<Self> {
        let silent = output.with_extension("video.mp4");
        let mut child = Command::new("ffmpeg")
            .args([
                "-y",
                "-loglevel",
                "error",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "-s",
            ])
            .arg(format!("{WIDTH}x{HEIGHT}"))
            .args(["-r", "30", "-i", "-", "-vf"])
            .arg(format!("scale={}:{}:flags=neighbor", WIDTH * 2, HEIGHT * 2))
            .args([
                "-c:v", "libx264", "-preset", "medium", "-crf", "22", "-pix_fmt", "yuv420p",
            ])
            .arg(&silent)
            .stdin(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take();
        Ok(Self {
            child,
            stdin,
            mixer: Mixer::new(RATE),
            audio: Vec::new(),
            recording: true,
            output: output.to_owned(),
            silent,
            buffer: vec![0; WIDTH * HEIGHT * 3],
            frames: 0,
        })
    }
    /// Advance the synthesizer by one frame (always, so music stays continuous) and keep the
    /// samples while recording.
    pub fn audio(&mut self, frame: Frame) {
        self.mixer.update(frame);
        let mut samples = [0.0_f32; SAMPLES_PER_FRAME * 2];
        self.mixer.render(&mut samples);
        if self.recording {
            self.audio.extend(
                samples
                    .iter()
                    .map(|v| (v.clamp(-1.0, 1.0) * 32767.0) as i16),
            );
        }
    }
    /// Add one framed device image (631×408 RGBA) with a caption line and a status line.
    pub fn frame(&mut self, framed: &[u8], caption: &str, status: &str) -> io::Result<()> {
        if !self.recording {
            return Ok(());
        }
        let b = &mut self.buffer;
        b.fill(18);
        for y in 0..408 {
            for x in 0..631 {
                let s = (y * 631 + x) * 4;
                let d = (y * WIDTH + x) * 3;
                b[d..d + 3].copy_from_slice(&framed[s..s + 3]);
            }
        }
        for y in STRIP..HEIGHT {
            for x in 0..WIDTH {
                let d = (y * WIDTH + x) * 3;
                b[d..d + 3].copy_from_slice(&[14, 22, 34]);
            }
        }
        text(b, 12, STRIP + 5, caption, [111, 237, 218], 2);
        let width = status.chars().count() * 8;
        text(
            b,
            WIDTH - 12 - width,
            STRIP + 28,
            status,
            [255, 208, 105],
            1,
        );
        self.frames += 1;
        match &mut self.stdin {
            Some(stdin) => stdin.write_all(b),
            None => Ok(()),
        }
    }
    /// Close the video stream and add the sound track.
    pub fn finish(mut self) -> io::Result<PathBuf> {
        drop(self.stdin.take());
        self.child.wait()?;
        let wav = self.output.with_extension("wav");
        write_wav(&wav, &self.audio)?;
        let status = Command::new("ffmpeg")
            .args(["-y", "-loglevel", "error", "-i"])
            .arg(&self.silent)
            .arg("-i")
            .arg(&wav)
            .args([
                "-c:v",
                "copy",
                "-c:a",
                "aac",
                "-b:a",
                "160k",
                "-shortest",
                "-movflags",
                "+faststart",
            ])
            .arg(&self.output)
            .status()?;
        if !status.success() {
            return Err(io::Error::other("ffmpeg could not add the sound track"));
        }
        let _ = std::fs::remove_file(&self.silent);
        let _ = std::fs::remove_file(&wav);
        Ok(self.output)
    }
}

fn write_wav(path: &Path, samples: &[i16]) -> io::Result<()> {
    let mut out = io::BufWriter::new(std::fs::File::create(path)?);
    let bytes = (samples.len() * 2) as u32;
    out.write_all(b"RIFF")?;
    out.write_all(&(bytes + 36).to_le_bytes())?;
    out.write_all(b"WAVEfmt ")?;
    out.write_all(&16_u32.to_le_bytes())?;
    out.write_all(&1_u16.to_le_bytes())?;
    out.write_all(&2_u16.to_le_bytes())?;
    out.write_all(&RATE.to_le_bytes())?;
    out.write_all(&(RATE * 4).to_le_bytes())?;
    out.write_all(&4_u16.to_le_bytes())?;
    out.write_all(&16_u16.to_le_bytes())?;
    out.write_all(b"data")?;
    out.write_all(&bytes.to_le_bytes())?;
    for s in samples {
        out.write_all(&s.to_le_bytes())?;
    }
    out.flush()
}

fn text(buffer: &mut [u8], x: usize, y: usize, value: &str, color: [u8; 3], scale: usize) {
    for (i, ch) in value.chars().enumerate() {
        let Some(glyph) = font8x8::BASIC_FONTS.get(ch) else {
            continue;
        };
        for (gy, row) in glyph.iter().enumerate() {
            for gx in 0..8 {
                if row & (1 << gx) == 0 {
                    continue;
                }
                for sy in 0..scale {
                    for sx in 0..scale {
                        let px = x + (i * 8 + gx) * scale + sx;
                        let py = y + gy * scale + sy;
                        if px < WIDTH && py < HEIGHT {
                            let d = (py * WIDTH + px) * 3;
                            buffer[d..d + 3].copy_from_slice(&color);
                        }
                    }
                }
            }
        }
    }
}
