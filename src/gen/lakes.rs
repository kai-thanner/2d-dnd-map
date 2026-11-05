use crate::config::Config;
use crate::world::{Biome, World};

pub fn fill(w: &mut World, cfg: &Config) {
    let mut ocean = vec![false; w.tiles.len()];
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for x in 0..w.width {
        stack.push((x, 0));
        stack.push((x, w.height - 1));
    }
    for y in 0..w.height {
        stack.push((0, y));
        stack.push((w.width - 1, y));
    }
    while let Some((x, y)) = stack.pop() {
        let i = w.idx(x, y);
        if ocean[i] || w.tiles[i].elevation >= cfg.sea_level {
            continue;
        }
        ocean[i] = true;
        for n in w.neighbors4(x, y) {
            stack.push(n);
        }
    }
    for y in 0..w.height {
        for x in 0..w.width {
            let i = w.idx(x, y);
            let is_ocean = ocean[i];
            let t = w.get_mut(x, y).unwrap();
            if t.elevation < cfg.sea_level {
                t.biome = if t.temperature <= 0.0 {
                    Biome::Ice
                } else if t.river {
                    Biome::River
                } else if is_ocean {
                    Biome::Ocean
                } else {
                    Biome::Lake
                };
            }
        }
    }
}
