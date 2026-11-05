pub struct Noise {
    seed: u64,
}

impl Noise {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    fn hash(&self, x: i64, y: i64) -> u64 {
        let mut z = self
            .seed
            .wrapping_add((x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
            .wrapping_add((y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F));
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn lattice(&self, x: i64, y: i64) -> f32 {
        let bits = self.hash(x, y) >> 11;
        (bits as f64 / 9_007_199_254_740_992.0) as f32
    }

    fn smooth(t: f32) -> f32 {
        t * t * (3.0 - 2.0 * t)
    }

    pub fn value(&self, x: f32, y: f32) -> f32 {
        let x0 = x.floor() as i64;
        let y0 = y.floor() as i64;
        let fx = x - x0 as f32;
        let fy = y - y0 as f32;
        let v00 = self.lattice(x0, y0);
        let v10 = self.lattice(x0 + 1, y0);
        let v01 = self.lattice(x0, y0 + 1);
        let v11 = self.lattice(x0 + 1, y0 + 1);
        let sx = Self::smooth(fx);
        let sy = Self::smooth(fy);
        lerp(lerp(v00, v10, sx), lerp(v01, v11, sx), sy)
    }

    pub fn fbm(&self, x: f32, y: f32, octaves: u32) -> f32 {
        let mut sum = 0.0f32;
        let mut amp = 1.0f32;
        let mut freq = 1.0f32;
        let mut norm = 0.0f32;
        for _ in 0..octaves {
            sum += self.value(x * freq, y * freq) * amp;
            norm += amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        sum / norm
    }
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
