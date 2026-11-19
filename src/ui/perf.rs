use super::canvas::{CYAN, Canvas, GOLD, MUTED, RED, WHITE};
use crate::app::App;

const X: i32 = 18;
const Y: i32 = 12;
const W: i32 = 284;
const H: i32 = 110;
const LOGIC: [u8; 3] = [114, 211, 135];
const OUTPUT: [u8; 3] = [188, 139, 236];
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
        'J' => [1, 1, 1, 5, 2],
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
        'X' => [5, 5, 2, 5, 5],
        'Y' => [5, 5, 2, 2, 2],
        '-' => [0, 0, 7, 0, 0],
        ':' => [0, 2, 0, 2, 0],
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

fn value(value: Option<f32>, decimals: usize) -> String {
    value.map_or_else(|| "--".into(), |v| format!("{v:.decimals$}"))
}

fn graph(
    canvas: &mut Canvas,
    bounds: [i32; 4],
    count: usize,
    sample: impl Fn(usize) -> Option<f32>,
    maximum: f32,
    color: [u8; 3],
    limit: Option<f32>,
) {
    let [x, y, w, h] = bounds;
    canvas.rect(x, y, w, h, [14, 23, 34]);
    if count == 0 {
        return;
    }
    // 每列取对应采样区间的峰值，压缩历史时保留卡顿尖峰
    for column in 0..w as usize {
        let start = column * count / w as usize;
        let end = ((column + 1) * count).div_ceil(w as usize);
        let peak = (start..end).filter_map(&sample).reduce(f32::max);
        if let Some(peak) = peak {
            let height = ((peak / maximum).clamp(0.0, 1.0) * (h - 1) as f32).round() as i32;
            let color = if limit.is_some_and(|limit| peak > limit * 1.1) {
                RED
            } else {
                color
            };
            canvas.rect(
                x + column as i32,
                y + h - height.max(1),
                1,
                height.max(1),
                color,
            );
        }
    }
    if let Some(limit) = limit {
        let row = y + h - 1 - (limit / maximum * (h - 1) as f32).round() as i32;
        for column in (0..w).step_by(4) {
            canvas.pixel(x + column, row, WHITE);
        }
    }
}

