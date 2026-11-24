//! Syllable-based names for stars, planets, flora and minerals. Uppercase ASCII so the 8×8 font
//! shows them; at most 12 characters so they fit the HUD.
use super::rng::Rng;

const START: [&str; 32] = [
    "AL", "BE", "CA", "DRA", "E", "FO", "GA", "HE", "I", "JU", "KA", "LO", "MI", "NE", "O", "PRA",
    "QUA", "RHO", "SE", "TA", "U", "VE", "WA", "XI", "YO", "ZE", "THE", "SKA", "VOR", "KRY", "OR",
    "AE",
];
const MIDDLE: [&str; 24] = [
    "LA", "RI", "NO", "VE", "TA", "SI", "MO", "KE", "DU", "RA", "LI", "NA", "ZO", "PE", "XA", "TU",
    "BI", "GO", "RE", "SA", "VI", "LU", "ME", "DA",
];
const END: [&str; 24] = [
    "N", "S", "X", "RA", "US", "ON", "IA", "IS", "AR", "EL", "OS", "UM", "ETH", "ION", "ARA",
    "ITE", "ORE", "ANT", "IX", "YN", "OR", "A", "E", "O",
];
const SUFFIX: [&str; 8] = [
    " PRIME", " MINOR", " MAJOR", " II", " IV", " V", " VII", " IX",
];

/// A pronounceable name. `kind` separates streams so a star and its first planet differ.
pub fn name(seed: u64, kind: u64) -> String {
    let mut rng = Rng::new(seed ^ kind.wrapping_mul(0x2545_f491_4f6c_dd1d));
    let mut out = String::with_capacity(14);
    out.push_str(START[rng.below(START.len() as u32) as usize]);
    for _ in 0..rng.below(2) {
        out.push_str(MIDDLE[rng.below(MIDDLE.len() as u32) as usize]);
    }
    out.push_str(END[rng.below(END.len() as u32) as usize]);
    if out.len() < 3 {
        out.push_str(MIDDLE[rng.below(MIDDLE.len() as u32) as usize]);
    }
    if rng.below(5) == 0 && out.len() <= 6 {
        out.push_str(SUFFIX[rng.below(SUFFIX.len() as u32) as usize]);
    }
    out.truncate(12);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_are_stable_short_and_varied() {
        let mut seen = std::collections::HashSet::new();
        for seed in 0..500 {
            let n = name(seed, 1);
            assert_eq!(n, name(seed, 1));
            assert!((3..=12).contains(&n.len()), "{n}");
            assert!(n.chars().all(|c| c.is_ascii_uppercase() || c == ' '));
            seen.insert(n);
        }
        assert!(seen.len() > 400);
    }
}
