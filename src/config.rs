use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Climate {
    pub south_temp: f32,
    pub north_temp: f32,
    pub lapse_per_unit: f32,
    #[serde(default)]
    pub snow_temp: f32,
}

impl Default for Climate {
    fn default() -> Self {
        Self {
            south_temp: 28.0,
            north_temp: -6.0,
            lapse_per_unit: 45.0,
            snow_temp: 0.0,
        }
    }
}

fn default_dry_south() -> f32 {
    0.45
}

fn default_river_amount() -> f32 {
    0.5
}

fn default_max_elevation_m() -> f32 {
    3000.0
}

fn default_treeline_m() -> f32 {
    2000.0
}

/// Fester Massstab: 14 px (1 Kachel) = 1000 m. 100 Kacheln breit = 100 km breit.
pub const TILE_SIZE_M: f32 = 1000.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub seed: u64,
    pub width: usize,
    pub height: usize,
    pub sea_level: f32,
    pub moisture_scale: f32,
    #[serde(default = "default_dry_south")]
    pub dry_south: f32,
    #[serde(default = "default_river_amount")]
    pub river_amount: f32,
    #[serde(default = "default_max_elevation_m")]
    pub max_elevation_m: f32,
    #[serde(default = "default_treeline_m")]
    pub treeline_m: f32,
    pub settlement_density: f32,
    pub climate: Climate,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            seed: 1,
            width: 120,
            height: 60,
            sea_level: 0.42,
            moisture_scale: 0.6,
            dry_south: default_dry_south(),
            river_amount: default_river_amount(),
            max_elevation_m: default_max_elevation_m(),
            treeline_m: default_treeline_m(),
            settlement_density: 0.5,
            climate: Climate::default(),
        }
    }
}
