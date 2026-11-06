use dnd_2d_map::config::{Config, TILE_SIZE_M};
use dnd_2d_map::edit::{Edit, GameState, PoiKind};
use dnd_2d_map::gen;
use dnd_2d_map::palette;
use dnd_2d_map::rng;
use dnd_2d_map::save;
use dnd_2d_map::view3d::View3D;
use dnd_2d_map::world::{Biome, Poi, SettlementTier, WaterKind, World};
use eframe::egui;
use rand::rngs::StdRng;

const SUB: usize = 3;

fn bilinear_elev(w: &World, fx: f32, fy: f32) -> f32 {
    let x0 = (fx.floor() as usize).min(w.width - 1);
    let y0 = (fy.floor() as usize).min(w.height - 1);
    let x1 = (x0 + 1).min(w.width - 1);
    let y1 = (y0 + 1).min(w.height - 1);
    let tx = (fx - x0 as f32).clamp(0.0, 1.0);
    let ty = (fy - y0 as f32).clamp(0.0, 1.0);
    let e00 = w.get(x0, y0).unwrap().elevation;
    let e10 = w.get(x1, y0).unwrap().elevation;
    let e01 = w.get(x0, y1).unwrap().elevation;
    let e11 = w.get(x1, y1).unwrap().elevation;
    let e0 = e00 + (e10 - e00) * tx;
    let e1 = e01 + (e11 - e01) * tx;
    e0 + (e1 - e0) * ty
}

fn is_edge_tile(w: &World, x: usize, y: usize) -> bool {
    let t = w.get(x, y).unwrap();
    let eb = t.effective_biome();
    let ew = t.water;
    w.neighbors4(x, y).iter().any(|n| {
        let nt = w.get(n.0, n.1).unwrap();
        nt.effective_biome() != eb || nt.water != ew
    })
}

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
            Tool::Select => "Auswählen / Info (Klick pinnt Kachel, ESC wählt ab)",
            Tool::AddSettlement => "Siedlung setzen (Linksklick, Default 150 Ew.)",
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
    view: View3D,
    hover: Option<(usize, usize)>,
    selected: Option<(usize, usize)>,
    selected_screen: Option<egui::Pos2>,
    edit_name: String,
    edit_pop: u32,
    map_name: Option<String>,
    dirty: bool,
    save_open: bool,
    save_input: String,
    load_open: bool,
    load_entries: Vec<(String, String)>,
    load_warn: Option<String>,
    tex: Option<egui::TextureHandle>,
    tex_key: (u64, usize, usize, u64, usize),
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
            view: View3D::default(),
            hover: None,
            selected: None,
            selected_screen: None,
            edit_name: String::new(),
            edit_pop: 0,
            map_name: None,
            dirty: false,
            save_open: false,
            save_input: "karte.json".to_string(),
            load_open: false,
            load_entries: Vec::new(),
            load_warn: None,
            tex: None,
            tex_key: (u64::MAX, 0, 0, 0, 0),
            status: "Bereit – Werte links einstellen und „Generieren“ drücken.".to_string(),
        }
    }
}

