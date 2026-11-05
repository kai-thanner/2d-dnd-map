use crate::config::Config;
use crate::noise::lerp;
use crate::world::World;

pub fn assign(w: &mut World, cfg: &Config) {
    for y in 0..w.height {
        let lat = y as f32 / w.height as f32;
        for x in 0..w.width {
            let t = w.get_mut(x, y).unwrap();
            let mut temp = lerp(cfg.climate.north_temp, cfg.climate.south_temp, lat);
            temp -= (t.elevation - cfg.sea_level).max(0.0) * cfg.climate.lapse_per_unit;
            t.temperature = temp;
        }
    }
}
