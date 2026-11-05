use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Biome {
    Ocean,
    Lake,
    River,
    Ice,
    Coast,
    Desert,
    Steppe,
    Grassland,
    Forest,
    Jungle,
    Taiga,
    Tundra,
    Snow,
    Rock,
    Mountain,
}

impl Biome {
    pub fn is_water(&self) -> bool {
        matches!(self, Biome::Ocean | Biome::Lake | Biome::River | Biome::Ice)
    }

    pub fn is_passable(&self) -> bool {
        !matches!(self, Biome::Ocean | Biome::Lake | Biome::Ice | Biome::Mountain)
    }

    pub fn name(&self) -> &'static str {
        match self {
            Biome::Ocean => "ocean",
            Biome::Lake => "lake",
            Biome::River => "river",
            Biome::Ice => "ice",
            Biome::Coast => "coast",
            Biome::Desert => "desert",
            Biome::Steppe => "steppe",
            Biome::Grassland => "grassland",
            Biome::Forest => "forest",
            Biome::Jungle => "jungle",
            Biome::Taiga => "taiga",
            Biome::Tundra => "tundra",
            Biome::Snow => "snow",
            Biome::Rock => "rock",
            Biome::Mountain => "mountain",
        }
    }
}

impl fmt::Display for Biome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Biome {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "ocean" => Ok(Biome::Ocean),
            "lake" => Ok(Biome::Lake),
            "river" => Ok(Biome::River),
            "ice" => Ok(Biome::Ice),
            "coast" | "beach" => Ok(Biome::Coast),
            "desert" => Ok(Biome::Desert),
            "steppe" => Ok(Biome::Steppe),
            "grassland" | "grass" | "meadow" => Ok(Biome::Grassland),
            "forest" => Ok(Biome::Forest),
            "jungle" => Ok(Biome::Jungle),
            "taiga" => Ok(Biome::Taiga),
            "tundra" => Ok(Biome::Tundra),
            "snow" => Ok(Biome::Snow),
            "rock" => Ok(Biome::Rock),
            "mountain" => Ok(Biome::Mountain),
            other => Err(format!("unbekanntes Biome: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementKind {
    City,
    Village,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settlement {
    pub name: String,
    pub kind: SettlementKind,
    pub population: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Poi {
    Dungeon { name: String },
    Inn { name: String },
    Merchant { name: String, goods: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tile {
    pub elevation: f32,
    pub moisture: f32,
    pub temperature: f32,
    pub biome: Biome,
    pub river: bool,
    pub road: bool,
    pub settlement: Option<Settlement>,
    pub poi: Option<Poi>,
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            elevation: 0.0,
            moisture: 0.0,
            temperature: 0.0,
            biome: Biome::Grassland,
            river: false,
            road: false,
            settlement: None,
            poi: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub width: usize,
    pub height: usize,
    pub sea_level: f32,
    pub tiles: Vec<Tile>,
}

impl World {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            sea_level: 0.42,
            tiles: vec![Tile::default(); width * height],
        }
    }

    pub fn in_bounds(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height
    }

    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&Tile> {
        if self.in_bounds(x, y) {
            let i = self.idx(x, y);
            Some(&self.tiles[i])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut Tile> {
        if self.in_bounds(x, y) {
            let i = self.idx(x, y);
            Some(&mut self.tiles[i])
        } else {
            None
        }
    }

    pub fn neighbors8(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::with_capacity(8);
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x as i64 + dx;
                let ny = y as i64 + dy;
                if nx >= 0 && ny >= 0 && self.in_bounds(nx as usize, ny as usize) {
                    out.push((nx as usize, ny as usize));
                }
            }
        }
        out
    }

    pub fn neighbors4(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::with_capacity(4);
        for (dx, dy) in [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)] {
            let nx = x as i64 + dx;
            let ny = y as i64 + dy;
            if nx >= 0 && ny >= 0 && self.in_bounds(nx as usize, ny as usize) {
                out.push((nx as usize, ny as usize));
            }
        }
        out
    }

    pub fn tiles_in_radius(&self, cx: usize, cy: usize, radius: u32) -> Vec<(usize, usize)> {
        let r = radius as i64;
        let mut out = Vec::new();
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy > r * r {
                    continue;
                }
                let nx = cx as i64 + dx;
                let ny = cy as i64 + dy;
                if nx >= 0 && ny >= 0 && self.in_bounds(nx as usize, ny as usize) {
                    out.push((nx as usize, ny as usize));
                }
            }
        }
        out
    }

    pub fn settlements(&self) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if self.get(x, y).unwrap().settlement.is_some() {
                    out.push((x, y));
                }
            }
        }
        out
    }

    pub fn dist(a: (usize, usize), b: (usize, usize)) -> f32 {
        let dx = a.0 as f32 - b.0 as f32;
        let dy = a.1 as f32 - b.1 as f32;
        (dx * dx + dy * dy).sqrt()
    }
}
