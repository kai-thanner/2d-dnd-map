use dnd_2d_map::config::Config;
use dnd_2d_map::edit::{Edit, GameState, PoiKind};
use dnd_2d_map::gen;
use dnd_2d_map::palette;
use dnd_2d_map::rng;
use dnd_2d_map::save;
use dnd_2d_map::world::{Biome, Poi, SettlementTier, WaterKind};
use eframe::egui;
use rand::rngs::StdRng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tool {
    Select,
    AddSettlement,
    DestroySettlement,
    AddDungeon,
    RemovePoi,
    AddInn,
    AddMerchant,
    AddRoad,
    AddRiver,
    BurnForest,
    DrainLake,
    Flood,
}

const TOOLS: [Tool; 12] = [
    Tool::Select,
    Tool::AddSettlement,
    Tool::DestroySettlement,
    Tool::AddDungeon,
    Tool::RemovePoi,
    Tool::AddInn,
    Tool::AddMerchant,
    Tool::AddRoad,
    Tool::AddRiver,
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
            Tool::Select => "Auswählen / Info (Klick pinnt Kachel)",
            Tool::AddSettlement => "Siedlung setzen (Linksklick, Default 150 Ew. – danach pinnen & Einwohner ändern)",
            Tool::DestroySettlement => "Siedlung zerstören (Linksklick)",
            Tool::AddDungeon => "Dungeon setzen (Linksklick)",
            Tool::RemovePoi => "POI entfernen (Linksklick)",
            Tool::AddInn => "Gasthaus setzen (Linksklick)",
            Tool::AddMerchant => "Händler setzen (Linksklick)",
            Tool::AddRoad => "Straße bauen (2 Linksklicks)",
            Tool::AddRiver => "Fluss anlegen (Quelle anklicken)",
            Tool::BurnForest => "Wald abbrennen (Linksklick)",
            Tool::DrainLake => "See/Fluss austrocknen (Linksklick)",
            Tool::Flood => "See anlegen (Land fluten, Linksklick + Radius)",
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
    pan: egui::Vec2,
    hover: Option<(usize, usize)>,
    selected: Option<(usize, usize)>,
    edit_for: Option<(usize, usize)>,
    edit_name: String,
    edit_pop: u32,
    map_name: Option<String>,
    dirty: bool,
    save_open: bool,
    save_input: String,
    load_open: bool,
    load_entries: Vec<(String, String)>,
    load_warn: Option<String>,
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
            pan: egui::vec2(8.0, 8.0),
            hover: None,
            selected: None,
            edit_for: None,
            edit_name: String::new(),
            edit_pop: 0,
            map_name: None,
            dirty: false,
            save_open: false,
            save_input: "karte.json".to_string(),
            load_open: false,
            load_entries: Vec::new(),
            load_warn: None,
            status: "Bereit – Werte links einstellen und „Generieren“ drücken.".to_string(),
        }
    }
}

impl MapApp {
    fn generate(&mut self) {
        let world = gen::generate(&self.cfg);
        self.state = Some(GameState::new(world));
        self.rng = rng::edit_rng(self.cfg.seed);
        self.road_from = None;
        self.selected = None;
        self.map_name = None;
        self.dirty = true;
        self.status = format!(
            "Karte generiert (Seed {}, {}×{})",
            self.cfg.seed, self.cfg.width, self.cfg.height
        );
    }

    fn open_save_dialog(&mut self) {
        if self.state.is_none() {
            self.status = "Nichts zu speichern – erst generieren.".to_string();
            return;
        }
        self.save_input = self.file_path.clone();
        self.save_open = true;
    }

