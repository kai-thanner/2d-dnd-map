use crate::config::Config;
use crate::names;
use crate::world::{Biome, Settlement, WaterKind, World};
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
    ensure_harbors(w);
}

fn ensure_harbors(w: &mut World) {
    let sea = w.sea_level;
    let mut visited = vec![false; w.tiles.len()];
    for y in 0..w.height {
        for x in 0..w.width {
            let i = w.idx(x, y);
            if visited[i] || w.tiles[i].elevation < sea {
                continue;
            }
            let mut comp: Vec<(usize, usize)> = Vec::new();
            let mut stack = vec![(x, y)];
            visited[i] = true;
            while let Some((cx, cy)) = stack.pop() {
                comp.push((cx, cy));
                for n in w.neighbors4(cx, cy) {
                    let ni = w.idx(n.0, n.1);
                    if !visited[ni] && w.tiles[ni].elevation >= sea {
                        visited[ni] = true;
                        stack.push(n);
                    }
                }
            }
            let setts: Vec<(usize, usize)> = comp
                .iter()
                .filter(|(cx, cy)| w.get(*cx, *cy).unwrap().settlement.is_some())
                .cloned()
                .collect();
            if setts.is_empty() {
                continue;
            }
            let total_pop: u64 = setts
                .iter()
                .map(|s| w.get(s.0, s.1).unwrap().settlement.as_ref().unwrap().population as u64)
                .sum();
            if total_pop < 2000 {
                continue;
            }
            let n_harbors = ((total_pop / 25_000).max(1) as usize).min(setts.len());
            let mut coastal = setts.clone();
            coastal.sort_by_key(|s| {
                let ocean_n = w
                    .neighbors8(s.0, s.1)
                    .iter()
                    .filter(|n| {
                        matches!(
                            w.get(n.0, n.1).unwrap().water,
                            Some(WaterKind::Ocean) | Some(WaterKind::Lake)
                        )
                    })
                    .count();
                std::cmp::Reverse(ocean_n)
            });
            for s in coastal.iter().take(n_harbors) {
                if !w.near_port_water(s.0, s.1) {
                    continue;
                }
                let t = w.get_mut(s.0, s.1).unwrap();
                if let Some(st) = &mut t.settlement {
                    st.has_port = true;
                    if st.population <= 2000 {
                        st.population = 2500;
                    }
                }
            }
        }
    }
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
        let has_port = w.near_port_water(x, y);
        let t = w.get_mut(x, y).unwrap();
        t.settlement = Some(Settlement {
            name,
            population,
            has_port,
        });
        points.push((x, y));
        placed += 1;
    }
}
