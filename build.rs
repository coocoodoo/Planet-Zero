use std::{collections::HashMap, env, fs, io::BufReader, path::Path};

#[path = "src/render/palette.rs"]
mod palette;

fn read_png(path: &Path, width: u32, height: u32) -> Vec<[u8; 4]> {
    println!("cargo:rerun-if-changed={}", path.display());
    let mut decoder = png::Decoder::new(BufReader::new(fs::File::open(path).unwrap()));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().unwrap();
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).unwrap();
    assert_eq!(
        (info.width, info.height),
        (width, height),
        "{}",
        path.display()
    );
    buffer[..info.buffer_size()]
        .chunks_exact(info.color_type.samples())
        .map(|p| match info.color_type {
            png::ColorType::Rgba => [p[0], p[1], p[2], p[3]],
            png::ColorType::Rgb => [p[0], p[1], p[2], 255],
            png::ColorType::GrayscaleAlpha => [p[0], p[0], p[0], p[1]],
            png::ColorType::Grayscale => [p[0], p[0], p[0], 255],
            png::ColorType::Indexed => unreachable!(),
        })
        .collect()
}

fn cockpit(path: &Path) -> Vec<u8> {
    let mut palette = vec![[0; 4]];
    let mut lookup = HashMap::from([([0; 4], 0_u8)]);
    let pixels: Vec<_> = read_png(path, 342, 192)
        .into_iter()
        .map(|mut color| {
            assert!(
                matches!(color[3], 0 | 255),
                "{}: alpha must be 0 or 255",
                path.display()
            );
            if color[3] == 0 {
                color = [0; 4];
            }
            *lookup.entry(color).or_insert_with(|| {
                let index =
                    u8::try_from(palette.len()).expect("cockpit palette exceeds 256 colors");
                palette.push(color);
                index
            })
        })
        .collect();
    let mut output = (palette.len() as u16).to_le_bytes().to_vec();
    output.extend(palette.into_iter().flatten());
    let mut start = 0;
    while start < pixels.len() {
        let mut end = start + 1;
        while end < pixels.len() && pixels[end] == pixels[start] && end - start < u16::MAX as usize
        {
            end += 1;
        }
        output.extend_from_slice(&((end - start) as u16).to_le_bytes());
        output.push(pixels[start]);
        start = end;
    }
    output
}

fn main() {
    let output = std::path::PathBuf::from(env::var_os("OUT_DIR").unwrap());
    for vehicle in ["falcon", "dart", "atlas", "wasp", "paper"] {
        for layer in ["canopy", "panel", "deck"] {
            let path = format!("assets/cockpit/{vehicle}/{layer}.png");
            fs::write(
                output.join(format!("{vehicle}-{layer}.bin")),
                cockpit(Path::new(&path)),
            )
            .unwrap();
        }
    }
    println!("cargo:rerun-if-changed=src/render/palette.rs");
    let materials: HashMap<_, _> = palette::JET
        .iter()
        .map(|&(i, r, g, b)| ([r, g, b, 255], i as u8))
        .chain([([0; 4], 0)])
        .collect();
    let model: Vec<_> = read_png(Path::new("assets/vehicles/falcon.png"), 34, 46 * 13)
        .into_iter()
        .map(|color| {
            if color[3] == 0 {
                0
            } else {
                *materials
                    .get(&color)
                    .expect("falcon.png uses a color outside the JET material palette")
            }
        })
        .collect();
    fs::write(output.join("falcon.bin"), model).unwrap();
    // Device bezel for the PC emulator's screenshots (only the emulator binary embeds it)
    let frame: Vec<u8> = read_png(Path::new("docs/media/cardputer-zero-frame.png"), 631, 408)
        .into_iter()
        .flatten()
        .collect();
    fs::write(output.join("cardputer-frame.rgba"), frame).unwrap();
}
