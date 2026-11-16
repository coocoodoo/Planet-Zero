fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "--benchmark") {
        rust_fly::audio::export::benchmark(30.0);
    } else {
        let directory = args
            .get(1)
            .ok_or("请指定 WAV 导出目录，或使用 --benchmark")?;
        rust_fly::audio::export::soundtrack(std::path::Path::new(directory))?;
    }
    Ok(())
}
