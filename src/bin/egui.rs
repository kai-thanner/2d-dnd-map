use dnd_2d_map::config::Config;
use dnd_2d_map::gen;
use dnd_2d_map::palette;
use dnd_2d_map::world::World;
use eframe::egui;

struct View3D {
    cell: f32,
    pan: egui::Vec2,
    rot: f32,
    hf: f32,
    tilt: f32,
    e_off: f32,
    e_scale: f32,
}

impl View3D {
    fn disp_e(&self, e: f32) -> f32 {
        ((e - self.e_off) * self.e_scale).clamp(0.0, 1.5)
    }

    fn project(&self, x: f32, y: f32, e: f32, origin: egui::Pos2) -> egui::Pos2 {
        let e = self.disp_e(e);
        let (ts, tc) = self.tilt.sin_cos();
        let ox = x + e * 0.35 * ts;
        let oy = y * tc - e * self.hf;
        let (s, c) = self.rot.sin_cos();
        origin + self.pan + egui::vec2((ox * c - oy * s) * self.cell, (ox * s + oy * c) * self.cell)
    }

    fn unproject(&self, p: egui::Pos2, origin: egui::Pos2) -> (f32, f32) {
        let v = (p - origin - self.pan) / self.cell;
        let (s, c) = self.rot.sin_cos();
        let rx = v.x * c + v.y * s;
        let ry = -v.x * s + v.y * c;
        let tc = self.tilt.cos().max(0.15);
        (rx, ry / tc)
    }
}

struct MapApp25D {
    cfg: Config,
    world: Option<World>,
    view: View3D,
    hover: Option<(usize, usize)>,
    status: String,
}

impl Default for MapApp25D {
    fn default() -> Self {
        Self {
            cfg: Config::default(),
            world: None,
            view: View3D {
                cell: 14.0,
                pan: egui::vec2(60.0, 40.0),
                rot: 0.0,
                hf: 2.0,
                tilt: 0.5,
                e_off: 0.0,
                e_scale: 1.0,
            },
            hover: None,
            status: "2.5D-Testansicht – Generieren drücken. Mausrad = Zoom, mittlere Taste = Pan, Q/E = Drehen.".to_string(),
        }
    }
}

impl MapApp25D {
    fn generate(&mut self) {
        let world = gen::generate(&self.cfg);
        let sea = self.cfg.sea_level;
        let max_e = world.tiles.iter().map(|t| t.elevation).fold(0.0f32, f32::max);
        self.view.e_off = sea;
        self.view.e_scale = 1.0 / (max_e - sea).max(0.01);
        self.world = Some(world);
        self.status = format!("Karte generiert (Seed {})", self.cfg.seed);
    }

    fn tile_at(&self, p: egui::Pos2, origin: egui::Pos2, rect: egui::Rect) -> Option<(usize, usize)> {
        let w = self.world.as_ref()?;
        let (rx, ry) = self.view.unproject(p, origin);
        let bx = rx.round() as i64;
        let by = ry.round() as i64;
        let mut best: Option<((usize, usize), f32)> = None;
        for dy in -2..=2i64 {
            for dx in -2..=2i64 {
                let (cx, cy) = (bx + dx, by + dy);
                if cx < 0 || cy < 0 || cx >= w.width as i64 || cy >= w.height as i64 {
                    continue;
                }
                let e = w.get(cx as usize, cy as usize).unwrap().elevation;
                let sp = self.view.project(cx as f32, cy as f32, e, origin);
                let d = sp.distance(p);
                if rect.contains(sp) && best.map_or(true, |(_, bd)| d < bd) {
                    best = Some(((cx as usize, cy as usize), d));
                }
            }
        }
        best.filter(|(_, d)| *d < self.view.cell * 0.8).map(|(t, _)| t)
    }

    fn draw_map(&mut self, ui: &mut egui::Ui) {
        let Some(world) = &self.world else {
            ui.centered_and_justified(|ui| {
                ui.label("Noch keine Karte – links „Generieren“ drücken.");
            });
            return;
        };
        let avail = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(avail, egui::Sense::click_and_drag());
        let rect = response.rect;
        let origin = rect.min;

        let (middle_down, delta) = ui.input(|i| {
            (
                i.pointer.button_down(egui::PointerButton::Middle),
                i.pointer.delta(),
            )
        });
        if middle_down && response.hovered() {
            self.view.pan += delta;
        }
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            if let Some(p) = ui.input(|i| i.pointer.latest_pos()) {
                if rect.contains(p) {
                    let factor = (scroll * 0.0016).exp();
                    let new_cell = (self.view.cell * factor).clamp(4.0, 40.0);
                    if let Some(t) = self.tile_at(p, origin, rect) {
                        let e = self.world.as_ref().unwrap().get(t.0, t.1).unwrap().elevation;
                        let sp = self.view.project(t.0 as f32 + 0.5, t.1 as f32 + 0.5, e, origin);
                        self.view.pan += p - sp;
                    }
                    self.view.cell = new_cell;
                    if !self.view.pan.is_finite() || !self.view.cell.is_finite() {
                        self.view.pan = egui::vec2(60.0, 40.0);
                        self.view.cell = 14.0;
                    }
                }
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::Q)) {
            self.view.rot -= 0.12;
        }
        if ui.input(|i| i.key_pressed(egui::Key::E)) {
            self.view.rot += 0.12;
        }

