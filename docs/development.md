# 目录组织与内容配置

```text
content/
  levels/                 # 六个关卡各一个 TOML 文件
    01-coast-guard.toml
    02-ice-convoy.toml
    03-canyon-raid.toml
    04-sand-front.toml
    05-marsh-rescue.toml
    06-caldera.toml
  maps/                   # 十二类地图及特色地标、虫洞布局
  music/                  # 24 首分层编配的配乐，带场景与地图标签
assets/
  cockpit/                # 座舱四层索引色美术
  vehicles/               # 战机体素切片
src/
  app/                    # 会话生命周期、跨地图旅行、输入、成绩存储和流程测试
  desktop/                # 高清倍率与后台渲染线程
  audio/                  # 振荡器、乐谱、音效、混音和平台音频输出
  gameplay/
    director.rs           # 阶段推进、增援与任务生成
    combat.rs             # 敌人行为、碰撞和伤害
    objectives.rs         # 检查点、金币、补给和目标选择
    weapons.rs            # 武器、瞄准、炸弹和超载
    progression.rs        # 阶段类型、装备和机体强化
    entities.rs           # 飞机、炮台、弹体、运输机和效果状态
    tests.rs              # 玩法与六关完整流程测试
  content/                # TOML 读取、内容校验、载具与天气定义
  world/                  # 地形生成、噪声、采样与后台地图加载
    celestial.rs          # 月面撞击地形、基地平台与空间实体布置
    regions.rs            # 峡湾、冰山、城市与高地地形
    city.rs               # 城市道路、住宅与高楼分布
    regions_detail.rs     # 植被、农田、谷底、洞穴与天梯
    network.rs            # 跨地图虫洞网络与目的地缓存
    airspace.rs           # 区域保护高度、高建筑缓坡与峡谷谷底采样
    landmarks.rs          # 建筑、风车、灯塔、石柱与障碍高度
    portal.rs             # 虫洞几何变换与线段穿越检测
  render/                 # 渲染管线、地形、飞行器、任务对象、天气和座舱
    celestial.rs          # 星云立方体贴图、天体与基地、飞船网格
    portals.rs            # 对端相机、窗口裁剪与边缘折射
    tilted.rs             # 全角度俯仰的地形网格和云层
    landmarks.rs          # 高楼、风车和灯塔网格
  ui/                     # 菜单、航图、FPV 仪表、战斗状态和像素绘制
  cardputer/              # BMI270、电池与 Wi-Fi 遥测、evdev、RGB565 帧缓冲
  flight/
    mod.rs                # 飞行动力、完整筋斗、横滚与编队
    camera.rs             # 全姿态相机、投影与反投影
    portals.rs            # 相机穿越时机、圆形转场与编队空间变换
    safety.rs             # 曲线路径预测、提前抬升与平滑下降
  main.rs                 # 桌面窗口入口
  bin/cardputer.rs        # 设备入口、性能测量与阶段日志
  bin/audio.rs            # 配乐导出与合成性能测量
```

关卡文件控制区域名称、地图、天气、三个阶段、阶段时长、巡逻配额、炮台配额和增援间隔。地图文件控制种子、水域比例、地形高度倍率、树木密度、云团数量和噪声层级。配置使用类型化 TOML 解析并在测试中校验，随二进制嵌入，修改后重新构建即可部署，运行时无需外部文件路径

默认窗口为 1152 × 612，桌面与设备共享 320 × 170 内部画幅，桌面按比例最近邻放大，调整窗口尺寸时保留宽高比

投影使用 `scale = S / (1 + q × depth)`，`q = 0` 对应俯视的等比例切片效果，追尾视角提高 q 并降低视线俯角

## 验证

```sh
cargo fmt --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test --no-default-features --features cardputer --lib
cargo build --release
cargo run --release -- --capture captures
```

截图模式会打开原生窗口，导出菜单、竞速、金币、空战、Boss、座舱、暂停和四种载具的 PNG，随后退出

57 项单元测试覆盖离地保护、演示、IMU、顺序穿越与越界判定、金币连击、全部武器与补给、六个关卡完整通关、五种载具的完整赛道与金币路线、菜单确认、胜利下一关、失败重试、护航失败和战斗侧栏的中央视野，以及实际音效事件、整曲循环、节拍切换、静音恢复和混音幅值。宇宙场景测试覆盖高速穿越、有效区域与任意接近方向判定、可逆变换、相机投影一致、编队和速度保持、僚机从出口追赶、空间下降、分列绘制的几何一致性，以及实时窗口随对端实体变化和跨地图误拾取防护。设备基准在倒计时结束后测量实际玩法，空战额外包含三个视角的 Boss 场景，性能采样维持双方护盾与双僚机，持续呈现任务或 Boss 的战斗负载；实体显示帧率另行记录
