use crate::config::Config;
use crate::noise::Noise;
use crate::world::World;

pub fn generate(w: &mut World, cfg: &Config) {
    let noise = Noise::new(cfg.seed ^ 0xA015_7EA5);
    let scale = 3.0;
    for y in 0..w.height {
        let lat = y as f32 / w.height as f32;
        let factor = 1.0 - cfg.dry_south * lat;
        for x in 0..w.width {
            let nx = x as f32 / w.width as f32 * scale + 31.7;
            let ny = y as f32 / w.height as f32 * scale + 11.3;
            let m = (noise.fbm(nx, ny, 4) * cfg.moisture_scale * factor).clamp(0.0, 1.0);
            w.get_mut(x, y).unwrap().moisture = m;
        }
    }
}
