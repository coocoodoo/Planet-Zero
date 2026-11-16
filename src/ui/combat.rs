use super::*;
pub(super) fn combat_hud(
    c: &mut Canvas<'_>,
    m: &crate::gameplay::Mission,
    f: &crate::flight::Flight,
    input: &str,
) {
    use crate::gameplay::progression::Phase;
    c.shade(0, 0, 29, 170);
    c.shade(291, 0, 29, 170);
    c.small(2, 3, &format!("{:07}", m.score), GOLD);
    c.icon(
        3,
        17,
        'S',
        if m.health / m.max_health < 0.3 {
            RED
        } else {
            CYAN
        },
    );
    c.small(14, 18, &format!("{:03.0}", m.health), WHITE);
    c.bar(3, 27, 23, m.health / m.max_health, CYAN);
    c.icon(3, 37, 'A', [103, 196, 255]);
    c.small(14, 38, &format!("{:03.0}", m.energy), WHITE);
    c.bar(3, 47, 23, m.energy / 100.0, [103, 196, 255]);
    let letter = match m.weapon {
        crate::gameplay::Weapon::Gun => "G",
        crate::gameplay::Weapon::Missile => "M",
        crate::gameplay::Weapon::Laser => "L",
        crate::gameplay::Weapon::Plasma => "P",
        crate::gameplay::Weapon::Orb => "O",
    };
    c.text(3, 57, letter, m.weapon.color());
    c.small(15, 59, &format!("{}", m.levels[m.weapon as usize]), WHITE);
    for i in 0..m.levels[m.weapon as usize] {
        c.rect(3 + i as i32 * 8, 69, 6, 2, m.weapon.color());
    }
    c.icon(3, 80, 'B', GOLD);
    c.small(17, 81, &m.bombs.to_string(), WHITE);
    c.icon(
        3,
        96,
        'Y',
        if m.overdrive >= 100.0 || m.overdrive_time > 0.0 {
            GOLD
        } else {
            MUTED
        },
    );
    c.small(
        14,
        97,
        &format!(
            "{:03.0}",
            if m.overdrive_time > 0.0 {
                m.overdrive_time
            } else {
                m.overdrive
            }
        ),
        WHITE,
    );
    c.bar(
        3,
        107,
        23,
        if m.overdrive_time > 0.0 {
            m.overdrive_time / 8.0
        } else {
            m.overdrive / 100.0
        },
        GOLD,
    );
    c.icon(3, 119, 'W', if m.support > 0.0 { CYAN } else { MUTED });
    c.small(16, 120, &format!("{:02.0}", m.support), WHITE);
    for (i, notice) in m.notices.iter().flatten().enumerate() {
        let color = if notice.kind == crate::gameplay::progression::NoticeKind::Hit {
            RED
        } else {
            GOLD
        };
        c.icon(3 + i as i32 * 9, 139, notice.kind.icon(), color);
    }
    c.text(3, 159, if m.autofire { "A" } else { "M" }, CYAN);
    c.text(
        17,
        159,
        match input {
            "IMU" => "I",
            "CAL" => "K",
            "FAIL" => "!",
            _ => "",
        },
        if input == "FAIL" { RED } else { MUTED },
    );
    c.icon(294, 3, 'O', MUTED);
    c.small(305, 4, &format!("{:03.0}", m.remaining), WHITE);
    c.icon(294, 19, 'F', GOLD);
    c.small(306, 20, &format!("{:02}", m.level), WHITE);
    for i in 0..3 {
        c.rect(
            294 + i * 8,
            32,
            6,
            3,
            if i as usize <= m.phase_index {
                CYAN
            } else {
                MUTED
            },
        );
    }
    c.icon(294, 44, m.phase().icon(), CYAN);
    c.small(306, 45, &format!("{:02}", m.wave), WHITE);
    if let Some(boss) = m.enemies.iter().find(|e| e.is_boss()) {
        c.icon(
            299,
            60,
            'X',
            if boss.health < boss.max_health * 0.5 {
                RED
            } else {
                GOLD
            },
        );
        c.rect(315, 61, 3, 48, [39, 43, 61]);
        let h = (boss.health / boss.max_health * 48.0) as i32;
        c.rect(315, 109 - h, 3, h, RED);
        c.small(
            295,
            75,
            &format!("{:03.0}", boss.health / boss.max_health * 100.0),
            WHITE,
        );
    } else {
        let progress = if m.phase() == Phase::Patrol || m.phase() == Phase::Strike {
            m.objective as f32 / m.objective_goal().max(1) as f32
        } else {
            m.phase_elapsed / m.phase_duration().max(1.0)
        };
        c.bar(294, 60, 22, progress, CYAN);
        if m.objective_goal() > 0 {
            c.small(
                294,
                69,
                &format!("{:02}/{:02}", m.objective, m.objective_goal()),
                WHITE,
            );
        } else {
            c.small(
                297,
                69,
                &format!("{:03.0}", (m.phase_duration() - m.phase_elapsed).max(0.0)),
                WHITE,
            );
        }
        if let Some(convoy) = &m.convoy {
            c.icon(295, 82, 'C', CYAN);
            c.small(
                306,
                84,
                &format!("{:02.0}", convoy.health / convoy.max_health * 99.0),
                WHITE,
            );
            c.bar(294, 94, 22, convoy.health / convoy.max_health, CYAN);
        }
    }
    let cx = 304;
    let cy = 131;
    for i in 0..24 {
        let a = i as f32 * std::f32::consts::TAU / 24.0;
        c.pixel(
            cx + (a.sin() * 12.0) as i32,
            cy + (a.cos() * 12.0) as i32,
            MUTED,
        );
    }
    c.line(cx - 2, cy + 2, cx, cy - 3, CYAN);
    c.line(cx, cy - 3, cx + 2, cy + 2, CYAN);
    for enemy in &m.enemies {
        let delta = enemy.position - f.position;
        let a = angle(delta.x.atan2(-delta.y) - f.yaw);
        let r = (delta.length() / 30.0).min(11.0);
        c.pixel(cx + (a.sin() * r) as i32, cy - (a.cos() * r) as i32, RED);
    }
    if let Some(target) = m.target(f) {
        let delta = target - f.position;
        let a = angle(delta.x.atan2(-delta.y) - f.yaw);
        let x = cx + (a.sin() * 10.0) as i32;
        let y = cy - (a.cos() * 10.0) as i32;
        c.rect(x - 1, y - 1, 2, 2, GOLD);
        let dy = if delta.z > 15.0 {
            -3
        } else if delta.z < -15.0 {
            3
        } else {
            0
        };
        c.line(297, 151, 303, 151 + dy, CYAN);
        c.line(303, 151 + dy, 309, 151, CYAN);
    }
    c.text(301, 159, "U", MUTED);
    if m.flash > 0.0 {
        c.rect(0, 0, 2, 170, CYAN);
        c.rect(318, 0, 2, 170, CYAN);
    }
    if m.countdown > 0.0 {
        c.shade(43, 43, 234, 69);
        let op = crate::gameplay::progression::operation(m.level);
        c.text(56, 48, op.name.as_str(), GOLD);
        c.text(56, 62, op.phases[0].name(), CYAN);
        c.text(56, 75, op.phases[1].name(), CYAN);
        c.text(56, 88, "THEN BOSS INTERCEPT", MUTED);
        c.text_scale(242, 78, &format!("{}", m.countdown.ceil() as u32), GOLD, 2);
    }
}
