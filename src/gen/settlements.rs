use crate::config::Config;
use crate::names;
use crate::world::{Biome, Settlement, World};
use rand::rngs::StdRng;
use rand::RngExt;
use std::ops::Range;

fn allowed(biome: Biome) -> bool {
    matches!(
        biome,
        Biome::Coast | Biome::Grassland | Biome::Steppe | Biome::Forest | Biome::Taiga
    )
}

pub fn place(w: &mut World, cfg: &Config, rng: &mut StdRng) {
    let n_big = ((cfg.settlement_density * 4.0).round() as usize).max(1);
    let n_small = (cfg.settlement_density * 16.0).round() as usize;
    place_range(w, rng, n_big, 2000..50000, true, 10.0);
    place_range(w, rng, n_small, 40..1500, false, 5.0);
}

fn place_range(
    w: &mut World,
    rng: &mut StdRng,
    count: usize,
    pop_range: Range<u32>,
    prefer_water: bool,
    min_dist: f32,
) {
    let mut points = w.settlements();
    let mut placed = 0usize;
    let mut attempts = 0usize;
    let max_attempts = count * 300 + 100;
    while placed < count && attempts < max_attempts {
        attempts += 1;
        let x = rng.random_range(0..w.width);
        let y = rng.random_range(0..w.height);
        let t = w.get(x, y).unwrap();
        if t.settlement.is_some() || t.poi.is_some() || t.road || t.is_water() {
            continue;
        }
        if !allowed(t.biome) {
            continue;
        }
        if points.iter().any(|s| World::dist(*s, (x, y)) < min_dist) {
            continue;
        }
        let near_water = w.tiles_in_radius(x, y, 6).iter().any(|n| {
            let nt = w.get(n.0, n.1).unwrap();
            nt.river || nt.is_water()
        });
        if prefer_water && !near_water && attempts % 2 == 0 {
            continue;
        }
        let population = rng.random_range(pop_range.clone());
        let name = names::settlement(rng, population, w.near_ocean(x, y));
        let t = w.get_mut(x, y).unwrap();
        t.settlement = Some(Settlement { name, population });
        points.push((x, y));
        placed += 1;
    }
}
