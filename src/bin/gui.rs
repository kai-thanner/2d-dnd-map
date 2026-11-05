use dnd_2d_map::config::Config;
use dnd_2d_map::edit::{Edit, GameState, PoiKind};
use dnd_2d_map::gen;
use dnd_2d_map::palette;
use dnd_2d_map::rng;
use dnd_2d_map::save;
use dnd_2d_map::world::{Biome, Poi, SettlementKind};
use eframe::egui;
use rand::rngs::StdRng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tool {
    Select,
    AddCity,
    AddVillage,
    DestroySettlement,
    AddDungeon,
    RemoveDungeon,
    AddInn,
    AddMerchant,
    AddRoad,
    BurnForest,
    DrainLake,
    Flood,
}

const TOOLS: [Tool; 12] = [
    Tool::Select,
    Tool::AddCity,
    Tool::AddVillage,
    Tool::DestroySettlement,
    Tool::AddDungeon,
    Tool::RemoveDungeon,
    Tool::AddInn,
    Tool::AddMerchant,
    Tool::AddRoad,
    Tool::BurnForest,
    Tool::DrainLake,
    Tool::Flood,
];

const ALL_BIOMES: [Biome; 15] = [
    Biome::Ocean,
    Biome::Lake,
    Biome::River,
    Biome::Ice,
    Biome::Coast,
    Biome::Desert,
    Biome::Steppe,
    Biome::Grassland,
    Biome::Forest,
    Biome::Jungle,
    Biome::Taiga,
    Biome::Tundra,
    Biome::Snow,
    Biome::Rock,
    Biome::Mountain,
];

impl Tool {
    fn label(self) -> &'static str {
        match self {
            Tool::Select => "Auswählen / Info",
            Tool::AddCity => "Stadt setzen",
            Tool::AddVillage => "Dorf setzen",
            Tool::DestroySettlement => "Siedlung zerstören",
            Tool::AddDungeon => "Dungeon setzen",
            Tool::RemoveDungeon => "Dungeon entfernen",
            Tool::AddInn => "Gasthaus setzen",
            Tool::AddMerchant => "Händler setzen",
            Tool::AddRoad => "Straße bauen (2 Klicks)",
            Tool::BurnForest => "Wald abbrennen",
            Tool::DrainLake => "See austrocknen",
            Tool::Flood => "Land fluten",
        }
    }
}

struct MapApp {
    cfg: Config,
    state: Option<GameState>,
    rng: StdRng,
    file_path: String,
    tool: Tool,
    road_from: Option<(usize, usize)>,
    radius: u32,
    cell: f32,
    hover: Option<(usize, usize)>,
    status: String,
}

impl Default for MapApp {
    fn default() -> Self {
        Self {
            cfg: Config::default(),
            state: None,
            rng: rng::from_seed(1),
            file_path: "karte.json".to_string(),
            tool: Tool::Select,
            road_from: None,
            radius: 3,
            cell: 10.0,
            hover: None,
            status: "Bereit – Werte links einstellen und „Generieren“ drücken.".to_string(),
        }
    }
}

impl MapApp {
    fn generate(&mut self) {
        let world = gen::generate(&self.cfg);
        self.state = Some(GameState::new(world));
        self.rng = rng::from_seed(self.cfg.seed ^ 0xED17_5EED);
        self.road_from = None;
        self.status = format!(
            "Karte generiert (Seed {}, {}×{})",
            self.cfg.seed, self.cfg.width, self.cfg.height
        );
    }

    fn save(&mut self) {
        match &self.state {
            Some(state) => {
                let path = std::path::Path::new(&self.file_path);
                match save::save(path, &self.cfg, state) {
                    Ok(()) => {
                        self.status = format!(
                            "Gespeichert: {} (Revision {})",
                            self.file_path, state.revision
                        )
                    }
                    Err(e) => self.status = format!("Fehler beim Speichern: {e}"),
                }
            }
            None => self.status = "Nichts zu speichern – erst generieren.".to_string(),
        }
    }

    fn load(&mut self) {
        let path = std::path::Path::new(&self.file_path);
        match save::load(path) {
            Ok((cfg, state)) => {
                self.cfg = cfg;
                self.rng = rng::from_seed(self.cfg.seed ^ 0xED17_5EED);
                self.road_from = None;
                let rev = state.revision;
                self.state = Some(state);
                self.status = format!("Geladen: {} (Revision {rev})", self.file_path);
            }
            Err(e) => self.status = format!("Fehler beim Laden: {e}"),
        }
    }