        let painter = ui.painter_at(rect);
        let cell = self.view.cell;

        let mut hover_cell = None;
        if let Some(p) = ui.input(|i| i.pointer.latest_pos()) {
            if rect.contains(p) {
                hover_cell = self.tile_at(p, origin, rect);
            }
        }

        for y in 0..world.height {
            for x in 0..world.width {
                let t = world.get(x, y).unwrap();
                let e = t.elevation;
                let sp = self.view.project(x as f32, y as f32, e, origin);
                if sp.x < rect.min.x - cell || sp.y < rect.min.y - cell
                    || sp.x > rect.max.x + cell || sp.y > rect.max.y + cell
                {
                    continue;
                }
                let r = egui::Rect::from_min_size(sp, egui::vec2(cell, cell));
                let (r8, g8, b8) = palette::shade_rgb(world, x, y, palette::biome_rgb(t.effective_biome()));
                painter.rect_filled(r, 0.0, egui::Color32::from_rgb(r8, g8, b8));
                if let Some(tier) = world.settlement_tier(x, y) {
                    painter.text(
                        r.center(),
                        egui::Align2::CENTER_CENTER,
                        tier.marker().to_string(),
                        egui::FontId::proportional(cell * 0.6),
                        egui::Color32::BLACK,
                    );
                } else if t.road {
                    painter.rect_filled(r, 0.0, egui::Color32::from_rgba_premultiplied(122, 84, 44, 160));
                }
            }
        }

        if let Some(pos) = hover_cell {
            let t = world.get(pos.0, pos.1).unwrap();
            let sp = self.view.project(pos.0 as f32, pos.1 as f32, t.elevation, origin);
            let r = egui::Rect::from_min_size(sp, egui::vec2(cell, cell));
            painter.rect_filled(r, 0.0, egui::Color32::from_white_alpha(70));
        }
        self.hover = hover_cell;

        let n0 = self.view.project(0.0, 0.0, 0.0, origin);
        let n1 = self.view.project(0.0, -1.0, 0.0, origin);
        let mut nd = n1 - n0;
        if nd.length() > 0.001 {
            nd = nd.normalized();
        }
        let anchor = rect.min + egui::vec2(rect.width() - 56.0, 10.0);
        let c = anchor + egui::vec2(20.0, 20.0);
        painter.circle_filled(c, 17.0, egui::Color32::from_white_alpha(210));
        painter.line_segment([c - nd * 11.0, c + nd * 8.0], egui::Stroke::new(2.5, egui::Color32::BLACK));
        painter.text(
            c + egui::vec2(0.0, 13.0),
            egui::Align2::CENTER_CENTER,
            "N",
            egui::FontId::proportional(11.0),
            egui::Color32::BLACK,
        );
    }
}

impl eframe::App for MapApp25D {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::containers::Panel::left("sidebar").default_size(240.0).show(ui, |ui| {
            ui.heading("2.5D-Test (oblique)");
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
            ui.add(egui::Slider::new(&mut self.cfg.dry_south, 0.0..=1.0).text("Trockenheit Süden"));
            ui.add(egui::Slider::new(&mut self.cfg.settlement_density, 0.0..=1.0).text("Besiedelung"));
            if ui.add_sized([220.0, 30.0], egui::Button::new("▶ Generieren")).clicked() {
                self.generate();
            }
            ui.separator();
            ui.heading("Ansicht");
            ui.add(egui::Slider::new(&mut self.view.cell, 4.0..=40.0).text("Zoom (px/Kachel)"));
            ui.add(egui::Slider::new(&mut self.view.rot, 0.0..=std::f32::consts::TAU).text("Drehung (rad)"));
            ui.add(egui::Slider::new(&mut self.view.hf, 0.0..=3.0).text("Höhenfaktor"));
            ui.add(egui::Slider::new(&mut self.view.tilt, 0.0..=1.35).text("Kippwinkel (rad)"));
            if ui.button("Ansicht zurücksetzen").clicked() {
                self.view = View3D {
                    cell: 14.0,
                    pan: egui::vec2(60.0, 40.0),
                    rot: 0.0,
                    hf: 2.0,
                    tilt: 0.5,
                    e_off: self.view.e_off,
                    e_scale: self.view.e_scale,
                };
            }
            ui.separator();
            ui.heading("Kachel (Hover)");
            if let (Some((hx, hy)), Some(w)) = (self.hover, &self.world) {
                let t = w.get(hx, hy).unwrap();
                ui.label(format!("Pos: ({hx}, {hy})"));
                ui.label(format!("Biom: {}  Temp: {:.1} °C", t.effective_biome(), t.temperature));
                ui.label(format!("Höhe: {:.3}  Feuchte: {:.2}", t.elevation, t.moisture));
            } else {
                ui.label("– Maus über die Karte –");
            }
        });

        egui::CentralPanel::default().show(ui, |ui| {
            self.draw_map(ui);
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
        "dnd-2d-map 2.5D",
        options,
        Box::new(|_cc| Ok(Box::new(MapApp25D::default()) as Box<dyn eframe::App>)),
    )
}
