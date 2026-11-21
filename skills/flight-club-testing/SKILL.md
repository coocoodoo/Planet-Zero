---
name: flight-club-testing
description: flight-club 飞行模拟器的测试方法，包含离屏测试与 CardputerZero 真机测试。离屏测试用于验证飞行物理、输入行为、HUD 显示等改动，不需要打开窗口或人工操作；真机测试用于验证屏幕呈现、按键、音频与 IMU。当修改 flight、app、ui 等模块后需要跑数值断言、生成截图或上真机验证时使用
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
use flight_club::flight::{Controls, Flight};
use flight_club::world::{MapKind, World};

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
  --extern flight_club=target/debug/libflight_club.rlib \
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

fn write_ppm(app: &flight_club::app::App, path: &Path) {
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
- 按键常量在 `flight_club::keys`（`HOVER`、`LEFT`、`RIGHT`、`UP`、`DOWN`、`BOOST` 等）
- `MapKind::Moon` 为大气星球，`MapKind::Space` 为太空；`Flight::new` 据 `world.kind` 设置 `space`

## 5. 真机测试（CardputerZero）

离屏测试只能证明计算与离屏绘制正确，屏幕呈现、按键、音频、IMU 仍需上真机验证。

### 5.1 连接

设备凭据统一放在仓库外的设备信息目录 `/home/dev/projects/.secrets/`，文件为 `.device.env`（包含 `DEVICE_IP`、`DEVICE_USER`、`DEVICE_PASS`、`DEVICE_SUDO_PASS`，`DEVICE_SUDO_PASS` 留空时复用 `DEVICE_PASS`），通过环境变量导入，不复制进仓库、不提交、不在输出中打印口令。连接前先确认设备 IP，换网后直接修改该文件中的 `DEVICE_IP`。

```sh
DEVICE_SECRETS=/home/dev/projects/.secrets
set -a; . "$DEVICE_SECRETS/.device.env"; set +a
: "${DEVICE_SUDO_PASS:=$DEVICE_PASS}"
# 设备只接受口令登录；用同目录的 askpass.sh（仅回显 $DEVICE_PASS，不含口令）免交互输入，无需 sshpass
export DEVICE_PASS SSH_ASKPASS="$DEVICE_SECRETS/askpass.sh" SSH_ASKPASS_REQUIRE=force
export DEVICE_HOST="${DEVICE_USER:-pi}@${DEVICE_IP}"
ssh "$DEVICE_HOST" -o ConnectTimeout=8 'uname -m; ls /dev/fb_lcd'
```

连接脚本通过 `DEVICE_HOST` 传入主机地址。`asroot`、`copy`、`fetch` 等封装可以手工展开成 `ssh`/`scp`。

设备是 aarch64 Debian（CardputerZero），屏幕 `/dev/fb_lcd` 为 320×170 RGB565，键盘事件节点 `/dev/input/by-path/platform-3f804000.i2c-event`。

### 5.2 构建与部署

```sh
bash scripts/build-cardputer.sh   # 脚本可能没有执行位，用 bash 调用
scp dist/flight-club-cardputer "$DEVICE_HOST":/tmp/flight-club-cardputer
printf '%s\n' "$DEVICE_SUDO_PASS" | ssh "$DEVICE_HOST" 'sudo -S -p "" install -m 755 /tmp/flight-club-cardputer /usr/share/APPLaunch/bin/flight-club-cardputer'
```

交叉编译使用 `aarch64-unknown-linux-musl`、`cortex-a53`、`link-self-contained`；脚本已封装这些参数，产物固定为 `dist/flight-club-cardputer`。

### 5.3 端到端自检

端到端自检按以下流程实现：

1. `systemctl --user stop APPLaunch.service`
2. 建 FIFO 作为键盘，设 `FLIGHT_CLUB_KEYBOARD=<fifo>`、`FLIGHT_CLUB_SETTINGS=<root>/settings.toml`、`FLIGHT_CLUB_RECORDS=<root>/records.txt`
3. 启动 `flight-club-cardputer --mode free --map moon --view chase --frames 2400 --audio on --imu off --capture-dir <root>`
4. 等 stderr 出现 `"event":"state"` 表示进入游戏
5. 用 `struct.pack('qqHHi', 0, 0, 1, code, value)` 写 FIFO 注入按键，value 1 按下、0 松开，中间留 90ms
6. 直接读 `/dev/fb_lcd` 前 320×170×2 字节，`<H` 解 RGB565 转 RGB，写 P6 PPM
7. ESC 退出，断言退出码 0；`pactl list short sink-inputs` 检查 `application.name == "Flight Club"` 的音频流
8. 结束时 `systemctl --user start APPLaunch.service` 恢复桌面

按键码就是 `src/keys.rs` 的常量值，也是 Linux evdev 键码：`BOOST=42`（左 Shift）、`SPACE=57`、`ESC=1`、方向键 `103/105/106/108`、`HOVER=56`（左 Alt）。Cardputer 的实体方向键会先经过 `keys::cardputer_direction` 把 `33/45/44/46` 映射为上下左右。

改动输入键位后先确认设备实际发出的 evdev 码，再改 `src/keys.rs` 常量，否则桌面上能用的键在真机上可能无效。

## 6. 归档与清理

- 截图等临时产物归档到 `target/<主题>/`（`target` 已被 git 忽略），不污染仓库
- 临时的 `examples/<名称>.rs` 或 `probe.rs` 在验证完成后删除
- probe 只用于验证，不作为项目长期测试；稳定的行为预期应固化到 `#[test]` 用例中
