# Cardputer Zero 设备版

设备版面向 Linux ARM64 的 Cardputer Zero，已在 Raspberry Pi Compute Module 0、320 × 170 RGB565 内屏上验证

```sh
rustup target add aarch64-unknown-linux-musl
bash scripts/build-cardputer.sh
python3 scripts/package-cardputer.py
```

构建生成 `dist/flight-club-cardputer` 静态 AArch64 可执行文件和 `dist/flight-club_0.13.0_arm64.deb` 安装包。打包工具使用本机 dpkg-deb，或 Docker 中的 ubuntu:22.04

将安装包复制到设备后执行：

```sh
sudo apt install ./flight-club_0.13.0_arm64.deb
```

APPLaunch 中会出现带战机图标的 Flight Club，选择即可进入主菜单。菜单操作与上方表格一致，操作说明分别提供桌面版与设备版内容，每页七项，每行对应一项操作

设备沿用上方统一快捷键，专用体感按键见「Cardputer Zero 体感」表格。F1、F2、F9 和 F10 使用设备功能键输入，桌面显示功能见「桌面显示与鼠标」表格

启动后默认进入演示背景的主菜单，BMI270 在后台校准，开始飞行后提供体感操控。自由飞行有两架僚机，空战通过 W 补给呼叫支援。按平常玩游戏的姿势握住设备，保持静止约 0.8 秒，校准完成后即可操控，战斗侧栏 K 标记会变成 I；改变握姿时按 K 重新校准

- 左右倾斜：协调转向和侧倾，35° 倾斜对应最大操纵量，常规倾角为 65°
- 前后倾斜：控制俯仰，抬起设备上缘为拉升，30° 倾斜对应最大操纵量；筋斗需先按住对应方向键 3 秒，松开后 0.3 秒内再按两次
- 水平转动：陀螺仪角速度提供附加转向输入
- 体感具有 2.5° 死区，F、X、Z、C 可覆盖对应操纵轴
- 按 I 关闭体感后使用实体键，松开操纵键保持航向并逐渐恢复水平姿态
- 按 7 开启自动巡航时关闭体感

## 实现与调试

设备版通过 `glam` 复用场景数学，直接向 fbdev mmap 写入 RGB565，再用 evdev 接收完整按下/松开事件，运行时依赖静态链接在可执行文件中

屏幕路径优先读取 `FLIGHT_CLUB_FB`、`APPLAUNCH_LINUX_FBDEV_DEVICE`、`LV_LINUX_FBDEV_DEVICE`，随后使用 `/dev/fb_lcd` 或扫描 `/proc/fb`；打开后通过 ioctl 校验尺寸、RGB565 位布局与 stride。键盘支持 `FLIGHT_CLUB_KEYBOARD`、`APPLAUNCH_LINUX_KEYBOARD_DEVICE`，默认使用 I²C 键盘的 by-path 节点

设备上可独立运行 CPU 基准，或限定帧数检查实际显示：

```sh
flight-club-cardputer --benchmark 180 --scale 1 --capture-dir /tmp/flight-club-captures
flight-club-cardputer --mode combat --frames 300 --view cockpit --weather storm
flight-club-cardputer --mode race --vehicle 1 --level 3
flight-club-cardputer --mode combat --level 2 --view cockpit
flight-club-cardputer --mode combat --benchmark 180 --capture-dir /tmp/combat-check
flight-club-cardputer --map moon --mode free --view chase
flight-club-cardputer --map space --mode free --view cockpit
flight-club-cardputer --map space --benchmark 180 --capture-dir /tmp/space-check
flight-club-cardputer --map city --maneuvers 330 --capture-dir /tmp/flight-loops
flight-club-cardputer --map canyon --weather sandstorm --demo on
flight-club-cardputer --map alpine --seed 8082 --weather snow
flight-club-cardputer --imu-test 10 --imu-csv /tmp/flight-club-imu.csv
```

直接运行显示测试前暂停 APPLaunch 用户服务，结束后恢复；从 APPLaunch 正常启动时由启动器管理前台应用生命周期

已通过真机 framebuffer 截图、evdev 事件回放和 IIO 姿态回放检查转向、横滚、爬升、加速松开、视角切换、僚机数量、体感开关、重新校准、菜单及长按 Esc 退出。真实 BMI270 的读取和静止校准已验证，手持运动的方向与控制链路另用确定性姿态回放验证
