# 来源与复刻范围

参考页面：[Fighter jet scene](https://forge.ui8.ai/api/fragment-preview/frag_1eb8944c-57b1-42ca-a909-5c2d7e517c24.1794887081.iF-OXPMS98qN2dokSnEW1qD4shhsBPqA1j363uDuFIA/?component=var_54ce6aac-b1aa-40c8-af96-1422cf55e018)

战机切片和调色板来自该页面公开加载的 `src/lib/jet-model.ts`，将其 Base64/RLE 数据解码为二进制后随程序嵌入；四层座舱美术由原页面 `cockpit.ts` 的构建函数离线生成索引色资源；地形、投影、动态仪表和飞行交互以 Rust 重写

当前版本包含完整座舱静态美术、动态仪表与 HUD、两架带尾迹/尾焰/投影的僚机、三种视角和光照，以及 Cardputer Zero 的体感与实体键操控。尾焰采用软件像素绘制；触摸、手柄、原版 WebGL 平滑火焰及全部调参面板保留在参考项目中

IMU 接口与坐标轴参考设备已安装的 Compass 及其[公开实现](https://github.com/CardputerZero/Compass/blob/main/src/models/compass_pose.cpp)。BMI270 原始坐标到屏幕坐标为 `(Y, -X, Z)`；运行时从 `in_accel_scale` 和 `in_anglvel_scale` 读取 SI 单位比例，以独立线程每 10 ms 读取六个 IIO 通道，用陀螺仪预测重力方向并通过加速度修正，再相对校准握姿求横滚与俯仰。转向使用倾斜量及偏航角速度，适合飞行操控，不依赖磁北航向

IMU 默认按 IIO `name=bmi270` 自动发现，测试时可通过 `FLIGHT_CLUB_IMU` 指定包含同名通道的目录。采样失效或超过 150 ms 未更新时显示 `FAIL` 并保留键盘操作；`--imu off` 可用于关闭体感的显示基准

项目包含原页面的战机美术数据，来源保留上述记录，项目未另行授予这些素材的许可