pub(super) fn draw(app: &mut App) {
    if !app.perf_show {
        return;
    }
    let stats = &app.perf;
    let resource = stats.resources.back().copied().unwrap_or_default();
    let mut canvas = Canvas {
        pixels: &mut app.screen,
    };
    canvas.rect(X, Y, W, H, [8, 12, 20]);
    canvas.rect(X, Y, W, 1, MUTED);
    let left = X + 4;
    let width = W - 8;
    text(
        &mut canvas,
        left,
        Y + 3,
        &format!(
            "{} {:.1}  AVG {:.1}  P95 {:.1}MS  SLOW {:.0}%",
            if stats.asynchronous { "UI FPS" } else { "FPS" },
            stats.fps,
            stats.fps_avg,
            stats.frame_p95,
            stats.slow_percent
        ),
        WHITE,
    );
    text(
        &mut canvas,
        left,
        Y + 10,
        &format!(
            "FRAME {:.1}MS  MAX {:.1}MS  TRI {}  GOAL {:.0}",
            stats.frame_ms,
            stats.frame_peak,
            compact(stats.triangles as f32),
            stats.target_fps
        ),
        CYAN,
    );
    text(
        &mut canvas,
        left,
        Y + 17,
        &format!(
            "CPU {}/{:.0}%  RSS {}M  SWAP {}M  FREE {}M",
            value(resource.cpu, 0),
            stats.cpu_capacity,
            value(resource.rss_mb, 1),
            value(resource.swap_mb, 1),
            value(resource.available_mb, 0)
        ),
        GOLD,
    );
    let loading = app.loader.pending.is_some() || app.network.loading();
    text(
        &mut canvas,
        left,
        Y + 24,
        &format!(
            "FAULT {}/S  TEMP {}C  MAP {}",
            value(resource.faults_per_second, 1),
            value(resource.temperature, 0),
            if loading { "LOADING" } else { "READY" }
        ),
        if loading || resource.swap_mb.is_some_and(|m| m > 0.0) {
            GOLD
        } else {
            MUTED
        },
    );
    text(
        &mut canvas,
        left,
        Y + 31,
        &format!(
            "TERR {:.1}  CLOUD {:.1}  OBJECT {:.1}  PORTAL {:.1} MS",
            stats.detail[0], stats.detail[1], stats.detail[2], stats.detail[3]
        ),
        CYAN,
    );
    let budget = 1000.0 / stats.target_fps;
    let maximum = budget * 3.0;
    let span = stats.frames.iter().map(|s| s.ms).sum::<f32>() / 1000.0;
    text(
        &mut canvas,
        left,
        Y + 39,
        &format!("FRAME 0-{maximum:.0}MS  {span:.1}S  LIMIT {budget:.1}MS"),
        MUTED,
    );
    graph(
        &mut canvas,
        [left, Y + 46, width, 20],
        stats.frames.len(),
        |i| Some(stats.frames[i].ms),
        maximum,
        CYAN,
        Some(budget),
    );
    if let Some(timing) = stats.timing {
        let values = [timing.logic, timing.draw, timing.present, timing.wait];
        let colors = [LOGIC, CYAN, OUTPUT, MUTED];
        for (i, label) in ["LOGIC", "DRAW", "OUT", "WAIT"].iter().enumerate() {
            text(
                &mut canvas,
                left + i as i32 * 68,
                Y + 69,
                &format!("{label} {:.1}MS", values[i]),
                colors[i],
            );
        }
        let total = values.iter().sum::<f32>().max(0.001);
        let mut sum = 0.0;
        let mut previous = 0;
        for (amount, color) in values.into_iter().zip(colors) {
            sum += amount;
            let end = (sum / total * width as f32).round() as i32;
            canvas.rect(left + previous, Y + 76, end - previous, 5, color);
            previous = end;
        }
    } else {
        text(
            &mut canvas,
            left,
            Y + 71,
            &format!(
                "DRAW {:.1}MS  {}",
                stats.render_ms,
                if stats.asynchronous {
                    "ASYNC"
                } else {
                    "SAMPLING"
                }
            ),
            CYAN,
        );
    }
    let half = (width - 8) / 2;
    let maximum_memory = stats
        .resources
        .iter()
        .filter_map(|r| r.rss_mb)
        .fold(64.0, f32::max);
    let maximum_memory = (maximum_memory / 32.0).ceil() * 32.0;
    let seconds = stats
        .resources
        .back()
        .zip(stats.resources.front())
        .map_or(0.0, |(last, first)| last.elapsed - first.elapsed);
    text(
        &mut canvas,
        left,
        Y + 85,
        &format!("CPU 0-{:.0}%  {seconds:.0}S", stats.cpu_capacity),
        LOGIC,
    );
    text(
        &mut canvas,
        left + half + 8,
        Y + 85,
        &format!("RSS 0-{maximum_memory:.0}M  {seconds:.0}S"),
        GOLD,
    );
    graph(
        &mut canvas,
        [left, Y + 92, half, 14],
        stats.resources.len(),
        |i| stats.resources[i].cpu,
        stats.cpu_capacity,
        LOGIC,
        None,
    );
    graph(
        &mut canvas,
        [left + half + 8, Y + 92, half, 14],
        stats.resources.len(),
        |i| stats.resources[i].rss_mb,
        maximum_memory,
        GOLD,
        None,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_spans_the_width_and_keeps_compressed_spikes() {
        let mut pixels = vec![0; crate::app::WIDTH * crate::app::HEIGHT * 4];
        let mut canvas = Canvas {
            pixels: &mut pixels,
        };
        graph(
            &mut canvas,
            [10, 10, 276, 20],
            96,
            |_| Some(30.0),
            100.0,
            CYAN,
            None,
        );
        for x in 10..286 {
            let at = (29 * crate::app::WIDTH + x) * 4;
            assert_eq!(&canvas.pixels[at..at + 3], &CYAN);
        }
        graph(
            &mut canvas,
            [10, 40, 80, 20],
            180,
            |i| Some(if i == 83 { 100.0 } else { 20.0 }),
            100.0,
            CYAN,
            Some(33.3),
        );
        assert!((10..90).any(|x| {
            let at = (41 * crate::app::WIDTH + x) * 4;
            canvas.pixels[at..at + 3] == RED
        }));
    }
}