    fn start_game(&mut self) {
        self.status = "„Spiel starten“ kommt in Phase 2 (Server/Client mit Erzähler & Spielern).".to_string();
    }

    fn apply_tool(&mut self, pos: (usize, usize)) {
        if self.state.is_none() {
            self.status = "Erst eine Karte generieren.".to_string();
            return;
        }
        if self.tool == Tool::AddRoad && self.road_from.is_some() {
            let from = self.road_from.take().unwrap();
            let before = self
                .state
                .as_ref()
                .unwrap()
                .world
                .tiles
                .iter()
                .filter(|t| t.road)
                .count();
            let state = self.state.as_mut().unwrap();
            state.apply(Edit::AddRoad { from, to: pos }, &mut self.rng);
            let after = state.world.tiles.iter().filter(|t| t.road).count();
            self.status = format!(
                "Straße ({},{}) → ({},{}) prozedural gebaut, +{} Straßenkacheln (Revision {})",
                from.0, from.1, pos.0, pos.1, after - before, state.revision
            );
            return;
        }
        let edit = match self.tool {
            Tool::Select => None,
            Tool::AddCity => Some(Edit::AddSettlement {
                x: pos.0,
                y: pos.1,
                kind: SettlementKind::City,
                name: None,
            }),
            Tool::AddVillage => Some(Edit::AddSettlement {
                x: pos.0,
                y: pos.1,
                kind: SettlementKind::Village,
                name: None,
            }),
            Tool::DestroySettlement => Some(Edit::DestroySettlement { x: pos.0, y: pos.1 }),
            Tool::AddDungeon => Some(Edit::AddDungeon { x: pos.0, y: pos.1 }),
            Tool::RemoveDungeon => Some(Edit::RemoveDungeon { x: pos.0, y: pos.1 }),
            Tool::AddInn => Some(Edit::AddPoi {
                x: pos.0,
                y: pos.1,
                kind: PoiKind::Inn,
            }),
            Tool::AddMerchant => Some(Edit::AddPoi {
                x: pos.0,
                y: pos.1,
                kind: PoiKind::Merchant,
            }),
            Tool::AddRoad => {
                self.road_from = Some(pos);
                self.status = format!("Straßen-Start bei ({}, {}) – jetzt Ziel anklicken.", pos.0, pos.1);
                None
            }
            Tool::BurnForest => Some(Edit::BurnForest {
                x: pos.0,
                y: pos.1,
                radius: self.radius,
            }),
            Tool::DrainLake => Some(Edit::DrainLake {
                x: pos.0,
                y: pos.1,
                radius: self.radius,
            }),
            Tool::Flood => Some(Edit::Flood {
                x: pos.0,
                y: pos.1,
                radius: self.radius,
            }),
        };
        if let Some(e) = edit {
            let state = self.state.as_mut().unwrap();
            state.apply(e, &mut self.rng);
            self.status = format!("{} – Revision {}", self.tool.label(), state.revision);
        }
    }

