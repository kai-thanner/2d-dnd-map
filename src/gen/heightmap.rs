use crate::config::Config;
use crate::noise::Noise;
use crate::world::World;

const NOISE_TILES_PER_UNIT: f32 = 25.0;

pub fn generate(w: &mut World, cfg: &Config) {
    let noise = Noise::new(cfg.seed ^ 0xA53A_9D1C);
    let ridge = Noise::new(cfg.seed ^ 0xB16E_5EED);
    let cx = w.width as f32 / 2.0;
    let cy = w.height as f32 / 2.0;
    let max_d = (cx * cx + cy * cy).sqrt();
    for y in 0..w.height {
        for x in 0..w.width {
            let nx = x as f32 / NOISE_TILES_PER_UNIT;
            let ny = y as f32 / NOISE_TILES_PER_UNIT;
            let wx = nx + noise.fbm(nx + 9.2, ny + 3.1, 3) * 0.7;
            let wy = ny + noise.fbm(nx + 4.7, ny + 8.9, 3) * 0.7;
            let base = noise.fbm(wx, wy, 5);
            let mut r = 0.0f32;
            let mut amp = 0.55f32;
            let mut freq = 1.0f32;
            let mut norm = 0.0f32;
            for _ in 0..4 {
                let v = ridge.fbm(wx * 2.2 * freq, wy * 2.2 * freq, 2);
                r += (1.0 - (2.0 * v - 1.0).abs()).powi(2) * amp;
                norm += amp;
                amp *= 0.5;
                freq *= 2.1;
            }
            let ridge = r / norm;
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt() / max_d;
            let mut e = (base * 0.78 + ridge * 0.48 - d * 0.34).clamp(0.0, 1.0);
            e = (e * e * (3.0 - 2.0 * e)).clamp(0.0, 1.0);
            w.get_mut(x, y).unwrap().elevation = e;
        }
    }
}
