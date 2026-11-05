use crate::config::Config;
use crate::noise::Noise;
use crate::world::World;

pub fn generate(w: &mut World, cfg: &Config) {
    let noise = Noise::new(cfg.seed ^ 0xA53A_9D1C);
    let scale = 4.5;
    let cx = w.width as f32 / 2.0;
    let cy = w.height as f32 / 2.0;
    let max_d = (cx * cx + cy * cy).sqrt();
    for y in 0..w.height {
        for x in 0..w.width {
            let nx = x as f32 / w.width as f32 * scale;
            let ny = y as f32 / w.height as f32 * scale;
            let mut e = noise.fbm(nx, ny, 5);
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt() / max_d;
            e = (e * 1.15 - d * 0.45).clamp(0.0, 1.0);
            w.get_mut(x, y).unwrap().elevation = e;
        }
    }
}
