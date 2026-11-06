use crate::world::{Biome, World};

pub fn biome_rgb(b: Biome) -> (u8, u8, u8) {
    match b {
        Biome::Ocean => (30, 80, 170),
        Biome::Lake => (30, 80, 170),
        Biome::River => (30, 80, 170),
        Biome::Ice => (214, 232, 248),
        Biome::Coast => (226, 214, 162),
        Biome::Desert => (228, 198, 116),
        Biome::Steppe => (176, 198, 104),
        Biome::Grassland => (104, 172, 84),
        Biome::Forest => (42, 118, 60),
        Biome::Jungle => (24, 98, 52),
        Biome::Taiga => (64, 110, 88),
        Biome::Tundra => (178, 188, 178),
        Biome::Snow => (244, 246, 250),
        Biome::Rock => (128, 128, 132),
        Biome::Mountain => (88, 86, 96),
    }
}

pub fn cell_at(px: f32, py: f32, cell: f32, width: usize, height: usize) -> Option<(usize, usize)> {
    if cell <= 0.0 {
        return None;
    }
    let x = (px / cell).floor() as i64;
    let y = (py / cell).floor() as i64;
    if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
        Some((x as usize, y as usize))
    } else {
        None
    }
}

pub fn hillshade(w: &World, x: usize, y: usize) -> f32 {
    let e = w.get(x, y).unwrap().elevation;
    let el = if x > 0 { w.get(x - 1, y).unwrap().elevation } else { e };
    let er = if x + 1 < w.width { w.get(x + 1, y).unwrap().elevation } else { e };
    let eu = if y > 0 { w.get(x, y - 1).unwrap().elevation } else { e };
    let ed = if y + 1 < w.height { w.get(x, y + 1).unwrap().elevation } else { e };
    let dzdx = (er - el) / 2.0;
    let dzdy = (ed - eu) / 2.0;
    let lx = -0.55f32;
    let ly = -0.55;
    let lz = 0.75;
    let ll = (lx * lx + ly * ly + lz * lz).sqrt();
    let nl = (1.0 + dzdx * dzdx + dzdy * dzdy).sqrt();
    let dot = (-dzdx * lx / ll - dzdy * ly / ll + lz / ll) / nl;
    (0.55 + 0.5 * dot.clamp(0.0, 1.0)).clamp(0.35, 1.15)
}

pub fn shade_rgb(w: &World, x: usize, y: usize, rgb: (u8, u8, u8)) -> (u8, u8, u8) {
    let s = hillshade(w, x, y);
    (
        (rgb.0 as f32 * s).min(255.0) as u8,
        (rgb.1 as f32 * s).min(255.0) as u8,
        (rgb.2 as f32 * s).min(255.0) as u8,
    )
}
