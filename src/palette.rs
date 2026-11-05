use crate::world::Biome;

pub fn biome_rgb(b: Biome) -> (u8, u8, u8) {
    match b {
        Biome::Ocean => (20, 50, 120),
        Biome::Lake => (110, 190, 240),
        Biome::River => (60, 110, 210),
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