    fn draw_map(&mut self, ui: &mut egui::Ui) {
        let Some(state) = &self.state else {
            ui.centered_and_justified(|ui| {
                ui.label("Noch keine Karte vorhanden – links „Generieren“ drücken.");
            });
            return;
        };
        let cell = self.cell;
        let road_from = self.road_from;
        let world = &state.world;
        let size = egui::vec2(world.width as f32 * cell, world.height as f32 * cell);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        let painter = ui.painter_at(rect);

        for y in 0..world.height {
            for x in 0..world.width {
                let t = world.get(x, y).unwrap();
                let c0 = rect.min + egui::vec2(x as f32 * cell, y as f32 * cell);
                let cell_rect = egui::Rect::from_min_size(c0, egui::vec2(cell, cell));
                let (r, g, b) = palette::biome_rgb(t.biome);
                painter.rect_filled(cell_rect, 0.0, egui::Color32::from_rgb(r, g, b));
            }
        }

        let road_stroke = egui::Stroke::new((cell * 0.22).max(1.5), egui::Color32::from_rgb(122, 84, 44));
        for y in 0..world.height {
            for x in 0..world.width {
                if !world.get(x, y).unwrap().road {
                    continue;
                }
                let center = rect.min + egui::vec2((x as f32 + 0.5) * cell, (y as f32 + 0.5) * cell);
                for (nx, ny) in world.neighbors8(x, y) {
                    if world.get(nx, ny).unwrap().road {
                        let nc = rect.min + egui::vec2((nx as f32 + 0.5) * cell, (ny as f32 + 0.5) * cell);
                        painter.line_segment([center, nc], road_stroke);
                    }
                }
            }
        }

        for y in 0..world.height {
            for x in 0..world.width {
                let t = world.get(x, y).unwrap();
                let center = rect.min + egui::vec2((x as f32 + 0.5) * cell, (y as f32 + 0.5) * cell);
                if let Some(s) = &t.settlement {
                    let (col, ch) = match s.kind {
                        SettlementKind::City => (egui::Color32::GOLD, "C"),
                        SettlementKind::Village => (egui::Color32::from_rgb(255, 200, 120), "o"),
                    };
                    let r = (cell * 0.32).max(2.0);
                    painter.circle_filled(center, r, col);
                    painter.text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        ch,
                        egui::FontId::proportional(cell * 0.6),
                        egui::Color32::BLACK,
                    );
                } else if let Some(p) = &t.poi {
                    let (ch, col) = match p {
                        Poi::Dungeon { .. } => ("D", egui::Color32::from_rgb(235, 70, 70)),
                        Poi::Inn { .. } => ("I", egui::Color32::from_rgb(245, 225, 90)),
                        Poi::Merchant { .. } => ("M", egui::Color32::from_rgb(90, 225, 225)),
                    };
                    painter.text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        ch,
                        egui::FontId::proportional(cell * 0.7),
                        col,
                    );
                }
            }
        }

        if let Some((x, y)) = road_from {
            let r = egui::Rect::from_min_size(
                rect.min + egui::vec2(x as f32 * cell, y as f32 * cell),
                egui::vec2(cell, cell),
            );
            painter.rect_filled(r, 0.0, egui::Color32::from_rgba_premultiplied(255, 60, 60, 110));
        }

        let mut hover_cell = None;
        if let Some(p) = ui.input(|i| i.pointer.latest_pos()) {
            if rect.contains(p) {
                let rel = p - rect.min;
                hover_cell = palette::cell_at(rel.x, rel.y, cell, world.width, world.height);
            }
        }
        let mut click_cell = None;
        if response.clicked() {
            if let Some(p) = response.interact_pointer_pos() {
                let rel = p - rect.min;
                click_cell = palette::cell_at(rel.x, rel.y, cell, world.width, world.height);
            }
        }
        if let Some((hx, hy)) = hover_cell {
            let r = egui::Rect::from_min_size(
                rect.min + egui::vec2(hx as f32 * cell, hy as f32 * cell),
                egui::vec2(cell, cell),
            );
            painter.rect_filled(r, 0.0, egui::Color32::from_white_alpha(70));
        }

        self.hover = hover_cell;
        if let Some(pos) = click_cell {
            self.apply_tool(pos);
        }
    }
}