impl MapApp {
    fn generate(&mut self) {
        let world = gen::generate(&self.cfg);
        let sea = self.cfg.sea_level;
        let max_e = world.tiles.iter().map(|t| t.elevation).fold(0.0f32, f32::max);
        self.view.e_off = sea;
        self.view.e_scale = (self.cfg.max_elevation_m / TILE_SIZE_M) / (max_e - sea).max(0.01);
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
                self.map_name = path.file_stem().map(|s| s.to_string_lossy().to_string());
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
                        self.load_entries.push((
                            format!("{name}  (Revision {})", st.revision),
                            p.to_string_lossy().to_string(),
                        ));
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
                let sea = self.cfg.sea_level;
                let max_e = state.world.tiles.iter().map(|t| t.elevation).fold(0.0f32, f32::max);
                self.view.e_off = sea;
                self.view.e_scale = (self.cfg.max_elevation_m / TILE_SIZE_M) / (max_e - sea).max(0.01);
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
                .map(|s| s.world.get(x, y).unwrap().settlement.clone())
        });
        if let Some(Some(s)) = data {
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
                self.status =
                    format!("Entfernt an ({}, {}) – Revision {}", pos.0, pos.1, state.revision);
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
            Tool::AddRiver => Some(Edit::AddRiver { x: pos.0, y: pos.1 }),
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
            self.dirty = true;
            self.status = format!("{} – Revision {}", self.tool.label(), state.revision);
        }
    }

    fn tile_at(&self, p: (f32, f32), center: (f32, f32), rect: egui::Rect) -> Option<(usize, usize)> {
        let w = &self.state.as_ref()?.world;
        let cx = w.width as f32 / 2.0;
        let cy = w.height as f32 / 2.0;
        let (mut bx, mut by) = self.view.unproject(p, cx, cy, center, 0.0);
        if let Some(t0) = w.get(bx as usize, by as usize) {
            let (rx, ry) = self.view.unproject(p, cx, cy, center, t0.elevation);
            bx = rx;
            by = ry;
        }
        let mut best: Option<((usize, usize), f32)> = None;
        for dy in -2..=2i64 {
            for dx in -2..=2i64 {
                let tx = bx + dx as f32;
                let ty = by + dy as f32;
                if tx < 0.0 || ty < 0.0 || tx >= w.width as f32 || ty >= w.height as f32 {
                    continue;
                }
                let (x, y) = (tx as usize, ty as usize);
                let e = w.get(x, y).unwrap().elevation;
                let sp = self.view.project(x as f32 + 0.5, y as f32 + 0.5, e, cx, cy, center);
                let spp = egui::pos2(sp.0, sp.1);
                let d = spp.distance(egui::pos2(p.0, p.1));
                if rect.contains(spp) && best.map_or(true, |(_, bd)| d < bd) {
                    best = Some(((x, y), d));
                }
            }
        }
        best.filter(|(_, d)| *d < self.view.cell * 0.8).map(|(t, _)| t)
    }

    fn snap_to_settlement(
        &self,
        pos: (usize, usize),
        click: egui::Pos2,
        center: (f32, f32),
    ) -> Option<(usize, usize)> {
        let w = &self.state.as_ref()?.world;
        let cx = w.width as f32 / 2.0;
        let cy = w.height as f32 / 2.0;
        let mut best: Option<((usize, usize), f32)> = None;
        for dy in -2..=2i64 {
            for dx in -2..=2i64 {
                let x = pos.0 as i64 + dx;
                let y = pos.1 as i64 + dy;
                if x < 0 || y < 0 || x >= w.width as i64 || y >= w.height as i64 {
                    continue;
                }
                let (x, y) = (x as usize, y as usize);
                if w.get(x, y).unwrap().settlement.is_none() {
                    continue;
                }
                let e = w.get(x, y).unwrap().elevation;
                let sp = self.view.project(x as f32 + 0.5, y as f32 + 0.5, e, cx, cy, center);
                let d = egui::pos2(sp.0, sp.1).distance(click);
                if best.map_or(true, |(_, bd)| d < bd) {
                    best = Some(((x, y), d));
                }
            }
        }
        best.filter(|(_, d)| *d < self.view.cell * 1.6).map(|(t, _)| t)
    }

    fn ensure_texture(&mut self, ctx: &egui::Context) {
        let Some(state) = &self.state else { return };
        let ts = ((self.view.cell / 2.0) as usize).clamp(2, 8);
        let key = (
            state.revision,
            state.world.width,
            state.world.height,
            self.cfg.seed,
            ts,
        );
        if key == self.tex_key && self.tex.is_some() {
            return;
        }
        let img = {
            let w = &state.world;
            let iw = w.width * ts;
            let ih = w.height * ts;
            let mut px = vec![egui::Color32::BLACK; iw * ih];
            for py in 0..ih {
                for pxx in 0..iw {
                    let fx = pxx as f32 / ts as f32;
                    let fy = py as f32 / ts as f32;
                    let e_sub = bilinear_elev(w, fx, fy);
                    let tx = (fx.floor() as usize).min(w.width - 1);
                    let ty = (fy.floor() as usize).min(w.height - 1);
                    let t = w.get(tx, ty).unwrap();
                    let (r, g, b) = if e_sub < w.sea_level {
                        palette::shade_rgb(w, tx, ty, palette::biome_rgb(Biome::Ocean))
                    } else if t.water == Some(WaterKind::River) || t.river {
                        palette::shade_rgb(w, tx, ty, palette::biome_rgb(Biome::River))
                    } else if t.road {
                        (160, 120, 70)
                    } else {
                        palette::shade_rgb(w, tx, ty, palette::biome_rgb(t.biome))
                    };
                    px[py * iw + pxx] = egui::Color32::from_rgb(r, g, b);
                }
            }
            for (sx, sy) in w.settlements() {
                let t = w.get(sx, sy).unwrap();
                let tier = w.settlement_tier(sx, sy).unwrap();
                let (col, rad) = match tier {
                    SettlementTier::Gehoeft => (egui::Color32::from_rgb(170, 170, 170), 0.22),
                    SettlementTier::Dorf => (egui::Color32::from_rgb(255, 200, 120), 0.28),
                    SettlementTier::Kleinstadt => (egui::Color32::from_rgb(255, 230, 120), 0.32),
                    SettlementTier::Stadt => (egui::Color32::GOLD, 0.38),
                    SettlementTier::Kaiserstadt => (egui::Color32::from_rgb(255, 170, 40), 0.46),
                    SettlementTier::Hafenstadt => (egui::Color32::from_rgb(90, 205, 235), 0.42),
                };
                let has_port = t.settlement.as_ref().map_or(false, |s| s.has_port);
                let cxf = (sx as f32 + 0.5) * ts as f32;
                let cyf = (sy as f32 + 0.5) * ts as f32;
                let r = (rad * ts as f32).max(1.5);
                let ri = r as i32 + 2;
                for dy in -ri..=ri {
                    for dx in -ri..=ri {
                        let x = cxf as i32 + dx;
                        let y = cyf as i32 + dy;
                        if x < 0 || y < 0 || x >= iw as i32 || y >= ih as i32 {
                            continue;
                        }
                        let d = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();
                        let c = if has_port && d > r && d <= r + 1.6 {
                            egui::Color32::from_rgb(40, 110, 220)
                        } else if d <= r {
                            col
                        } else {
                            continue;
                        };
                        px[y as usize * iw + x as usize] = c;
                    }
                }
            }
            egui::ColorImage::new([iw, ih], px)
        };
        self.tex = Some(ctx.load_texture("map2d", img, egui::TextureOptions::LINEAR));
        self.tex_key = key;
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
        let center = (rect.center().x, rect.center().y);

        let (middle_down, delta, shift) = ui.input(|i| {
            (
                i.pointer.button_down(egui::PointerButton::Middle),
                i.pointer.delta(),
                i.modifiers.shift,
            )
        });
        let mouse = ui.input(|i| i.pointer.latest_pos());
        let (mcx, mcy) = self
            .state
            .as_ref()
            .map(|s| (s.world.width as f32 / 2.0, s.world.height as f32 / 2.0))
            .unwrap_or((0.0, 0.0));
        let pivot = match mouse {
            Some(m) if rect.contains(m) => (m.x, m.y),
            _ => center,
        };
        if middle_down && response.hovered() {
            if shift {
                self.view.tilt_deg = (self.view.tilt_deg - delta.y * 0.25).clamp(0.0, 90.0);
                if delta.x != 0.0 {
                    self.view.rotate_at(pivot, mcx, mcy, center, delta.x * 0.25);
                }
            } else {
                self.view.pan.0 += delta.x;
                self.view.pan.1 += delta.y;
            }
        }
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            if let Some(p) = ui.input(|i| i.pointer.latest_pos()) {
                if rect.contains(p) {
                    let pp = (p.x, p.y);
                    let factor = (scroll * 0.0016).exp();
                    let anchor = self.tile_at(pp, center, rect).map(|t| {
                        let w = &self.state.as_ref().unwrap().world;
                        let e = w.get(t.0, t.1).unwrap().elevation;
                        let cx = w.width as f32 / 2.0;
                        let cy = w.height as f32 / 2.0;
                        self.view.project(t.0 as f32 + 0.5, t.1 as f32 + 0.5, e, cx, cy, center)
                    });
                    match anchor {
                        Some(sp) => self.view.zoom_at(pp, sp, center, factor),
                        None => self.view.cell = (self.view.cell * factor).clamp(4.0, 40.0),
                    }
                }
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::Q)) {
            self.view.rotate_at(pivot, mcx, mcy, center, -6.0);
        }
        if ui.input(|i| i.key_pressed(egui::Key::E)) {
            self.view.rotate_at(pivot, mcx, mcy, center, 6.0);
        }
        if ui.input(|i| i.key_pressed(egui::Key::W) || i.key_pressed(egui::Key::ArrowUp)) {
            self.view.tilt_deg = (self.view.tilt_deg + 6.0).clamp(0.0, 90.0);
        }
        if ui.input(|i| i.key_pressed(egui::Key::S) || i.key_pressed(egui::Key::ArrowDown)) {
            self.view.tilt_deg = (self.view.tilt_deg - 6.0).clamp(0.0, 90.0);
        }

        let cell = self.view.cell;
        let road_from = self.road_from;
        let selected = self.selected;
        let fast = self.view.tilt_deg >= 89.9 && (self.view.rot_deg % 360.0).abs() < 0.01;

        self.ensure_texture(ui.ctx());

        let mut hover_cell = None;
        let mut primary_cell = None;
        let mut secondary_cell = None;
        let mut primary_screen = None;
        {
            let Some(state) = &self.state else { return };
            let world = &state.world;
            let cx = world.width as f32 / 2.0;
            let cy = world.height as f32 / 2.0;
            let painter = ui.painter_at(rect);

            if let Some(p) = ui.input(|i| i.pointer.latest_pos()) {
                if rect.contains(p) {
                    hover_cell = self.tile_at((p.x, p.y), center, rect);
                }
            }
            if let Some(p) = response.interact_pointer_pos() {
                let c = self.tile_at((p.x, p.y), center, rect);
                if response.clicked_by(egui::PointerButton::Primary) {
                    primary_cell = c;
                    primary_screen = Some(p);
                }
                if response.clicked_by(egui::PointerButton::Secondary) {
                    secondary_cell = c;
                }
            }

            let proj = |x: f32, y: f32, e: f32| {
                let sp = self.view.project(x, y, e, cx, cy, center);
                egui::pos2(sp.0, sp.1)
            };

            if fast {
                if let Some(tex) = &self.tex {
                    let p0 = proj(0.0, 0.0, self.view.e_off);
                    let p1 = proj(world.width as f32, world.height as f32, self.view.e_off);
                    painter.image(
                        tex.id(),
                        egui::Rect::from_two_pos(p0, p1),
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            } else {
                let mut tiles: Vec<(f32, f32, [egui::Pos2; 4], usize, usize)> = Vec::new();
                for y in 0..world.height {
                    for x in 0..world.width {
                        let cs = [
                            proj(x as f32, y as f32, bilinear_elev(world, x as f32, y as f32)),
                            proj(x as f32 + 1.0, y as f32, bilinear_elev(world, x as f32 + 1.0, y as f32)),
                            proj(x as f32 + 1.0, y as f32 + 1.0, bilinear_elev(world, x as f32 + 1.0, y as f32 + 1.0)),
                            proj(x as f32, y as f32 + 1.0, bilinear_elev(world, x as f32, y as f32 + 1.0)),
                        ];
                        let min_x = cs.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
                        let max_x = cs.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
                        let min_y = cs.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
                        let max_y = cs.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
                        let m = cell * 2.0 + 2.0;
                        if max_x < rect.min.x - m
                            || min_x > rect.max.x + m
                            || max_y < rect.min.y - m
                            || min_y > rect.max.y + m
                        {
                            continue;
                        }
                        let bp = proj(x as f32, y as f32, self.view.e_off);
                        tiles.push((bp.y, bp.x, cs, x, y));
                    }
                }
                tiles.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)));

                let quad = |pts: [egui::Pos2; 4], col: egui::Color32| {
                    painter.add(egui::Shape::convex_polygon(
                        pts.to_vec(),
                        col,
                        egui::Stroke::new(0.7, col),
                    ));
                };

                for (_, _, cs, x, y) in &tiles {
                    let (x, y) = (*x, *y);
                    let [c00, c10, c11, c01] = *cs;
                    let t = world.get(x, y).unwrap();
                    if cell >= 8.0 && is_edge_tile(world, x, y) {
                        for sy in 0..SUB {
                            for sx in 0..SUB {
                                let fx0 = x as f32 + sx as f32 / SUB as f32;
                                let fy0 = y as f32 + sy as f32 / SUB as f32;
                                let fx1 = x as f32 + (sx + 1) as f32 / SUB as f32;
                                let fy1 = y as f32 + (sy + 1) as f32 / SUB as f32;
                                let e_sub = bilinear_elev(world, (fx0 + fx1) * 0.5, (fy0 + fy1) * 0.5);
                                let (r, g, b) = if e_sub < world.sea_level {
                                    palette::biome_rgb(Biome::Ocean)
                                } else {
                                    palette::shade_rgb(world, x, y, palette::biome_rgb(t.biome))
                                };
                                let col = egui::Color32::from_rgb(r, g, b);
                                quad(
                                    [
                                        proj(fx0, fy0, bilinear_elev(world, fx0, fy0)),
                                        proj(fx1, fy0, bilinear_elev(world, fx1, fy0)),
                                        proj(fx1, fy1, bilinear_elev(world, fx1, fy1)),
                                        proj(fx0, fy1, bilinear_elev(world, fx0, fy1)),
                                    ],
                                    col,
                                );
                            }
                        }
                    } else {
                        let (r, g, b) = palette::shade_rgb(
                            world,
                            x,
                            y,
                            palette::biome_rgb(t.effective_biome()),
                        );
                        quad([c00, c10, c11, c01], egui::Color32::from_rgb(r, g, b));
                    }
                }

                let road_stroke =
                    egui::Stroke::new((cell * 0.22).max(1.5), egui::Color32::from_rgb(160, 120, 70));
                for y in 0..world.height {
                    for x in 0..world.width {
                        if !world.get(x, y).unwrap().road {
                            continue;
                        }
                        let sp = proj(x as f32, y as f32, world.get(x, y).unwrap().elevation);
                        if sp.x < rect.min.x - cell || sp.y < rect.min.y - cell
                            || sp.x > rect.max.x + cell || sp.y > rect.max.y + cell {
                            continue;
                        }
                        let t = world.get(x, y).unwrap();
                        let c0 = proj(x as f32 + 0.5, y as f32 + 0.5, t.elevation);
                        for (nx, ny) in world.neighbors8(x, y) {
                            let nt = world.get(nx, ny).unwrap();
                            if nt.road {
                                let c1 = proj(nx as f32 + 0.5, ny as f32 + 0.5, nt.elevation);
                                painter.line_segment([c0, c1], road_stroke);
                            }
                        }
                    }
                }

                for y in 0..world.height {
                    for x in 0..world.width {
                        let t = world.get(x, y).unwrap();
                        if t.water != Some(WaterKind::River) && !t.river {
                            continue;
                        }
                        let sp = proj(x as f32, y as f32, t.elevation);
                        if sp.x < rect.min.x - cell || sp.y < rect.min.y - cell
                            || sp.x > rect.max.x + cell || sp.y > rect.max.y + cell {
                            continue;
                        }
                        let (rr, rg, rb) =
                            palette::shade_rgb(world, x, y, palette::biome_rgb(Biome::River));
                        let wdt = t.river_width.max(1) as f32;
                        let stroke = egui::Stroke::new(
                            (cell * 0.14 * wdt).clamp(1.5, cell * 0.9),
                            egui::Color32::from_rgb(rr, rg, rb),
                        );
                        let c0 = proj(x as f32 + 0.5, y as f32 + 0.5, t.elevation);
                        for (nx, ny) in world.neighbors8(x, y) {
                            let nt = world.get(nx, ny).unwrap();
                            if nt.water == Some(WaterKind::River) || nt.river {
                                let c1 = proj(nx as f32 + 0.5, ny as f32 + 0.5, nt.elevation);
                                painter.line_segment([c0, c1], stroke);
                            }
                        }
                    }
                }

                for y in 0..world.height {
                    for x in 0..world.width {
                        let t = world.get(x, y).unwrap();
                        let sp = proj(x as f32, y as f32, t.elevation);
                        if sp.x < rect.min.x - cell || sp.y < rect.min.y - cell
                            || sp.x > rect.max.x + cell || sp.y > rect.max.y + cell {
                            continue;
                        }
                        let c0 = proj(x as f32 + 0.5, y as f32 + 0.5, t.elevation);
                        if world.settlement_tier(x, y).is_some() {
                            let tier = world.settlement_tier(x, y).unwrap();
                            let (col, rad) = match tier {
                                SettlementTier::Gehoeft => (egui::Color32::from_rgb(170, 170, 170), 0.22),
                                SettlementTier::Dorf => (egui::Color32::from_rgb(255, 200, 120), 0.28),
                                SettlementTier::Kleinstadt => (egui::Color32::from_rgb(255, 230, 120), 0.32),
                                SettlementTier::Stadt => (egui::Color32::GOLD, 0.38),
                                SettlementTier::Kaiserstadt => (egui::Color32::from_rgb(255, 170, 40), 0.46),
                                SettlementTier::Hafenstadt => (egui::Color32::from_rgb(90, 205, 235), 0.42),
                            };
                            let r = (cell * rad).max(2.0);
                            let has_port = t.settlement.as_ref().map_or(false, |s| s.has_port);
                            if has_port {
                                painter.circle_filled(c0, r + 2.5, egui::Color32::from_rgb(40, 110, 220));
                            }
                            painter.circle_filled(c0, r, col);
                        } else if let Some(p) = &t.poi {
                            let (ch, col) = match p {
                                Poi::Dungeon { .. } => ("D", egui::Color32::from_rgb(235, 70, 70)),
                                Poi::Inn { .. } => ("I", egui::Color32::from_rgb(245, 225, 90)),
                                Poi::Merchant { .. } => ("M", egui::Color32::from_rgb(90, 225, 225)),
                            };
                            painter.text(
                                c0,
                                egui::Align2::CENTER_CENTER,
                                ch,
                                egui::FontId::proportional(cell * 0.7),
                                col,
                            );
                        }
                    }
                }
            }

            if fast {
                for y in 0..world.height {
                    for x in 0..world.width {
                        let t = world.get(x, y).unwrap();
                        if t.poi.is_none() {
                            continue;
                        }
                        let sp = proj(x as f32, y as f32, t.elevation);
                        if sp.x < rect.min.x - cell || sp.y < rect.min.y - cell
                            || sp.x > rect.max.x + cell || sp.y > rect.max.y + cell {
                            continue;
                        }
                        let c0 = proj(x as f32 + 0.5, y as f32 + 0.5, t.elevation);
                        let (ch, col) = match &t.poi {
                            Some(Poi::Dungeon { .. }) => ("D", egui::Color32::from_rgb(235, 70, 70)),
                            Some(Poi::Inn { .. }) => ("I", egui::Color32::from_rgb(245, 225, 90)),
                            Some(Poi::Merchant { .. }) => ("M", egui::Color32::from_rgb(90, 225, 225)),
                            None => continue,
                        };
                        painter.text(
                            c0,
                            egui::Align2::CENTER_CENTER,
                            ch,
                            egui::FontId::proportional(cell * 0.7),
                            col,
                        );
                    }
                }
            }

            let mark = |pos: (usize, usize), col: egui::Color32| {
                let t = world.get(pos.0, pos.1).unwrap();
                let e = t.elevation;
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        proj(pos.0 as f32, pos.1 as f32, e),
                        proj(pos.0 as f32 + 1.0, pos.1 as f32, e),
                        proj(pos.0 as f32 + 1.0, pos.1 as f32 + 1.0, e),
                        proj(pos.0 as f32, pos.1 as f32 + 1.0, e),
                    ],
                    col,
                    egui::Stroke::NONE,
                ));
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

            let n0 = proj(cx, cy, 0.0);
            let n1 = proj(cx, cy - 1.0, 0.0);
            let mut nd = n1 - n0;
            if nd.length() > 0.001 {
                nd = nd.normalized();
            }
            let anchor = rect.min + egui::vec2(rect.width() - 56.0, 10.0);
            let nctr = anchor + egui::vec2(20.0, 20.0);
            painter.circle_filled(nctr, 17.0, egui::Color32::from_white_alpha(210));
            painter.line_segment(
                [nctr - nd * 11.0, nctr + nd * 8.0],
                egui::Stroke::new(2.5, egui::Color32::BLACK),
            );
            painter.text(
                nctr + egui::vec2(0.0, 13.0),
                egui::Align2::CENTER_CENTER,
                "N",
                egui::FontId::proportional(11.0),
                egui::Color32::BLACK,
            );
        }

        self.hover = hover_cell;
        if let Some(pos) = secondary_cell {
            self.remove_at(pos);
        }
        if let Some(pos) = primary_cell {
            let pos = match (
                primary_screen,
                self.snap_to_settlement(pos, primary_screen.unwrap_or(egui::pos2(0.0, 0.0)), center),
            ) {
                (Some(_click), Some(s)) if self.tool == Tool::Select => s,
                _ => pos,
            };
            if self.tool == Tool::Select {
                self.selected = Some(pos);
                self.selected_screen = primary_screen;
                self.sync_edit_buffers();
                self.status = format!("Kachel ({}, {}) ausgewählt – ESC wählt ab.", pos.0, pos.1);
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
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) && self.selected.is_some() {
            self.selected = None;
            self.selected_screen = None;
            self.sync_edit_buffers();
            self.status = "Auswahl aufgehoben (ESC).".to_string();
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
                ui.heading("Karte generieren (2.5D)");
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
                ui.add(egui::Slider::new(&mut self.cfg.dry_south, 0.0..=1.0).text("Trockenheit Süden (Wüste)"));
                ui.add(egui::Slider::new(&mut self.cfg.river_amount, 0.0..=2.0).text("Flussmenge"));
                ui.add(egui::Slider::new(&mut self.cfg.settlement_density, 0.0..=1.0).text("Besiedelung"));
                ui.add(egui::Slider::new(&mut self.cfg.max_elevation_m, 500.0..=6000.0).text("Max. Berghöhe (m)"));
                ui.add(egui::Slider::new(&mut self.cfg.treeline_m, 0.0..=5000.0).text("Baumgrenze (m)"));
                ui.label(format!(
                    "Kartenmaße: {}x{} Kacheln à {:.0} m (14 px) = {:.1} km x {:.1} km = {:.0} km²",
                    self.cfg.width,
                    self.cfg.height,
                    TILE_SIZE_M,
                    self.cfg.width as f32 * TILE_SIZE_M / 1000.0,
                    self.cfg.height as f32 * TILE_SIZE_M / 1000.0,
                    self.cfg.width as f32 * self.cfg.height as f32 * TILE_SIZE_M.powi(2) / 1e6
                ));
                ui.add(egui::Slider::new(&mut self.cfg.climate.south_temp, -40.0..=40.0).text("Temp Süden (°C)"));
                ui.add(egui::Slider::new(&mut self.cfg.climate.north_temp, -40.0..=40.0).text("Temp Norden (°C)"));
                ui.add(egui::Slider::new(&mut self.cfg.climate.lapse_per_unit, 0.0..=100.0).text("Höhen-Lapse"));
                ui.add(egui::Slider::new(&mut self.cfg.climate.snow_temp, -20.0..=10.0).text("Schneegrenze (°C)"));
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
                ui.label("Linksklick = Werkzeug, Rechtsklick = POI/Siedlung entfernen. Klick snapt an Siedlungen.");
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
                ui.heading("Ansicht (2.5D)");
                ui.add(egui::Slider::new(&mut self.view.cell, 4.0..=40.0).text("Zoom (px/Kachel)"));
                ui.add(egui::Slider::new(&mut self.view.rot_deg, 0.0..=360.0).text("Drehung (°)"));
                ui.add(egui::Slider::new(&mut self.view.tilt_deg, 0.0..=90.0).text("Kippwinkel (°)"));
                ui.label("Mausrad = Zoom an Mausposition. Mittlere Maustaste = Karte verschieben. Shift + mittlere Maustaste ziehen: hoch/runter = Kippen 0–90°, links/rechts = Drehung um den Mauszeiger. Q/E = Drehung um Mauszeiger, W/S (oder Pfeile hoch/runter) = Kippen.");
                if ui.button("Ansicht zurücksetzen").clicked() {
                    let e_off = self.view.e_off;
                    let e_scale = self.view.e_scale;
                    self.view = View3D {
                        e_off,
                        e_scale,
                        ..View3D::default()
                    };
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
                    ui.label("— Straße  ● Siedlung (Größe/Farbe = Stufe, blauer Ring = Hafen)  D Dungeon  I Gasthaus  M Händler");
                });

                ui.separator();
                ui.heading("Kachel-Info (Hover)");
                let info_pos = self.hover;
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
                    ui.label(format!("Pos: ({hx}, {hy})"));
                    ui.label(format!(
                        "Biom: {}   Temp: {:.1} °C   Höhe: {:.0} m",
                        t.effective_biome(),
                        t.temperature,
                        (t.elevation - self.cfg.sea_level).max(0.0) * self.cfg.max_elevation_m
                    ));
                    if t.is_water() {
                        ui.label(format!("Wasser-Layer: {:?} – darunter: {}", t.water, t.biome));
                    }
                    ui.label(format!(
                        "Feuchte: {:.2}   Straße: {}   Fluss: {} (Breite {})",
                        t.moisture, t.road, t.river, t.river_width
                    ));
                    if let Some(s) = &t.settlement {
                        let tier_name = tier.map(|t| t.name()).unwrap_or("?");
                        let hafen = if s.has_port { " [Hafen]" } else { "" };
                        ui.label(format!(
                            "Siedlung: {} – {} ({} Einwohner){hafen}",
                            s.name, tier_name, s.population
                        ));
                    }
                    if let Some(p) = &t.poi {
                        ui.label(format!("POI: {p:?}"));
                    }
                } else {
                    ui.label("– Maus über die Karte bewegen –");
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

        if let Some(sel) = self.selected {
            let mut open = true;
            let pos = self.selected_screen.unwrap_or(egui::pos2(220.0, 120.0));
            egui::Window::new(format!("Kachel ({}, {})", sel.0, sel.1))
                .open(&mut open)
                .fixed_pos(pos)
                .show(ui.ctx(), |ui| {
                    let info = self.state.as_ref().and_then(|st| {
                        st.world
                            .get(sel.0, sel.1)
                            .map(|t| (t.clone(), st.world.settlement_tier(sel.0, sel.1)))
                    });
                    if let Some((t, tier)) = info {
                        ui.label(format!(
                            "Biom: {}   Temp: {:.1} °C   Höhe: {:.0} m",
                            t.effective_biome(),
                            t.temperature,
                            (t.elevation - self.cfg.sea_level).max(0.0) * self.cfg.max_elevation_m
                        ));
                        if t.is_water() {
                            ui.label(format!("Wasser-Layer: {:?} – darunter: {}", t.water, t.biome));
                        }
                        ui.label(format!(
                            "Feuchte: {:.2}   Straße: {}   Fluss: {} (Breite {})",
                            t.moisture, t.road, t.river, t.river_width
                        ));
                        if let Some(s) = &t.settlement {
                            ui.label(format!(
                                "Siedlung: {} – {} ({} Einwohner)",
                                tier.map(|t| t.name()).unwrap_or("?"),
                                s.name,
                                s.population
                            ));
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
                                        x: sel.0,
                                        y: sel.1,
                                        name: Some(self.edit_name.clone()),
                                        population: Some(self.edit_pop),
                                    },
                                    &mut self.rng,
                                );
                                self.dirty = true;
                                self.status =
                                    format!("Siedlung aktualisiert – Revision {}", state.revision);
                            }
                            let near_water = self
                                .state
                                .as_ref()
                                .map_or(false, |st| st.world.near_water(sel.0, sel.1));
                            if near_water {
                                let mut port = s.has_port;
                                if ui.checkbox(&mut port, "Hafen").changed() {
                                    let state = self.state.as_mut().unwrap();
                                    state.apply(
                                        Edit::SetPort {
                                            x: sel.0,
                                            y: sel.1,
                                            port,
                                        },
                                        &mut self.rng,
                                    );
                                    self.dirty = true;
                                    self.status = format!(
                                        "Hafen {} – Revision {}",
                                        if port { "an" } else { "aus" },
                                        state.revision
                                    );
                                }
                            }
                        }
                        if let Some(p) = &t.poi {
                            ui.label(format!("POI: {p:?}"));
                        }
                        ui.label("Schließen: X oben rechts oder ESC.");
                    }
                });
            if !open {
                self.selected = None;
                self.selected_screen = None;
                self.sync_edit_buffers();
            }
        }

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
        "dnd-2d-map (2.5D)",
        options,
        Box::new(|_cc| Ok(Box::new(MapApp::default()) as Box<dyn eframe::App>)),
    )
}
