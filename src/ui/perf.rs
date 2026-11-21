use super::canvas::{CYAN, Canvas, GOLD, MUTED, RED, WHITE};
use crate::app::{App, PerfPanel, PerfStats};

const X: i32 = 18;
const Y: i32 = 12;
const W: i32 = 284;
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

fn background(canvas: &mut Canvas, bounds: [i32; 4]) {
    let [x, y, w, h] = bounds;
    for row in y..y + h {
        let start = (row as usize * crate::app::WIDTH + x as usize) * 4;
        for pixel in canvas.pixels[start..start + w as usize * 4]
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
        {
            let alpha = pixel[3] as u32;
            let combined = 7 * 255 + 3 * alpha;
            for (channel, color) in pixel[..3].iter_mut().zip([8, 12, 20]) {
                *channel = ((color * 7 * 255 + *channel as u32 * alpha * 3) / combined) as u8;
            }
            pixel[3] = (combined / 10) as u8;
        }
    }
}

pub(super) fn draw(app: &mut App) {
    if app.perf_panel == PerfPanel::Off {
        return;
    }
    let stats = &app.perf;
    let mut canvas = Canvas {
        pixels: &mut app.screen,
    };
    if app.perf_panel == PerfPanel::Mini {
        let resource = stats.resources.back().copied().unwrap_or_default();
        let line = format!(
            "{} {:.1}  DRAW {:.1}MS  CPU {}%  MEM {}M",
            if stats.asynchronous { "UI FPS" } else { "FPS" },
            stats.fps,
            stats.timing.map_or(stats.render_ms, |t| t.draw),
            value(resource.cpu, 0),
            value(resource.rss_mb, 1)
        );
        let width = line.len() as i32 * 4 + 8;
        let x = (crate::app::WIDTH as i32 - width) / 2;
        background(&mut canvas, [x, 0, width, 9]);
        text(&mut canvas, x + 4, 2, &line, WHITE);
        return;
    }
    let half = app.perf_panel == PerfPanel::Half;
    background(&mut canvas, [X, Y, W, if half { 60 } else { 110 }]);
    canvas.rect(X, Y, W, 1, MUTED);
    let loading = app.loader.pending.is_some() || app.network.loading();
    if half {
        half_summary(&mut canvas, stats, loading);
    } else {
        full_summary(&mut canvas, stats, loading);
    }
    charts(&mut canvas, stats, half);
}