impl eframe::App for MapApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::containers::Panel::top("menubar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("Datei", |ui| {
                    if ui.button("Generieren").clicked() {
                        self.generate();
                        ui.close();
                    }
                    if ui.button("Speichern").clicked() {
                        self.save();
                        ui.close();
                    }
                    if ui.button("Laden").clicked() {
                        self.load();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Beenden").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Spiel", |ui| {
                    if ui.button("Spiel starten").clicked() {
                        self.start_game();
                        ui.close();
                    }
                });
            });
        });

        egui::containers::Panel::left("sidebar").default_size(290.0).show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Karte generieren");
                ui.horizontal(|ui| {
                    ui.label("Seed:");
                    ui.add(egui::DragValue::new(&mut self.cfg.seed).range(0..=u64::MAX));
                    if ui.button("Zufall").clicked() {
                        self.cfg.seed = rand::random();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Breite:");
                    ui.add(egui::DragValue::new(&mut self.cfg.width).range(20..=400));
                    ui.label("Höhe:");
                    ui.add(egui::DragValue::new(&mut self.cfg.height).range(20..=400));
                });
                ui.add(egui::Slider::new(&mut self.cfg.sea_level, 0.0..=1.0).text("Wasser (sea_level)"));
                ui.add(egui::Slider::new(&mut self.cfg.moisture_scale, 0.0..=1.0).text("Feuchtigkeit"));
                ui.add(egui::Slider::new(&mut self.cfg.settlement_density, 0.0..=1.0).text("Besiedelung"));
                ui.add(egui::Slider::new(&mut self.cfg.climate.south_temp, -40.0..=40.0).text("Temp Süden (°C)"));
                ui.add(egui::Slider::new(&mut self.cfg.climate.north_temp, -40.0..=40.0).text("Temp Norden (°C)"));
                ui.add(egui::Slider::new(&mut self.cfg.climate.lapse_per_unit, 0.0..=100.0).text("Höhen-Lapse"));
                if ui.add_sized([270.0, 32.0], egui::Button::new("▶ Generieren")).clicked() {
                    self.generate();
                }

                ui.separator();
                ui.heading("Datei");
                ui.horizontal(|ui| {
                    ui.label("Pfad:");
                    ui.add(egui::TextEdit::singleline(&mut self.file_path).desired_width(190.0));
                });
                ui.horizontal(|ui| {
                    if ui.button("Speichern").clicked() {
                        self.save();
                    }
                    if ui.button("Laden").clicked() {
                        self.load();
                    }
                    if ui.button("Spiel starten").clicked() {
                        self.start_game();
                    }
                });

                ui.separator();
                ui.heading("Werkzeuge (Klick auf Karte)");
                for t in TOOLS {
                    ui.radio_value(&mut self.tool, t, t.label());
                }
                if matches!(self.tool, Tool::BurnForest | Tool::DrainLake | Tool::Flood) {
                    ui.add(egui::Slider::new(&mut self.radius, 1..=20u32).text("Radius"));
                }
                if self.tool == Tool::AddRoad && self.road_from.is_some() {
                    ui.label(format!("Start gesetzt: {:?} – Ziel anklicken", self.road_from.unwrap()));
                    if ui.button("Straßen-Modus abbrechen").clicked() {
                        self.road_from = None;
                    }
                }

                ui.separator();
                ui.heading("Ansicht");
                ui.add(egui::Slider::new(&mut self.cell, 4.0..=24.0).text("Zoom (px/Kachel)"));

                ui.separator();
                ui.collapsing("Legende", |ui| {
                    for b in ALL_BIOMES {
                        ui.horizontal(|ui| {
                            let (r, _resp) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                            let (rr, gg, bb) = palette::biome_rgb(b);
                            ui.painter().rect_filled(r, 2.0, egui::Color32::from_rgb(rr, gg, bb));
                            ui.label(b.name());
                        });
                    }
                    ui.label("— Straße   ● C Stadt   ● o Dorf   D Dungeon   I Gasthaus   M Händler");
                });

                ui.separator();
                ui.heading("Kachel-Info");
                let hover = self.hover;
                if let (Some((hx, hy)), Some(state)) = (hover, &self.state) {
                    let t = state.world.get(hx, hy).unwrap();
                    ui.label(format!("Pos: ({hx}, {hy})"));
                    ui.label(format!("Biom: {}   Temp: {:.1} °C", t.biome, t.temperature));
                    ui.label(format!("Höhe: {:.2}   Feuchte: {:.2}", t.elevation, t.moisture));
                    ui.label(format!("Straße: {}   Fluss: {}", t.road, t.river));
                    if let Some(s) = &t.settlement {
                        ui.label(format!("Siedlung: {} ({:?}, {} Einwohner)", s.name, s.kind, s.population));
                    }
                    if let Some(p) = &t.poi {
                        ui.label(format!("POI: {p:?}"));
                    }
                } else {
                    ui.label("– Maus über die Karte bewegen –");
                }
                if let Some(state) = &self.state {
                    ui.label(format!(
                        "Revision: {}   Karte: {}×{}",
                        state.revision, state.world.width, state.world.height
                    ));
                }
            });
        });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                self.draw_map(ui);
            });
        });

        egui::containers::Panel::bottom("statusbar").show(ui, |ui| {
            ui.label(&self.status);
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 850.0]),
        ..Default::default()
    };
    eframe::run_native(
        "dnd-2d-map",
        options,
        Box::new(|_cc| Ok(Box::new(MapApp::default()) as Box<dyn eframe::App>)),
    )
}
