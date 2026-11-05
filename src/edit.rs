use crate::config::Config;
use crate::gen;
use crate::gen::rivers;
use crate::gen::roads;
use crate::names;
use crate::rng;
use crate::world::{Biome, Poi, Settlement, SettlementKind, WaterKind, World};
use rand::rngs::StdRng;
use rand::RngExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoiKind {
    Dungeon,
    Inn,
    Merchant,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Edit {
    AddDungeon { x: usize, y: usize },
    RemoveDungeon { x: usize, y: usize },
    RemovePoi { x: usize, y: usize },
    AddSettlement {
        x: usize,
        y: usize,
        kind: SettlementKind,
        name: Option<String>,
    },
    UpdateSettlement {
        x: usize,
        y: usize,
        name: Option<String>,
        population: Option<u32>,
    },
    DestroySettlement { x: usize, y: usize },
    BurnForest { x: usize, y: usize, radius: u32 },
    DrainLake { x: usize, y: usize, radius: u32 },
    Flood { x: usize, y: usize, radius: u32 },
    AddRoad { from: (usize, usize), to: (usize, usize) },
    AddRiver { x: usize, y: usize },
    AddPoi { x: usize, y: usize, kind: PoiKind },
    SetBiome { x: usize, y: usize, biome: Biome },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    pub world: World,
    pub revision: u64,
    pub log: Vec<Edit>,
}

impl GameState {
    pub fn new(world: World) -> Self {
        Self {
            world,
            revision: 0,
            log: Vec::new(),
        }
    }

    pub fn apply(&mut self, edit: Edit, rng: &mut StdRng) {
        self.world.apply(&edit, rng);
        self.revision += 1;
        self.log.push(edit);
    }

    pub fn undo(&mut self, cfg: &Config) {
        if self.log.pop().is_some() {
            *self = rebuild(cfg, &self.log);
        }
    }
}

pub fn rebuild(cfg: &Config, log: &[Edit]) -> GameState {
    let world = gen::generate(cfg);
    let mut state = GameState::new(world);
    let mut rng = rng::edit_rng(cfg.seed);
    for e in log {
        state.apply(e.clone(), &mut rng);
    }
    state
}

impl World {
    pub fn apply(&mut self, edit: &Edit, rng: &mut StdRng) {
        match edit {
            Edit::AddDungeon { x, y } => {
                if let Some(t) = self.get_mut(*x, *y) {
                    if t.is_passable() && t.poi.is_none() {
                        t.poi = Some(Poi::Dungeon {
                            name: names::dungeon(rng),
                        });
                    }
                }
            }
            Edit::RemoveDungeon { x, y } => {
                if let Some(t) = self.get_mut(*x, *y) {
                    if matches!(t.poi, Some(Poi::Dungeon { .. })) {
                        t.poi = None;
                    }
                }
            }
            Edit::RemovePoi { x, y } => {
                if let Some(t) = self.get_mut(*x, *y) {
                    if t.poi.is_some() {
                        t.poi = None;
                    }
                }
            }
            Edit::AddSettlement { x, y, kind, name } => {
                if let Some(t) = self.get_mut(*x, *y) {
                    if t.is_passable() && t.settlement.is_none() {
                        let name = name.clone().unwrap_or_else(|| names::settlement(rng, *kind));
                        let population = match kind {
                            SettlementKind::City => rng.random_range(5000..50000u32),
                            SettlementKind::Village => rng.random_range(50..600u32),
                        };
                        t.settlement = Some(Settlement {
                            name,
                            kind: *kind,
                            population,
                        });
                    }
                }
            }
            Edit::UpdateSettlement { x, y, name, population } => {
                if let Some(t) = self.get_mut(*x, *y) {
                    if let Some(s) = &mut t.settlement {
                        if let Some(n) = name {
                            s.name = n.clone();
                        }
                        if let Some(p) = population {
                            s.population = *p;
                        }
                    }
                }
            }
            Edit::DestroySettlement { x, y } => {
                if let Some(t) = self.get_mut(*x, *y) {
                    t.settlement = None;
                    if matches!(t.poi, Some(Poi::Merchant { .. })) {
                        t.poi = None;
                    }
                }
            }
            Edit::BurnForest { x, y, radius } => {
                for (px, py) in self.tiles_in_radius(*x, *y, *radius) {
                    let t = self.get_mut(px, py).unwrap();
                    if t.water.is_some() {
                        continue;
                    }
                    if matches!(t.biome, Biome::Forest | Biome::Jungle | Biome::Taiga) {
                        t.biome = Biome::Grassland;
                    }
                }
            }
            Edit::DrainLake { x, y, radius } => {
                for (px, py) in self.tiles_in_radius(*x, *y, *radius) {
                    let t = self.get_mut(px, py).unwrap();
                    if matches!(t.water, Some(WaterKind::Lake) | Some(WaterKind::River)) {
                        t.water = None;
                        t.river = false;
                        t.river_width = 0;
                    }
                }
                rivers::recompute_river_widths(self);
            }
            Edit::Flood { x, y, radius } => {
                for (px, py) in self.tiles_in_radius(*x, *y, *radius) {
                    let t = self.get_mut(px, py).unwrap();
                    if t.water.is_none()
                        && t.is_passable()
                        && !matches!(t.biome, Biome::Rock)
                    {
                        t.water = Some(WaterKind::Lake);
                    }
                }
            }
            Edit::AddRoad { from, to } => {
                if let Some(path) = roads::carve_path(self, *from, *to) {
                    for (x, y) in path {
                        if let Some(t) = self.get_mut(x, y) {
                            t.road = true;
                        }
                    }
                }
                for p in [*from, *to] {
                    if !roads::has_adjacent_road(self, p) {
                        if let Some(r) = roads::nearest_road(self, p) {
                            if let Some(path) = roads::carve_path(self, p, r) {
                                for (x, y) in path {
                                    if let Some(t) = self.get_mut(x, y) {
                                        t.road = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Edit::AddRiver { x, y } => {
                let ok = self
                    .get(*x, *y)
                    .map_or(false, |t| t.elevation > self.sea_level && !t.is_water());
                if ok {
                    let sea_level = self.sea_level;
                    let path = rivers::carve_downhill(self, sea_level, (*x, *y));
                    for (px, py) in path {
                        let t = self.get_mut(px, py).unwrap();
                        t.water = Some(if t.temperature <= 0.0 {
                            WaterKind::Ice
                        } else {
                            WaterKind::River
                        });
                    }
                    rivers::recompute_river_widths(self);
                }
            }
            Edit::AddPoi { x, y, kind } => {
                if let Some(t) = self.get_mut(*x, *y) {
                    if t.poi.is_none() && t.is_passable() {
                        t.poi = Some(match kind {
                            PoiKind::Dungeon => Poi::Dungeon {
                                name: names::dungeon(rng),
                            },
                            PoiKind::Inn => Poi::Inn {
                                name: names::inn(rng),
                            },
                            PoiKind::Merchant => {
                                let (name, goods) = names::merchant(rng);
                                Poi::Merchant { name, goods }
                            }
                        });
                    }
                }
            }
            Edit::SetBiome { x, y, biome } => {
                if let Some(t) = self.get_mut(*x, *y) {
                    t.biome = *biome;
                }
            }
        }
    }
}
