use crate::config::Config;
use crate::world::{Biome, World};

pub fn assign(w: &mut World, cfg: &Config) {
    for y in 0..w.height {
        for x in 0..w.width {
            let t = w.get(x, y).unwrap();
            if t.elevation < cfg.sea_level {
                continue;
            }
            let h = t.elevation - cfg.sea_level;
            let biome = if t.temperature <= 0.0 {
                Biome::Snow
            } else if t.river {
                Biome::River
            } else if h >= 0.34 {
                Biome::Mountain
            } else if h >= 0.22 {
                Biome::Rock
            } else {
                let near_water = w
                    .neighbors8(x, y)
                    .iter()
                    .any(|n| w.get(n.0, n.1).unwrap().elevation < cfg.sea_level);
                if near_water && h < 0.03 {
                    Biome::Coast
                } else {
                    land_biome(t.temperature, t.moisture)
                }
            };
            w.get_mut(x, y).unwrap().biome = biome;
        }
    }
}

fn land_biome(temp: f32, moisture: f32) -> Biome {
    if temp <= 0.0 {
        Biome::Snow
    } else if temp < 8.0 {
        if moisture >= 0.35 {
            Biome::Taiga
        } else {
            Biome::Tundra
        }
    } else if temp < 18.0 {
        if moisture >= 0.6 {
            Biome::Forest
        } else if moisture >= 0.35 {
            Biome::Grassland
        } else {
            Biome::Steppe
        }
    } else if moisture >= 0.65 {
        Biome::Jungle
    } else if moisture >= 0.4 {
        Biome::Grassland
    } else if moisture >= 0.2 {
        Biome::Steppe
    } else {
        Biome::Desert
    }
}
