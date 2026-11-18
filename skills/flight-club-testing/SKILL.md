---
name: flight-club-testing
description: flight-club 飞行模拟器的离屏测试方法。用于验证飞行物理、输入行为、HUD 显示等改动，不需要打开窗口或人工操作。当修改 flight、app、ui 等模块后需要跑数值断言或生成 HUD 截图时使用
---

# flight-club-testing 离屏测试

本项目可完全离屏测试：`App` 与 `Flight` 的计算和绘制都作用于内存缓冲 `app.screen`（RGBA `Vec<u8>`，`WIDTH = 320`、`HEIGHT = 170`），不依赖窗口或 GPU。整套流程分三层：单元测试、飞行逻辑 probe、HUD 截图。

## 1. 单元测试

改完先跑全量测试，确认无回归：

```
cargo test
```

只跑相关用例按名称过滤：

```
cargo test hover
```

项目已有测试在 `src/flight/tests.rs`、`src/app/tests.rs`。新增行为尽量补一条对应测试，用 `App::new(World::generate(MapKind::X, seed))` 构造，按需 `app.start()`、设 `app.auto = false`、用 `app.key(code, 1)` / `app.key(code, 0)` 模拟按键，再循环 `app.step(dt, None)` 推进并断言状态。

## 2. 飞行逻辑 probe

需要观察随时间演化的具体数值或断言物理行为时，写一个临时的独立可执行文件，直接调用库，离屏 `step` 后打印数值并断言。

推荐放到 `examples/` 下用 cargo 运行（会自动链接依赖）：

```
cargo run --example <名称>
```

示例骨架：

```rust
use rust_fly::flight::{Controls, Flight};
use rust_fly::world::{MapKind, World};

fn main() {
    let world = World::generate(MapKind::Moon, 7);
    let mut f = Flight::new(&world);
    f.position.z = 900.0;
    let controls = Controls { throttle: Some(1.0), manual: true, aerobatic: true, ..Default::default() };
    let start = f.position;
    for _ in 0..60 {
        f.step(1.0 / 30.0, &controls, &world);
    }
    println!("speed={:.1} moved={:.1}", f.speed, (f.position - start).length());
    assert!(f.speed > 50.0);
}
```

也可以不经 cargo、直接链接已构建的库：

```
cargo build --lib
rustc --edition 2024 probe.rs \
  --extern rust_fly=target/debug/librust_fly.rlib \
  -L dependency=target/debug/deps -O -o probe
./probe
```

要点：
- 用 `dt = 1.0 / 30.0` 或 `0.1` 推进；不同 dt 只影响收敛帧数
- 每个场景独立构造 `Flight` / `App`，避免状态串扰
- 打印足够字段（速度、位置、高度、偏航等）便于定位问题，再用 `assert!` 固化预期

## 3. HUD 截图

2D HUD 通过 `app.draw_overlay()` 绘制到 `app.screen`（内部先清屏再调用 `ui::draw`），不渲染 3D，适合检查仪表与文字。含 3D 场景时用 `app.draw()`。

把 `screen` 的 RGBA 取前三通道写成 P6 PPM：

```rust
use std::fs;
use std::path::Path;

fn write_ppm(app: &rust_fly::app::App, path: &Path) {
    let mut data = b"P6\n320 170\n255\n".to_vec();
    for pixel in app.screen.chunks_exact(4) {
        data.extend_from_slice(&pixel[..3]);
    }
    fs::write(path, data).unwrap();
}
```

渲染前无需手动清屏（`draw_overlay` 会清），如要自定义背景可先 `app.screen.fill(12)` 再调用绘制。

再转 PNG 查看：

```python
from PIL import Image
im = Image.open("/tmp/x.ppm")
im.save("/tmp/x.png")
```

同一状态切换前后各截一张做对比，最容易看出差异。

## 4. 常用接口

- `App::new(World::generate(MapKind::X, seed))`、`app.start()`、`app.step(dt, None)`、`app.key(code, value)`
- `app.page`（`Page`）、`app.flight`（`Flight`）、`app.screen`
- `Flight::new(&world)`、`f.step(dt, &controls, &world)`、`f.position`、`f.speed`、`f.yaw`、`f.space`
- `Controls { throttle: Option<f32>, turn: f32, climb: f32, boost: bool, manual: bool, aerobatic: bool, ..Default::default() }`
- 按键常量在 `rust_fly::keys`（`HOVER`、`LEFT`、`RIGHT`、`UP`、`DOWN`、`BOOST` 等）
- `MapKind::Moon` 为大气星球，`MapKind::Space` 为太空；`Flight::new` 据 `world.kind` 设置 `space`

## 5. 归档与清理

- 截图等临时产物归档到 `target/<主题>/`（`target` 已被 git 忽略），不污染仓库
- 临时的 `examples/<名称>.rs` 或 `probe.rs` 在验证完成后删除
- probe 只用于验证，不作为项目长期测试；稳定的行为预期应固化到 `#[test]` 用例中
