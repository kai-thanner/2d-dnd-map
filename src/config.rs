use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Climate {
    pub south_temp: f32,
    pub north_temp: f32,
    pub lapse_per_unit: f32,
}

impl Default for Climate {
    fn default() -> Self {
        Self {
            south_temp: 28.0,
            north_temp: -6.0,
            lapse_per_unit: 55.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub seed: u64,
    pub width: usize,
    pub height: usize,
    pub sea_level: f32,
    pub moisture_scale: f32,
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
            settlement_density: 0.5,
            climate: Climate::default(),
        }
    }
}
