# 第三方组件与分发说明

Flight Club 原创代码、文档、脚本与 skill 使用根目录的 MIT 许可证。下列第三方组件各自保留原许可证与版权声明

| 组件 | 许可证 | 用途 |
| --- | --- | --- |
| macroquad 0.4 | MIT 或 Apache-2.0，[上游](https://github.com/not-fl3/macroquad) | 桌面窗口、输入与音频 |
| miniquad（随 macroquad） | MIT 或 Apache-2.0，[上游](https://github.com/not-fl3/miniquad) | 图形上下文 |
| glam 0.27 | MIT 或 Apache-2.0，[上游](https://github.com/bitshifter/glam-rs) | 向量与矩阵运算 |
| serde 1 | MIT 或 Apache-2.0，[上游](https://github.com/serde-rs/serde) | 配置与存档序列化 |
| toml 0.9 | MIT 或 Apache-2.0，[上游](https://github.com/toml-rs/toml) | 关卡与地图配置解析 |
| libc 0.2 | MIT 或 Apache-2.0，[上游](https://github.com/rust-lang/libc) | 设备帧缓冲与 evdev 系统调用 |
| memmap2 0.9 | MIT 或 Apache-2.0，[上游](https://github.com/RazrFalcon/memmap2-rs) | 设备资源内存映射 |
| font8x8 0.3 | MIT 或 Public Domain，[上游](https://github.com/dhepper/font8x8) | 8×8 点阵字体 |
| cpal 0.15 | MIT 或 Apache-2.0，[上游](https://github.com/RustAudio/cpal) | macOS 与 Windows 音频输出 |

完整依赖树与版本见 Cargo.lock。分发二进制时请同时附上本文件与根目录 LICENSE。