fn full_summary(canvas: &mut Canvas, stats: &PerfStats, loading: bool) {
    let resource = stats.resources.back().copied().unwrap_or_default();
    let left = X + 4;
    text(
        canvas,
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
        canvas,
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
        canvas,
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
    text(
        canvas,
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
        canvas,
        left,
        Y + 31,
        &format!(
            "TERR {:.1}  CLOUD {:.1}  OBJECT {:.1}  PORTAL {:.1} MS",
            stats.detail[0], stats.detail[1], stats.detail[2], stats.detail[3]
        ),
        CYAN,
    );
}

fn half_summary(canvas: &mut Canvas, stats: &PerfStats, loading: bool) {
    let resource = stats.resources.back().copied().unwrap_or_default();
    let lines = [
        (
            format!(
                "{} {:.1} AVG {:.1} GOAL {:.0}",
                if stats.asynchronous { "UI FPS" } else { "FPS" },
                stats.fps,
                stats.fps_avg,
                stats.target_fps
            ),
            WHITE,
        ),
        (
            format!(
                "FRAME {:.1} P95 {:.1} MAX {:.1}MS",
                stats.frame_ms, stats.frame_p95, stats.frame_peak
            ),
            CYAN,
        ),
        (
            format!(
                "SLOW {:.0}% TRI {} MAP {}",
                stats.slow_percent,
                compact(stats.triangles as f32),
                if loading { "LOADING" } else { "READY" }
            ),
            MUTED,
        ),
        (
            format!(
                "CPU {}/{:.0}% RSS {}M",
                value(resource.cpu, 0),
                stats.cpu_capacity,
                value(resource.rss_mb, 1)
            ),
            GOLD,
        ),
        (
            format!(
                "SWAP {}M FREE {}M",
                value(resource.swap_mb, 1),
                value(resource.available_mb, 0)
            ),
            GOLD,
        ),
        (
            format!(
                "FAULT {}/S TEMP {}C",
                value(resource.faults_per_second, 1),
                value(resource.temperature, 0)
            ),
            MUTED,
        ),
        (
            format!("TERR {:.1} CLOUD {:.1}MS", stats.detail[0], stats.detail[1]),
            CYAN,
        ),
        (
            format!(
                "OBJECT {:.1} PORTAL {:.1}MS",
                stats.detail[2], stats.detail[3]
            ),
            CYAN,
        ),
    ];
    for (i, (line, color)) in lines.iter().enumerate() {
        text(canvas, X + 4, Y + 3 + i as i32 * 7, line, *color);
    }
}

fn charts(canvas: &mut Canvas, stats: &PerfStats, half: bool) {
    let left = if half { X + 172 } else { X + 4 };
    let width = if half { W - 176 } else { W - 8 };
    let budget = 1000.0 / stats.target_fps;
    let maximum = budget * 3.0;
    let span = stats.frames.iter().map(|s| s.ms).sum::<f32>() / 1000.0;
    let label = if half {
        format!("FRAME 0-{maximum:.0}MS {span:.1}S")
    } else {
        format!("FRAME 0-{maximum:.0}MS  {span:.1}S  LIMIT {budget:.1}MS")
    };
    text(canvas, left, Y + if half { 3 } else { 39 }, &label, MUTED);
    graph(
        canvas,
        [
            left,
            Y + if half { 10 } else { 46 },
            width,
            if half { 14 } else { 20 },
        ],
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
            let label = if half {
                format!("{} {:.0}", &label[..1], values[i])
            } else {
                format!("{label} {:.1}MS", values[i])
            };
            text(
                canvas,
                left + i as i32 * if half { 27 } else { 68 },
                Y + if half { 27 } else { 69 },
                &label,
                colors[i],
            );
        }
        let total = values.iter().sum::<f32>().max(0.001);
        let mut sum = 0.0;
        let mut previous = 0;
        for (amount, color) in values.into_iter().zip(colors) {
            sum += amount;
            let end = (sum / total * width as f32).round() as i32;
            canvas.rect(
                left + previous,
                Y + if half { 34 } else { 76 },
                end - previous,
                5,
                color,
            );
            previous = end;
        }
    } else {
        text(
            canvas,
            left,
            Y + if half { 29 } else { 71 },
            &format!(
                "DRAW {:.1}MS {}",
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
    let graph_width = (width - 8) / 2;
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
    for (i, label, maximum, color) in [
        (0, "CPU", stats.cpu_capacity, LOGIC),
        (1, "RSS", maximum_memory, GOLD),
    ] {
        let unit = if i == 0 { "%" } else { "M" };
        let title = if half {
            format!("{label}{maximum:.0}{unit} {seconds:.0}S")
        } else {
            format!("{label} 0-{maximum:.0}{unit}  {seconds:.0}S")
        };
        let x = left + i * (graph_width + 8);
        text(canvas, x, Y + if half { 42 } else { 85 }, &title, color);
        graph(
            canvas,
            [
                x,
                Y + if half { 49 } else { 92 },
                graph_width,
                if half { 7 } else { 14 },
            ],
            stats.resources.len(),
            |n| {
                if i == 0 {
                    stats.resources[n].cpu
                } else {
                    stats.resources[n].rss_mb
                }
            },
            maximum,
            color,
            None,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_background_blends_over_both_scene_and_transparent_overlay() {
        let mut pixels = vec![0; crate::app::WIDTH * crate::app::HEIGHT * 4];
        pixels[4..8].copy_from_slice(&[200, 160, 120, 255]);
        let mut canvas = Canvas {
            pixels: &mut pixels,
        };
        background(&mut canvas, [0, 0, 2, 1]);
        assert_eq!(&canvas.pixels[..4], &[8, 12, 20, 178]);
        assert_eq!(&canvas.pixels[4..8], &[65, 56, 50, 255]);
        assert_eq!(&canvas.pixels[8..12], &[0, 0, 0, 0]);
    }

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
