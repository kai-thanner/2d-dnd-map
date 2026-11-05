pub mod biomes;
pub mod climate;
pub mod heightmap;
pub mod lakes;
pub mod moisture;
pub mod pois;
pub mod rivers;
pub mod roads;
pub mod settlements;

use crate::config::Config;
use crate::rng;
use crate::world::World;

pub fn generate(cfg: &Config) -> World {
    let mut rng = rng::from_seed(cfg.seed);
    let mut w = World::new(cfg.width, cfg.height);
    w.sea_level = cfg.sea_level;
    heightmap::generate(&mut w, cfg);
    moisture::generate(&mut w, cfg);
    climate::assign(&mut w, cfg);
    rivers::carve(&mut w, cfg);
    lakes::fill(&mut w, cfg);
    biomes::assign(&mut w, cfg);
    settlements::place(&mut w, cfg, &mut rng);
    roads::connect(&mut w);
    pois::place(&mut w, cfg, &mut rng);
    rivers::recompute_river_widths(&mut w);
    w
}
