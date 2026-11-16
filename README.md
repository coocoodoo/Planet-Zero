# Rust Fly

使用 Rust 和 macroquad 复刻的像素飞行场景，核心是 Sprite Stacking、高度图和 CPU 软件渲染

```sh
cd <user-home>/src/rust-fly
cargo run --release
```

| 操作 | 效果 |
| --- | --- |
| 移动鼠标 | 俯视时指向航向，追尾和座舱中左右转向、下移爬升、上移下降 |
| WASD / 方向键 | 左右转向，上键爬升，下键下降 |
| 短按鼠标左键 / Q | 翻滚 |
| 按住鼠标左键 / Shift | 加速 |
| Space | 俯视、追尾、座舱之间平滑切换 |
| B | 展开 / 合拢战机切片 |
| V | 切换 0 / 1 / 2 架僚机 |
| T | 白天、日落、夜间 |
| P / R | 暂停 / 重置飞行 |
| H / F / Esc | 提示显隐 / 全屏 / 退出 |

鼠标停止移动 5 秒、且没有方向键输入时恢复自动巡航

## Cardputer Zero 设备版

设备版面向 Linux ARM64 的 Cardputer Zero，已在 Raspberry Pi Compute Module 0、320 × 170 RGB565 内屏上验证

```sh
rustup target add aarch64-unknown-linux-musl
bash scripts/build-cardputer.sh
python3 scripts/package-cardputer.py
```

构建生成 `dist/rust-fly-cardputer` 静态 AArch64 可执行文件和 `dist/rust-fly_0.2.0_arm64.deb` 安装包。打包工具使用本机 dpkg-deb，或 Docker 中的 ubuntu:22.04

将安装包复制到设备后执行：

```sh
sudo apt install ./rust-fly_0.2.0_arm64.deb
```

APPLaunch 中会出现带战机图标的 Rust Fly，选择即可启动

| 实体键 | 功能 |
| --- | --- |
| Z / X / C / F | 左转 / 下降 / 右转 / 爬升 |
| 4 / Space | 切换视角 |
| 5 / Q | 翻滚 |
| 按住 6 / Shift | 加速，松开后恢复巡航速度 |
| 7 / T | 切换光照 |
| 8 / H / 短按 Esc | 打开或关闭暂停菜单 |
| Enter | 关闭菜单并继续 |
| 长按 Esc 2 秒 | 退出并返回 APPLaunch |
| V | 切换 0 / 1 / 2 架僚机 |
| I | 开关体感，开启时自动校准 |
| K | 将当前握持姿态重新校准为中立位置 |
| Tab | 自动巡航 / 手动驾驶 |
| B / R / P | 展开切片 / 重置 / 暂停 |

启动后默认进入座舱视角，开启 BMI270 体感操控，并显示两架僚机。按平常玩游戏的姿势握住设备，保持静止约 0.8 秒，顶部 `CAL` 变成 `IMU` 后即可操控；改变握姿时按 K 重新校准

- 左右倾斜：协调转向和横滚，35° 倾斜对应最大操纵量，飞机最大倾角为 65°
- 前后倾斜：控制爬升和俯仰，抬起设备上缘为爬升，30° 倾斜对应最大操纵量
- 水平转动：陀螺仪角速度提供附加转向输入
- 体感具有 2.5° 死区，实体方向键可覆盖对应操纵轴
- 按 I 关闭体感后使用实体键，松开方向键保持航向与高度；Tab 开启自动巡航并关闭体感

内部画幅固定为 **320 × 170**，每个屏幕像素独立渲染，目标提交频率为 30 FPS。渲染采用列连续像素存储、三线程地形分区、预计算雾色和光照、单次体素投影、战机局部轮廓扫描，以及座舱遮挡区域裁剪

真机 180 帧移动场景测量：俯视 15.27 ms、追尾 17.78 ms、座舱 28.45 ms，均包含两架僚机。完整座舱与真实 IMU 同时运行时，300 帧用时 10.069 秒，约 29.8 FPS；绘制平均 27.98 ms，RGB565 提交平均 1.96 ms。帧率随姿态、可见地形和系统负载变化

设备版通过 `glam` 复用场景数学，直接向 fbdev mmap 写入 RGB565，再用 evdev 接收完整按下/松开事件，运行时依赖静态链接在可执行文件中

屏幕路径优先读取 `RUST_FLY_FB`、`APPLAUNCH_LINUX_FBDEV_DEVICE`、`LV_LINUX_FBDEV_DEVICE`，随后使用 `/dev/fb_lcd` 或扫描 `/proc/fb`；打开后通过 ioctl 校验尺寸、RGB565 位布局与 stride。键盘支持 `RUST_FLY_KEYBOARD`、`APPLAUNCH_LINUX_KEYBOARD_DEVICE`，默认使用 I²C 键盘的 by-path 节点

