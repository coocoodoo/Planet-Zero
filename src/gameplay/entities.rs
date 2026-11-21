use super::Weapon;
use glam::Vec3;
#[derive(Clone)]
pub struct Gate {
    pub position: Vec3,
    pub normal: Vec3,
    pub radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickupKind {
    Coin,
    Upgrade(Weapon),
    Energy,
    Shield,
    Bomb,
    Support,
    Time,
}

#[derive(Clone)]
pub struct Pickup {
    pub position: Vec3,
    pub kind: PickupKind,
    pub life: f32,
    pub collected: bool,
}

/// 敌机种类；判别值参与生成时的随机偏移，顺序不可改
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EnemyKind {
    /// 轻型机
    Light,
    /// 截击机
    Interceptor,
    /// 重型机
    Heavy,
    Dreadnought,
    Serpent,
    Tempest,
    /// 地面防空炮台
    Battery,
    /// 修复附近敌机的维修无人机
    Jammer,
    /// 高速精英截击机
    Striker,
}

/// 单种敌机的全部参数
pub struct EnemySpec {
    pub health: f32,
    pub radius: f32,
    /// 机动相位的推进速率
    pub phase_rate: f32,
    /// 绕玩家横向游弋的幅度
    pub lateral: f32,
    /// 向目标位置收敛的速率
    pub agility: f32,
    /// 普通射击间隔（Boss 另按狂暴状态计算）
    pub cooldown: f32,
    /// 每轮齐射的弹数
    pub burst: u8,
    pub score: u32,
    /// 渲染缩放与机身颜色
    pub scale: f32,
    pub color: [u8; 3],
}

#[allow(clippy::too_many_arguments)]
const fn spec(
    health: f32,
    radius: f32,
    phase_rate: f32,
    lateral: f32,
    agility: f32,
    cooldown: f32,
    burst: u8,
    score: u32,
    scale: f32,
    color: [u8; 3],
) -> EnemySpec {
    EnemySpec {
        health,
        radius,
        phase_rate,
        lateral,
        agility,
        cooldown,
        burst,
        score,
        scale,
        color,
    }
}

const BOSS_HEALTH: f32 = 2400.0;
#[rustfmt::skip]
const ENEMY_SPECS: [EnemySpec; 9] = [
    //   health       radius phase lateral agility cooldown burst score  scale color
    spec(42.0,        8.0,   0.55, 90.0,   1.8,    3.2,     1,    300,   0.75, [239, 98, 112]),
    spec(64.0,        10.0,  0.67, 90.0,   1.8,    2.9,     2,    450,   0.93, [171, 119, 235]),
    spec(95.0,        12.0,  0.79, 90.0,   1.8,    2.6,     3,    600,   1.11, [209, 161, 101]),
    spec(BOSS_HEALTH, 24.0,  0.55, 60.0,   1.2,    1.3,     7,    10000, 3.2,  [113, 133, 172]),
    spec(BOSS_HEALTH, 24.0,  0.67, 115.0,  1.2,    1.3,     7,    10000, 3.2,  [141, 208, 123]),
    spec(BOSS_HEALTH, 24.0,  0.79, 75.0,   1.2,    1.3,     7,    10000, 3.2,  [189, 135, 215]),
    spec(180.0,       14.0,  0.55, 90.0,   0.0,    3.5,     3,    1200,  1.83, [255, 172, 76]),
    spec(115.0,       8.0,   0.67, 90.0,   1.8,    4.0,     2,    1350,  0.7,  [120, 221, 193]),
    spec(140.0,       12.0,  1.5,  90.0,   2.8,    2.3,     4,    1500,  1.2,  [255, 172, 76]),
];

impl EnemyKind {
    pub const BOSSES: [Self; 3] = [Self::Dreadnought, Self::Serpent, Self::Tempest];
    pub const FIGHTERS: [Self; 3] = [Self::Light, Self::Interceptor, Self::Heavy];
    pub fn spec(self) -> &'static EnemySpec {
        &ENEMY_SPECS[self as usize]
    }
    pub fn is_boss(self) -> bool {
        Self::BOSSES.contains(&self)
    }
}

#[derive(Clone)]
pub struct Enemy {
    pub id: u32,
    pub position: Vec3,
    pub yaw: f32,
    pub health: f32,
    pub max_health: f32,
    pub kind: EnemyKind,
    pub phase: f32,
    pub cooldown: f32,
    pub hit: f32,
    pub aim_point: Vec3,
}

impl Enemy {
    pub fn is_boss(&self) -> bool {
        self.kind.is_boss()
    }
    pub fn is_ground(&self) -> bool {
        self.kind == EnemyKind::Battery
    }
    pub fn radius(&self) -> f32 {
        self.kind.spec().radius
    }
}

#[derive(Clone)]
pub struct Shot {
    pub position: Vec3,
    pub previous: Vec3,
    pub velocity: Vec3,
    pub life: f32,
    pub damage: f32,
    pub weapon: Weapon,
    pub hostile: bool,
    pub grazed: bool,
    pub target: Option<u32>,
    pub radius: f32,
}

#[derive(Clone)]
pub struct Effect {
    pub position: Vec3,
    pub life: f32,
    pub duration: f32,
    pub radius: f32,
    pub color: [u8; 3],
}

#[derive(Clone)]
pub struct Convoy {
    pub position: Vec3,
    pub previous: Vec3,
    pub health: f32,
    pub max_health: f32,
    pub cooldown: f32,
}
