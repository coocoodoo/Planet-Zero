use memmap2::{MmapMut, MmapOptions};
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{Game, HEIGHT, WIDTH, rgb565};

static STOP: AtomicBool = AtomicBool::new(false);

#[repr(C)]
#[derive(Default)]
struct Bitfield {
    offset: u32,
    length: u32,
    msb_right: u32,
}

#[repr(C)]
#[derive(Default)]
struct VariableInfo {
    xres: u32,
    yres: u32,
    xres_virtual: u32,
    yres_virtual: u32,
    xoffset: u32,
    yoffset: u32,
    bits_per_pixel: u32,
    grayscale: u32,
    red: Bitfield,
    green: Bitfield,
    blue: Bitfield,
    transparency: Bitfield,
    nonstd: u32,
    activate: u32,
    height: u32,
    width: u32,
    accel_flags: u32,
    pixclock: u32,
    left_margin: u32,
    right_margin: u32,
    upper_margin: u32,
    lower_margin: u32,
    hsync_len: u32,
    vsync_len: u32,
    sync: u32,
    vmode: u32,
    rotate: u32,
    colorspace: u32,
    reserved: [u32; 4],
}

#[repr(C)]
#[derive(Default)]
struct FixedInfo {
    id: [u8; 16],
    memory_start: libc::c_ulong,
    memory_length: u32,
    kind: u32,
    kind_aux: u32,
    visual: u32,
    xpanstep: u16,
    ypanstep: u16,
    ywrapstep: u16,
    line_length: u32,
    mmio_start: libc::c_ulong,
    mmio_length: u32,
    accel: u32,
    capabilities: u16,
    reserved: [u16; 3],
}

fn framebuffer_path() -> io::Result<PathBuf> {
    for name in [
        "RUST_FLY_FB",
        "APPLAUNCH_LINUX_FBDEV_DEVICE",
        "LV_LINUX_FBDEV_DEVICE",
    ] {
        if let Some(path) = std::env::var_os(name) {
            return Ok(path.into());
        }
    }
    let lcd = PathBuf::from("/dev/fb_lcd");
    if lcd.exists() {
        return Ok(lcd);
    }
    for line in std::fs::read_to_string("/proc/fb")?.lines() {
        if (line.contains("fb_st7789v") || line.contains("panel-mipi-dbid"))
            && let Some(number) = line.split_whitespace().next()
        {
            return Ok(PathBuf::from(format!("/dev/fb{number}")));
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "请通过 RUST_FLY_FB 指定 Cardputer Zero 的屏幕节点",
    ))
}

pub struct Display {
    file: File,
    mapping: MmapMut,
    packed: Vec<u8>,
    stride: usize,
    offset: usize,
}

impl Display {
    pub fn open() -> io::Result<Self> {
        let path = framebuffer_path()?;
        let file = OpenOptions::new().read(true).write(true).open(&path)?;
        let mut variable = VariableInfo::default();
        let mut fixed = FixedInfo::default();
        // 结构体布局对应 Linux fb.h，ioctl 在调用期间写入完整结构体
        if unsafe { libc::ioctl(file.as_raw_fd(), 0x4600, &mut variable) } < 0
            || unsafe { libc::ioctl(file.as_raw_fd(), 0x4602, &mut fixed) } < 0
        {
            return Err(io::Error::last_os_error());
        }
        let fields = [&variable.red, &variable.green, &variable.blue];
        let format =
            fields
                .iter()
                .zip([(11, 5), (5, 6), (0, 5)])
                .all(|(field, (offset, length))| {
                    field.offset == offset && field.length == length && field.msb_right == 0
                });
        if variable.xres != WIDTH as u32
            || variable.yres != HEIGHT as u32
            || variable.bits_per_pixel != 16
            || !format
        {
            return Err(io::Error::other(format!(
                "屏幕 {} 需要 320×170 RGB565，当前为 {}×{} {}bpp",
                path.display(),
                variable.xres,
                variable.yres,
                variable.bits_per_pixel
            )));
        }
        let stride = fixed.line_length as usize;
        let offset = variable.yoffset as usize * stride + variable.xoffset as usize * 2;
        let end = offset + (HEIGHT - 1) * stride + WIDTH * 2;
        if stride < (variable.xoffset as usize + WIDTH) * 2 || end > fixed.memory_length as usize {
            return Err(io::Error::other("屏幕行跨度或映射长度不足"));
        }
        // 映射由 Display 独占写入，文件句柄与映射共同存活
        let mapping = unsafe {
            MmapOptions::new()
                .len(fixed.memory_length as usize)
                .map_mut(&file)?
        };
        eprintln!("屏幕 {}：320×170 RGB565，stride {stride}", path.display());
        Ok(Self {
            file,
            mapping,
            packed: vec![0; WIDTH * HEIGHT * 2],
            stride,
            offset,
        })
    }

    pub fn present(&mut self, rgba: &[u8]) -> io::Result<()> {
        rgb565(rgba, &mut self.packed);
        for y in 0..HEIGHT {
            let start = self.offset + y * self.stride;
            self.mapping[start..start + WIDTH * 2]
                .copy_from_slice(&self.packed[y * WIDTH * 2..(y + 1) * WIDTH * 2]);
        }
        self.file.sync_data()
    }
}

pub struct Keyboard(File);

impl Keyboard {
    pub fn open() -> io::Result<Self> {
        let path = std::env::var("RUST_FLY_KEYBOARD")
            .or_else(|_| std::env::var("APPLAUNCH_LINUX_KEYBOARD_DEVICE"))
            .unwrap_or_else(|_| "/dev/input/by-path/platform-3f804000.i2c-event".into());
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&path)?;
        eprintln!("键盘 {path}");
        Ok(Self(file))
    }

    pub fn poll(&mut self, game: &mut Game) -> io::Result<()> {
        loop {
            // libc::input_event 使用目标平台的 timeval 与对齐方式
            let mut event: libc::input_event = unsafe { std::mem::zeroed() };
            let size = std::mem::size_of::<libc::input_event>();
            let count = unsafe {
                libc::read(
                    self.0.as_raw_fd(),
                    (&mut event as *mut libc::input_event).cast(),
                    size,
                )
            };
            if count < 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::WouldBlock {
                    return Ok(());
                }
                if error.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(error);
            }
            if count == 0 {
                return Ok(());
            }
            if count != size as isize {
                return Err(io::Error::other("键盘事件长度不完整"));
            }
            if event.type_ == 1 {
                game.key(event.code, event.value);
            }
            if event.type_ == 0 && event.code == 3 {
                game.release_keys();
            }
        }
    }
}

extern "C" fn stop(_: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}

pub fn install_signals() -> io::Result<()> {
    for signal in [libc::SIGINT, libc::SIGTERM] {
        if unsafe { libc::signal(signal, stop as *const () as libc::sighandler_t) } == libc::SIG_ERR
        {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

pub fn stopped() -> bool {
    STOP.load(Ordering::Relaxed)
}