设备上可独立运行 CPU 基准，或限定帧数检查实际显示：

```sh
rust-fly-cardputer --benchmark 180 --scale 1 --capture-dir /tmp/rust-fly-captures
rust-fly-cardputer --frames 300 --view cockpit
rust-fly-cardputer --imu-test 10 --imu-csv /tmp/rust-fly-imu.csv
```

直接运行显示测试前暂停 APPLaunch 用户服务，结束后恢复；从 APPLaunch 正常启动时由启动器管理前台应用生命周期

已通过真机 framebuffer 截图、evdev 事件回放和 IIO 姿态回放检查转向、横滚、爬升、加速松开、视角切换、僚机数量、体感开关、重新校准、菜单及长按 Esc 退出。真实 BMI270 的读取和静止校准已验证，手持运动的方向与控制链路另用确定性姿态回放验证

## 实现

- `src/world.rs`：六个尺度的周期噪声、海平面分位数、阶梯地形、锥形树冠、云团和固定种子
- `src/flight.rs`：飞行状态、机体坐标系、翻滚、轨迹、三种视角及统一投影
- `src/render.rs`：多线程高度柱绘制、深度遮挡、切片采样、三机编队、阴影和尾焰
- `src/cockpit.rs`：四层座舱、分层视差、光照、HUD、雷达、姿态球、仪表、油门和操纵杆
- `src/cardputer/imu.rs`：IIO 采样、静止校准、重力与角速度融合、姿态控制和诊断 CSV
- `src/main.rs`：窗口、输入、最近邻纹理显示和截图验证入口
- `src/cardputer/`：设备控制、像素 HUD、RGB565 显示、evdev 输入和退出信号
- `src/bin/cardputer.rs`：设备运行与性能测量入口
- `assets/cockpit-*.bin`：原生 320 × 170 座舱的四层索引色美术，含用于分层视差的边缘余量
- `assets/jet.bin`：34 × 46 × 13 的战机切片，零表示透明，其余值为调色板索引

默认窗口为 1152 × 648，内部画幅为 384 × 216，最终使用最近邻采样放大

投影使用 `scale = S / (1 + q × depth)`，`q = 0` 对应俯视的等比例切片效果，追尾视角提高 q 并降低视线俯角

## 验证

```sh
cargo fmt --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test --no-default-features --features cardputer --lib
cargo build --release
cargo run --release -- --capture captures
```

截图模式会打开原生窗口，导出六个场景的 PNG 和 `verification.json`，检查移动、爬升、加速、转向、翻滚、轨迹和视角过渡，随后退出

测量记录为同一固定姿态连续绘制 30 帧的平均 CPU 时间，包含座舱离屏画布和像素转换，窗口显示、GPU 提交与垂直同步单独发生

## 来源与复刻范围

参考页面：[Fighter jet scene](https://forge.ui8.ai/api/fragment-preview/frag_1eb8944c-57b1-42ca-a909-5c2d7e517c24.1794887081.iF-OXPMS98qN2dokSnEW1qD4shhsBPqA1j363uDuFIA/?component=var_54ce6aac-b1aa-40c8-af96-1422cf55e018)

战机切片和调色板来自该页面公开加载的 `src/lib/jet-model.ts`，将其 Base64/RLE 数据解码为二进制后随程序嵌入；四层座舱美术由原页面 `cockpit.ts` 的构建函数离线生成索引色资源；地形、投影、动态仪表和飞行交互以 Rust 重写

当前版本包含完整座舱静态美术、动态仪表与 HUD、两架带尾迹/尾焰/投影的僚机、三种视角和光照，以及 Cardputer Zero 的体感与实体键操控。尾焰采用软件像素绘制；触摸、手柄、原版 WebGL 平滑火焰及全部调参面板保留在参考项目中

IMU 接口与坐标轴参考设备已安装的 Compass 及其[公开实现](https://github.com/CardputerZero/Compass/blob/main/src/models/compass_pose.cpp)。BMI270 原始坐标到屏幕坐标为 `(Y, -X, Z)`；运行时从 `in_accel_scale` 和 `in_anglvel_scale` 读取 SI 单位比例，以独立线程每 10 ms 读取六个 IIO 通道，用陀螺仪预测重力方向并通过加速度修正，再相对校准握姿求横滚与俯仰。转向使用倾斜量及偏航角速度，适合飞行操控，不依赖磁北航向

IMU 默认按 IIO `name=bmi270` 自动发现，测试时可通过 `RUST_FLY_IMU` 指定包含同名通道的目录。采样失效或超过 150 ms 未更新时显示 `FAIL` 并保留键盘操作；`--imu off` 可用于关闭体感的显示基准

项目包含原页面的战机美术数据，来源保留上述记录，项目未另行授予这些素材的许可
