//! Procedural lore fragments read at monoliths and ruins: three short lines built from phrase
//! tables and generated names, always the same for the same object.
use super::names;
use super::rng::Rng;

const OPEN: [&str; 10] = [
    "THE {} BUILT THIS PLACE",
    "WE CAME HERE FROM {}",
    "{} WATCHED THIS SKY",
    "HERE THE {} SLEPT",
    "THIS STONE REMEMBERS {}",
    "THE LAST OF {} STOOD HERE",
    "{} SPOKE TO THE STARS",
    "IN THE AGE OF {}",
    "THE ROAD TO {} BEGINS",
    "BEFORE {} THERE WAS DUST",
];
const MIDDLE: [&str; 10] = [
    "THE DRONES STILL GUARD IT",
    "OUR SEEDS WAIT UNDER ICE",
    "THE STAR GREW RED AND COLD",
    "WE TAUGHT THE MACHINES TO KEEP",
    "THE SKY OPENED LIKE A DOOR",
    "WE FOLLOWED THE WARP SONG",
    "THE CRYSTALS HOLD OUR VOICES",
    "THE STATIONS NEVER SLEEP",
    "WE TRADED IRON FOR LIGHT",
    "ONE PATH LEADS TO THE CORE",
];
const CLOSE: [&str; 10] = [
    "FOLLOW THE BEACONS",
    "NOTHING IS LOST ONLY HIDDEN",
    "LOOK FOR THE GREEN STAR",
    "DO NOT WAKE THE HIVE",
    "THE GATE IS {} AWAY",
    "TAKE ONLY WHAT YOU NEED",
    "WE WILL MEET IN {}",
    "THE SIGNAL REPEATS",
    "COUNT THE MOONS",
    "REMEMBER {}",
];

fn fill(template: &str, name: &str) -> String {
    let mut line = template.replace("{}", name);
    line.truncate(34);
    line
}

/// Three lines of at most 34 characters.
pub fn fragment(seed: u64) -> [String; 3] {
    let mut rng = Rng::new(seed ^ 0x10e);
    let name = names::name(seed, 91);
    let place = names::name(seed, 92);
    [
        fill(OPEN[rng.below(10) as usize], &name),
        fill(MIDDLE[rng.below(10) as usize], &place),
        fill(CLOSE[rng.below(10) as usize], &place),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fragments_are_stable_and_fit_the_screen() {
        let mut seen = std::collections::HashSet::new();
        for seed in 0..300 {
            let lines = fragment(seed);
            assert_eq!(lines, fragment(seed));
            for line in &lines {
                assert!(!line.is_empty() && line.len() <= 34 && !line.contains("{}"));
            }
            seen.insert(lines);
        }
        assert!(seen.len() > 250);
    }
}
