use crate::config::Config;
use crate::names;
use crate::world::{Biome, Poi, World};
use rand::rngs::StdRng;
use rand::RngExt;

pub fn place(w: &mut World, cfg: &Config, rng: &mut StdRng) {
    let settlements = w.settlements();
    let n_dungeons = ((cfg.settlement_density * 6.0).round() as usize).max(2);
    let mut placed = 0usize;
    let mut attempts = 0usize;
    while placed < n_dungeons && attempts < n_dungeons * 300 {
        attempts += 1;
        let x = rng.random_range(0..w.width);
        let y = rng.random_range(0..w.height);
        let t = w.get(x, y).unwrap();
        if t.poi.is_some() || t.settlement.is_some() || t.road {
            continue;
        }
        if !matches!(
            t.biome,
            Biome::Mountain | Biome::Rock | Biome::Forest | Biome::Taiga
        ) {
            continue;
        }
        if settlements.iter().any(|s| World::dist(*s, (x, y)) < 8.0) {
            continue;
        }
        let name = names::dungeon(rng);
        w.get_mut(x, y).unwrap().poi = Some(Poi::Dungeon { name });
        placed += 1;
    }
    let n_inns = (settlements.len() + 2).min(8);
    let mut placed = 0usize;
    let mut attempts = 0usize;
    while placed < n_inns && attempts < n_inns * 500 {
        attempts += 1;
        let x = rng.random_range(0..w.width);
        let y = rng.random_range(0..w.height);
        let t = w.get(x, y).unwrap();
        if t.poi.is_some() || t.settlement.is_some() || !t.is_passable() {
            continue;
        }
        if !w.neighbors8(x, y).iter().any(|n| w.get(n.0, n.1).unwrap().road) {
            continue;
        }
        let d_settlement = settlements
            .iter()
            .map(|s| World::dist(*s, (x, y)))
            .fold(f32::INFINITY, f32::min);
        if d_settlement < 2.0 || d_settlement > 14.0 {
            continue;
        }
        let name = names::inn(rng);
        w.get_mut(x, y).unwrap().poi = Some(Poi::Inn { name });
        placed += 1;
    }
    for (x, y) in &settlements {
        if let Some(t) = w.get_mut(*x, *y) {
            if t.poi.is_none() {
                let (mut name, goods) = names::merchant(rng);
                if rng.random_range(0..100) < 15 {
                    name = names::famous_merchant(rng);
                }
                t.poi = Some(Poi::Merchant { name, goods });
            }
        }
    }
}
