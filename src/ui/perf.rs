use super::canvas::{CYAN, Canvas, GOLD, MUTED, RED, WHITE};
use crate::app::App;

const X: i32 = 92;
const Y: i32 = 12;
const W: i32 = 158;
const H: i32 = 48;

// 3x5 像素字体，每个字符占 4 列
fn glyph(c: char) -> [u8; 5] {
    match c {
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [7, 1, 7, 4, 7],
        '3' => [7, 1, 7, 1, 7],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 7, 1, 7],
        '6' => [7, 4, 7, 5, 7],
        '7' => [7, 1, 1, 1, 1],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 7],
        'A' => [2, 5, 7, 5, 5],
        'B' => [6, 5, 6, 5, 6],
        'C' => [3, 4, 4, 4, 3],
        'D' => [6, 5, 5, 5, 6],
        'E' => [7, 4, 6, 4, 7],
        'F' => [7, 4, 6, 4, 4],
        'G' => [3, 4, 5, 5, 3],
        'H' => [5, 5, 7, 5, 5],
        'I' => [7, 2, 2, 2, 7],
        'K' => [5, 5, 6, 5, 5],
        'L' => [4, 4, 4, 4, 7],
        'M' => [5, 7, 7, 5, 5],
        'N' => [5, 7, 7, 7, 5],
        'O' => [2, 5, 5, 5, 2],
        'P' => [6, 5, 6, 4, 4],
        'R' => [6, 5, 6, 5, 5],
        'S' => [3, 4, 2, 1, 6],
        'T' => [7, 2, 2, 2, 2],
        'U' => [5, 5, 5, 5, 7],
        'V' => [5, 5, 5, 5, 2],
        'W' => [5, 5, 7, 7, 5],
        '%' => [5, 1, 2, 4, 5],
        '.' => [0, 0, 0, 0, 2],
        '/' => [1, 1, 2, 4, 4],
        _ => [0, 0, 0, 0, 0],
    }
}

fn text(canvas: &mut Canvas, x: i32, y: i32, value: &str, color: [u8; 3]) {
    for (i, ch) in value.chars().enumerate() {
        let bits = glyph(ch.to_ascii_uppercase());
        for (row, mask) in bits.iter().enumerate() {
            for col in 0..3 {
                if mask & (4 >> col) != 0 {
                    canvas.pixel(x + i as i32 * 4 + col, y + row as i32, color);
                }
            }
        }
    }
}

fn compact(value: f32) -> String {
    if value >= 10000.0 {
        format!("{:.1}K", value / 1000.0)
    } else {
        format!("{:.0}", value)
    }
}

pub(super) fn draw(app: &mut App) {
    if !app.perf_show {
        return;
    }
    let mut canvas = Canvas {
        pixels: &mut app.screen,
    };
    canvas.rect(X, Y, W, H, [8, 12, 20]);
    canvas.rect(X, Y, W, 1, MUTED);
    let fps_color = if app.perf.fps < 25.0 { RED } else { WHITE };
    text(
        &mut canvas,
        X + 3,
        Y + 3,
        &format!(
            "FPS {:.1} AVG {:.1} MIN {:.1}",
            app.perf.fps, app.perf.fps_avg, app.perf.fps_min
        ),
        fps_color,
    );
    text(
        &mut canvas,
        X + 3,
        Y + 10,
        &format!("FT {:.1} PK {:.1}", app.perf.frame_ms, app.perf.frame_peak),
        CYAN,
    );
    text(
        &mut canvas,
        X + 3,
        Y + 17,
        &format!("DRAW {:.1} CPU {:.0}%", app.perf.render_ms, app.perf.cpu),
        CYAN,
    );
    text(
        &mut canvas,
        X + 3,
        Y + 24,
        &format!(
            "TRI {} MEM {}M",
            compact(app.renderer.triangles as f32),
            compact(app.perf.mem_mb)
        ),
        GOLD,
    );

    let gx = X + 3;
    let gy = Y + 32;
    let gw = W - 6;
    let gh = 13;
    canvas.rect(gx, gy, gw, gh, [10, 16, 26]);
    let samples = app.perf.curve.len().min(gw as usize);
    for i in 0..samples {
        let value = app.perf.curve[app.perf.curve.len() - samples + i];
        let height = ((value / 60.0).clamp(0.0, 1.0) * (gh - 1) as f32) as i32;
        let color = if value < 25.0 {
            RED
        } else if value < 45.0 {
            GOLD
        } else {
            CYAN
        };
        canvas.rect(gx + i as i32, gy + gh - 1 - height, 1, height.max(1), color);
    }
}
