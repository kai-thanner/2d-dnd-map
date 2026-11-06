use crate::config::Config;
use crate::world::{Biome, WaterKind, World};

pub fn assign(w: &mut World, cfg: &Config) {
    for y in 0..w.height {
        for x in 0..w.width {
            let t = w.get(x, y).unwrap();
            if t.elevation < cfg.sea_level {
                continue;
            }
            let h = t.elevation - cfg.sea_level;
            let temp = t.temperature;
            let river = t.river;
            let h_m = h * cfg.max_elevation_m;
            let lat = y as f32 / w.height as f32;
            let treeline_eff = cfg.treeline_m * (0.8 + 0.5 * lat);
            let land = if temp <= cfg.climate.snow_temp {
                Biome::Snow
            } else if h_m > treeline_eff {
                if h >= 0.34 {
                    Biome::Mountain
                } else if h_m > treeline_eff + 800.0 {
                    Biome::Rock
                } else if t.moisture >= 0.35 {
                    Biome::Grassland
                } else {
                    Biome::Steppe
                }
            } else {
                let near_ocean = w
                    .neighbors8(x, y)
                    .iter()
                    .any(|n| w.get(n.0, n.1).unwrap().water == Some(WaterKind::Ocean));
                if near_ocean && h < 0.03 {
                    Biome::Coast
                } else {
                    land_biome(temp, t.moisture, cfg.climate.snow_temp)
                }
            };
            let t = w.get_mut(x, y).unwrap();
            t.biome = land;
            if river {
                t.water = Some(if temp <= cfg.climate.snow_temp {
                    WaterKind::Ice
                } else {
                    WaterKind::River
                });
            }
        }
    }
}

fn land_biome(temp: f32, moisture: f32, snow_temp: f32) -> Biome {
    if temp <= snow_temp {
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
