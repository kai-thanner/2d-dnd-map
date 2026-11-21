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
    // Strikt sequenzielle Phasen (single-threaded, keine Race-Conditions —
    // jede Phase ist komplett fertig, bevor die nächste startet):
    // 1. Karte (Heightmap, Klima, Feuchtigkeit)
    heightmap::generate(&mut w, cfg);
    moisture::generate(&mut w, cfg);
    climate::assign(&mut w, cfg);
    // 2. Flüsse & Seen (iterativ: Fluss -> Senke -> See -> Überlauf -> neuer
    //    Fluss ... bis jeder Fluss im Meer endet)
    rivers::carve(&mut w, cfg);
    lakes::fill(&mut w, cfg.sea_level, cfg.climate.snow_temp);
    // 2b. Biome/Nochmal-Höhen ableiten, Flussbreiten, Täler
    biomes::assign(&mut w, cfg);
    rivers::recompute_river_widths(&mut w);
    rivers::carve_valleys(&mut w, cfg.sea_level);
    // 3. von Lebewesen Gegebautes: Siedlungen & Straßen
    settlements::place(&mut w, cfg, &mut rng);
    roads::connect(&mut w);
    // 4. POIs: Dungeons, Gasthäuser, Händler
    pois::place(&mut w, cfg, &mut rng);
    w
}