    fn do_save(&mut self) {
        let Some(state) = &self.state else { return };
        let path = std::path::Path::new(&self.save_input);
        match save::save(path, &self.cfg, state) {
            Ok(()) => {
                self.file_path = self.save_input.clone();
                self.map_name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string());
                self.dirty = false;
                let rev = state.revision;
                self.status = format!("Gespeichert als „{}“ (Revision {rev})", self.save_input);
            }
            Err(e) => self.status = format!("Fehler beim Speichern: {e}"),
        }
    }

    fn scan_saves(&mut self) {
        self.load_entries.clear();
        let dir = std::path::Path::new(&self.file_path)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().map_or(false, |x| x == "json") {
                    if let Ok((_cfg, st)) = save::load(&p) {
                        let name = p.file_stem().unwrap().to_string_lossy().to_string();
                        self.load_entries
                            .push((format!("{name}  (Revision {})", st.revision), p.to_string_lossy().to_string()));
                    }
                }
            }
        }
        self.load_entries.sort();
    }

    fn open_load_dialog(&mut self) {
        self.scan_saves();
        self.load_warn = None;
        self.load_open = true;
    }

    fn do_load(&mut self, path: &str) {
        let p = std::path::Path::new(path);
        match save::load(p) {
            Ok((cfg, state)) => {
                self.cfg = cfg;
                self.rng = rng::edit_rng(self.cfg.seed);
                self.road_from = None;
                self.selected = None;
                let rev = state.revision;
                self.state = Some(state);
                self.file_path = path.to_string();
                self.map_name = p.file_stem().map(|s| s.to_string_lossy().to_string());
                self.dirty = false;
                self.status = format!("Geladen: {path} (Revision {rev})");
            }
            Err(e) => self.status = format!("Fehler beim Laden: {e}"),
        }
    }

    fn start_game(&mut self) {
        self.status =
            "„Spiel starten“ kommt in Phase 2 (Server/Client mit Erzähler & Spielern).".to_string();
    }

    fn undo(&mut self) {
        if self.state.as_ref().map_or(true, |s| s.log.is_empty()) {
            self.status = "Nichts rückgängig zu machen.".to_string();
            return;
        }
        let cfg = self.cfg.clone();
        let state = self.state.as_mut().unwrap();
        state.undo(&cfg);
        self.road_from = None;
        self.dirty = true;
        self.status = format!("Rückgängig – Revision {}", state.revision);
    }

    fn sync_edit_buffers(&mut self) {
        let sel = self.selected;
        let data = sel.and_then(|(x, y)| {
            self.state
                .as_ref()
                .map(|s| (x, y, s.world.get(x, y).unwrap().settlement.clone()))
        });
        self.edit_for = sel;
        if let Some((_, _, Some(s))) = data {
            self.edit_name = s.name;
            self.edit_pop = s.population;
        }
    }

    fn remove_at(&mut self, pos: (usize, usize)) {
        let Some(state) = &self.state else {
            self.status = "Erst eine Karte generieren/laden.".to_string();
            return;
        };
        let t = state.world.get(pos.0, pos.1).unwrap();
        let edit = if t.poi.is_some() {
            Some(Edit::RemovePoi { x: pos.0, y: pos.1 })
        } else if t.settlement.is_some() {
            Some(Edit::DestroySettlement { x: pos.0, y: pos.1 })
        } else {
            None
        };
        match edit {
            Some(e) => {
                let state = self.state.as_mut().unwrap();
                state.apply(e, &mut self.rng);
                self.dirty = true;
                self.status = format!(
                    "Entfernt an ({}, {}) – Revision {}",
                    pos.0, pos.1, state.revision
                );
            }
            None => {
                self.status = format!(
                    "An ({}, {}) ist kein POI / keine Siedlung zum Entfernen.",
                    pos.0, pos.1
                )
            }
        }
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
            self.dirty = true;
            let after = state.world.tiles.iter().filter(|t| t.road).count();
            self.status = format!(
                "Straße ({},{}) → ({},{}) prozedural gebaut, +{} Straßenkacheln (Revision {})",
                from.0, from.1, pos.0, pos.1, after - before, state.revision
            );
            return;
        }
        let edit = match self.tool {
            Tool::Select => None,
            Tool::AddSettlement => Some(Edit::AddSettlement {
                x: pos.0,
                y: pos.1,
                population: 150,
                name: None,
            }),
            Tool::DestroySettlement => Some(Edit::DestroySettlement { x: pos.0, y: pos.1 }),
            Tool::AddDungeon => Some(Edit::AddDungeon { x: pos.0, y: pos.1 }),
            Tool::RemovePoi => Some(Edit::RemovePoi { x: pos.0, y: pos.1 }),
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
                self.status =
                    format!("Straßen-Start bei ({}, {}) – jetzt Ziel anklicken.", pos.0, pos.1);
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
            Tool::AddRiver => Some(Edit::AddRiver { x: pos.0, y: pos.1 }),
            Tool::Flood => Some(Edit::Flood {
                x: pos.0,
                y: pos.1,
                radius: self.radius,
            }),
        };
        if let Some(e) = edit {
            let state = self.state.as_mut().unwrap();
            state.apply(e, &mut self.rng);
            self.dirty = true;
            self.status = format!("{} – Revision {}", self.tool.label(), state.revision);
        }
    }

    fn draw_map(&mut self, ui: &mut egui::Ui) {
        if self.state.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label("Noch keine Karte vorhanden – links „Generieren“ drücken.");
            });
            return;
        }
        let avail = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(avail, egui::Sense::click_and_drag());
        let rect = response.rect;

        let (middle_down, delta) = ui.input(|i| {
            (
                i.pointer.button_down(egui::PointerButton::Middle),
                i.pointer.delta(),
            )
        });
        if middle_down && response.hovered() {
            self.pan += delta;
        }
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            if let Some(p) = ui.input(|i| i.pointer.latest_pos()) {
                if rect.contains(p) {
                    let factor = (scroll * 0.0016).exp();
                    let new_cell = (self.cell * factor).clamp(4.0, 40.0);
                    let world_pos = (p - rect.min - self.pan) / self.cell;
                    self.pan = (p - rect.min) - world_pos * new_cell;
                    self.cell = new_cell;
                }
            }
        }

        let cell = self.cell;
        let origin = rect.min + self.pan;
        let road_from = self.road_from;
        let selected = self.selected;

        let mut hover_cell = None;
        let mut primary_cell = None;
        let mut secondary_cell = None;
        {
            let Some(state) = &self.state else { return };
            let world = &state.world;
            let painter = ui.painter_at(rect);
            let map_rect = egui::Rect::from_min_size(
                origin,
                egui::vec2(world.width as f32 * cell, world.height as f32 * cell),
            );

            if let Some(p) = ui.input(|i| i.pointer.latest_pos()) {
                if map_rect.contains(p) {
                    let rel = p - origin;
                    hover_cell = palette::cell_at(rel.x, rel.y, cell, world.width, world.height);
                }
            }
            if let Some(p) = response.interact_pointer_pos() {
                let rel = p - origin;
                let c = palette::cell_at(rel.x, rel.y, cell, world.width, world.height);
                if response.clicked_by(egui::PointerButton::Primary) {
                    primary_cell = c;
                }
                if response.clicked_by(egui::PointerButton::Secondary) {
                    secondary_cell = c;
                }
            }

            let x0 = (((rect.min.x - origin.x) / cell).floor().max(0.0)) as usize;
            let y0 = (((rect.min.y - origin.y) / cell).floor().max(0.0)) as usize;
            let x1 = ((((rect.max.x - origin.x) / cell).ceil()) as usize).min(world.width);
            let y1 = ((((rect.max.y - origin.y) / cell).ceil()) as usize).min(world.height);

            for y in y0..y1 {
                for x in x0..x1 {
                    let t = world.get(x, y).unwrap();
                    let c0 = origin + egui::vec2(x as f32 * cell, y as f32 * cell);
                    let cell_rect = egui::Rect::from_min_size(c0, egui::vec2(cell, cell));
                    let (r, g, b) = palette::biome_rgb(t.effective_biome());
                    painter.rect_filled(cell_rect, 0.0, egui::Color32::from_rgb(r, g, b));
                }
            }

            let (rr, rg, rb) = palette::biome_rgb(Biome::River);
            let river_col = egui::Color32::from_rgb(rr, rg, rb);
            for y in y0..y1 {
                for x in x0..x1 {
                    let t = world.get(x, y).unwrap();
                    if t.water != Some(WaterKind::River) && !t.river {
                        continue;
                    }
                    let wdt = t.river_width.max(1) as f32;
                    let stroke =
                        egui::Stroke::new((cell * 0.14 * wdt).clamp(1.5, cell * 0.9), river_col);
                    let center =
                        origin + egui::vec2((x as f32 + 0.5) * cell, (y as f32 + 0.5) * cell);
                    for (nx, ny) in world.neighbors8(x, y) {
                        let nt = world.get(nx, ny).unwrap();
                        if nt.water == Some(WaterKind::River) || nt.river {
                            let nc = origin
                                + egui::vec2((nx as f32 + 0.5) * cell, (ny as f32 + 0.5) * cell);
                            painter.line_segment([center, nc], stroke);
                        }
                    }
                }
            }

            let road_stroke =
                egui::Stroke::new((cell * 0.22).max(1.5), egui::Color32::from_rgb(122, 84, 44));
            for y in y0..y1 {
                for x in x0..x1 {
                    if !world.get(x, y).unwrap().road {
                        continue;
                    }
                    let center = origin + egui::vec2((x as f32 + 0.5) * cell, (y as f32 + 0.5) * cell);
                    for (nx, ny) in world.neighbors8(x, y) {
                        if world.get(nx, ny).unwrap().road {
                            let nc =
                                origin + egui::vec2((nx as f32 + 0.5) * cell, (ny as f32 + 0.5) * cell);
                            painter.line_segment([center, nc], road_stroke);
                        }
                    }
                }
            }

            for y in y0..y1 {
                for x in x0..x1 {
                    let t = world.get(x, y).unwrap();
                    let center = origin + egui::vec2((x as f32 + 0.5) * cell, (y as f32 + 0.5) * cell);
                    if let Some(tier) = world.settlement_tier(x, y) {
                        let (col, ch, rad) = match tier {
                            SettlementTier::Gehoeft => (egui::Color32::from_rgb(170, 170, 170), "g", 0.22),
                            SettlementTier::Dorf => (egui::Color32::from_rgb(255, 200, 120), "v", 0.28),
                            SettlementTier::Kleinstadt => (egui::Color32::from_rgb(255, 230, 120), "t", 0.32),
                            SettlementTier::Stadt => (egui::Color32::GOLD, "C", 0.36),
                            SettlementTier::Kaiserstadt => (egui::Color32::from_rgb(255, 170, 40), "M", 0.44),
                            SettlementTier::Hafenstadt => (egui::Color32::from_rgb(90, 205, 235), "H", 0.40),
                        };
                        painter.circle_filled(center, (cell * rad).max(2.0), col);
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

            let mark = |pos: (usize, usize), col: egui::Color32| {
                let r = egui::Rect::from_min_size(
                    origin + egui::vec2(pos.0 as f32 * cell, pos.1 as f32 * cell),
                    egui::vec2(cell, cell),
                );
                painter.rect_filled(r, 0.0, col);
            };
            if let Some(pos) = road_from {
                mark(pos, egui::Color32::from_rgba_premultiplied(255, 60, 60, 110));
            }
            if let Some(pos) = selected {
                mark(pos, egui::Color32::from_rgba_premultiplied(255, 255, 120, 90));
            }
            if let Some(pos) = hover_cell {
                mark(pos, egui::Color32::from_white_alpha(70));
            }
        }

        self.hover = hover_cell;
        if let Some(pos) = secondary_cell {
            self.remove_at(pos);
        }
        if let Some(pos) = primary_cell {
            if self.tool == Tool::Select {
                self.selected = Some(pos);
                self.sync_edit_buffers();
                self.status = format!("Kachel ({}, {}) ausgewählt.", pos.0, pos.1);
            } else {
                self.apply_tool(pos);
            }
        }
    }
}

impl eframe::App for MapApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
            self.undo();
        }

        egui::containers::Panel::top("menubar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("Datei", |ui| {
                    if ui.button("Generieren").clicked() {
                        self.generate();
                        ui.close();
                    }
                    if ui.button("Speichern…").clicked() {
                        self.open_save_dialog();
                        ui.close();
                    }
                    if ui.button("Laden…").clicked() {
                        self.open_load_dialog();
                        ui.close();
                    }
                    if ui.button("Rückgängig (Ctrl+Z)").clicked() {
                        self.undo();
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

        egui::containers::Panel::left("sidebar").default_size(300.0).show(ui, |ui| {
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
                if ui.add_sized([280.0, 32.0], egui::Button::new("▶ Generieren")).clicked() {
                    self.generate();
                }

                ui.separator();
                ui.heading("Datei");
                let name_label = match &self.map_name {
                    Some(n) => format!("Kartenname: {n}{}", if self.dirty { "  (ungespeichert)" } else { "" }),
                    None => "Karte noch nicht gespeichert".to_string(),
                };
                ui.label(name_label);
                ui.horizontal(|ui| {
                    if ui.button("Speichern…").clicked() {
                        self.open_save_dialog();
                    }
                    if ui.button("Laden…").clicked() {
                        self.open_load_dialog();
                    }
                    if ui.button("Spiel starten").clicked() {
                        self.start_game();
                    }
                });

                ui.separator();
                ui.heading("Werkzeuge");
                ui.label("Linksklick = Werkzeug anwenden, Rechtsklick = POI/Siedlung entfernen.");
                for t in TOOLS {
                    ui.radio_value(&mut self.tool, t, t.label());
                }
                if matches!(self.tool, Tool::BurnForest | Tool::DrainLake | Tool::Flood) {
                    ui.add(egui::Slider::new(&mut self.radius, 1..=20u32).text("Radius"));
                }
                if self.tool == Tool::AddRoad && self.road_from.is_some() {
                    ui.label(format!(
                        "Straßen-Start gesetzt: {:?} – Ziel anklicken",
                        self.road_from.unwrap()
                    ));
                    if ui.button("Straßen-Modus abbrechen").clicked() {
                        self.road_from = None;
                    }
                }
                ui.horizontal(|ui| {
                    let can_undo = self.state.as_ref().map_or(false, |s| !s.log.is_empty());
                    if ui.add_enabled(can_undo, egui::Button::new("↩ Rückgängig")).clicked() {
                        self.undo();
                    }
                });

                ui.separator();
                ui.heading("Ansicht");
                ui.add(egui::Slider::new(&mut self.cell, 4.0..=40.0).text("Zoom (px/Kachel)"));
                ui.label("Mausrad = Zoom an Mausposition, mittlere Maustaste = Karte ziehen.");
                if ui.button("Ansicht zurücksetzen").clicked() {
                    self.pan = egui::vec2(8.0, 8.0);
                    self.cell = 10.0;
                }

                ui.separator();
                ui.collapsing("Legende", |ui| {
                    for b in ALL_BIOMES {
                        ui.horizontal(|ui| {
                            let (r, _resp) =
                                ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                            let (rr, gg, bb) = palette::biome_rgb(b);
                            ui.painter().rect_filled(r, 2.0, egui::Color32::from_rgb(rr, gg, bb));
                            ui.label(b.name());
                        });
                    }
                    ui.label("— Straße   ● g Gehöft  ● v Dorf  ● t Kleinstadt  ● C Stadt  ● M Kaiserstadt  ● H Hafenstadt   D Dungeon   I Gasthaus   M Händler");
                });

                ui.separator();
                ui.heading("Kachel-Info");
                let info_pos = self.selected.or(self.hover);
                let info = info_pos.and_then(|(x, y)| {
                    self.state.as_ref().map(|s| {
                        (
                            x,
                            y,
                            s.world.get(x, y).unwrap().clone(),
                            s.world.settlement_tier(x, y),
                        )
                    })
                });
                if let Some((hx, hy, t, tier)) = info {
                    ui.label(format!("Pos: ({hx}, {hy}){}", if self.selected == Some((hx, hy)) { "  [angepinnt]" } else { "" }));
                    ui.label(format!(
                        "Biom: {}   Temp: {:.1} °C",
                        t.effective_biome(),
                        t.temperature
                    ));
                    if t.is_water() {
                        ui.label(format!("Wasser-Layer: {:?} – darunter Land: {}", t.water, t.biome));
                    }
                    ui.label(format!("Höhe: {:.2}   Feuchte: {:.2}", t.elevation, t.moisture));
                    ui.label(format!("Straße: {}   Fluss: {}", t.road, t.river));
                    if let Some(s) = &t.settlement {
                        let tier_name = tier.map(|t| t.name()).unwrap_or("?");
                        ui.label(format!("Siedlung: {} ({}, {} Einwohner)", s.name, tier_name, s.population));
                        if self.selected == Some((hx, hy)) {
                            ui.horizontal(|ui| {
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut self.edit_name);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Einwohner:");
                                ui.add(egui::DragValue::new(&mut self.edit_pop).range(1..=1_000_000u32));
                            });
                            if ui.button("Siedlung aktualisieren").clicked() {
                                let state = self.state.as_mut().unwrap();
                                state.apply(
                                    Edit::UpdateSettlement {
                                        x: hx,
                                        y: hy,
                                        name: Some(self.edit_name.clone()),
                                        population: Some(self.edit_pop),
                                    },
                                    &mut self.rng,
                                );
                                self.status = format!("Siedlung aktualisiert – Revision {}", state.revision);
                            }
                        }
                    }
                    if let Some(p) = &t.poi {
                        ui.label(format!("POI: {p:?}"));
                    }
                } else {
                    ui.label("– mit „Auswählen“ auf die Karte klicken –");
                }
                if let Some(state) = &self.state {
                    ui.label(format!(
                        "Revision: {}   Karte: {}×{}   Edits im Log: {}",
                        state.revision,
                        state.world.width,
                        state.world.height,
                        state.log.len()
                    ));
                }
            });
        });

        egui::CentralPanel::default().show(ui, |ui| {
            self.draw_map(ui);
        });

        egui::containers::Panel::bottom("statusbar").show(ui, |ui| {
            ui.label(&self.status);
        });

        if self.save_open {
            let mut open = true;
            let title = self
                .map_name
                .clone()
                .unwrap_or_else(|| "Karte nicht gespeichert".to_string());
            egui::Window::new(format!("Speichern – {title}"))
                .open(&mut open)
                .show(ui.ctx(), |ui| {
                    if self.map_name.is_none() {
                        ui.label("Noch nicht gespeichert – Dateiname angeben:");
                    }
                    ui.horizontal(|ui| {
                        ui.label("Datei:");
                        ui.text_edit_singleline(&mut self.save_input);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Speichern").clicked() {
                            self.do_save();
                            self.save_open = false;
                        }
                        if ui.button("Abbrechen").clicked() {
                            self.save_open = false;
                        }
                    });
                });
            self.save_open = open && self.save_open;
        }

        if self.load_open {
            let mut open = true;
            egui::Window::new("Karte laden")
                .open(&mut open)
                .show(ui.ctx(), |ui| {
                    if let Some(warn) = self.load_warn.clone() {
                        ui.label("Achtung: die aktuelle Karte ist noch nicht gespeichert!");
                        ui.label(format!("Wirklich „{warn}“ laden und Änderungen verlieren?"));
                        ui.horizontal(|ui| {
                            if ui.button("Trotzdem laden").clicked() {
                                self.load_warn = None;
                                self.do_load(&warn);
                                self.load_open = false;
                            }
                            if ui.button("Abbrechen").clicked() {
                                self.load_warn = None;
                            }
                        });
                    } else {
                        let entries = self.load_entries.clone();
                        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                            if entries.is_empty() {
                                ui.label("Keine gespeicherten Karten gefunden (*.json im aktuellen Verzeichnis).");
                            }
                            for (label, path) in entries {
                                if ui.button(label).clicked() {
                                    if self.dirty && self.state.is_some() {
                                        self.load_warn = Some(path);
                                    } else {
                                        self.do_load(&path);
                                        self.load_open = false;
                                    }
                                }
                            }
                        });
                    }
                });
            self.load_open = open && self.load_open;
        }
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
