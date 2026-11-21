// Regression-Test: echtes egui-Tessellating headless ueber Tilt/Zoom/Dreh-Sweep.
// Stellt sicher, dass keine Dreiecke mit riesigen Koordinaten entstehen
// (Miter-Join-Artefakte in epaint -> sichtbare Spikes/Linien bei flachem
// Kippwinkel) und dass die Kachel-Geometrie im erwarteten Rahmen bleibt.
use dnd_2d_map::config::Config;
use dnd_2d_map::gen;
use dnd_2d_map::palette;
use dnd_2d_map::view3d::{poly_has_sharp_spike, quad_is_convex, triangulate_quad, View3D};
use dnd_2d_map::world::{Biome, World};
use eframe::egui;

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
    (e00 + (e10 - e00) * tx) + ((e01 + (e11 - e01) * tx) - (e00 + (e10 - e00) * tx)) * ty
}

#[test]
fn egui_tessellation_stays_bounded() {
    let cfg = Config { seed: 3, width: 100, height: 100, ..Config::default() };
    let w = gen::generate(&cfg);
    let (sw, sh) = (666usize, 461usize);
    let mut total_bad = 0;
    for tilt in [20.0f32, 45.0, 52.0, 60.0] {
        for cell in [4.0f32, 12.658, 36.0] {
            for rot in [0.0f32, 137.0] {
                let v = View3D { cell, tilt_deg: tilt, rot_deg: rot, e_off: cfg.sea_level, e_scale: cfg.max_elevation_m / 1000.0, ..View3D::default() };
                let ctx = egui::Context::default();
                let cx = w.width as f32 / 2.0;
                let cy = w.height as f32 / 2.0;
                let mut input = egui::RawInput::default();
                input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(sw as f32, sh as f32)));
                let mut out = ctx.run_ui(input, |ui| {
                    egui::CentralPanel::default().show(ui, |ui: &mut egui::Ui| {
                        let rect = ui.available_rect_before_wrap();
                        let center = (rect.center().x, rect.center().y);
                        let painter = ui.painter_at(rect);
                        let proj = |x: f32, y: f32, e: f32| { let sp = v.project(x, y, e, cx, cy, center); egui::pos2(sp.0, sp.1) };
                        let mut tiles: Vec<(f32, f32, usize, usize)> = Vec::new();
                        for y in 0..w.height { for x in 0..w.width {
                            let key = v.depth(x as f32 + 0.5, y as f32 + 0.5, bilinear_elev(&w, x as f32 + 0.5, y as f32 + 0.5), cx, cy);
                            tiles.push((key, x as f32, x, y));
                        }}
                        tiles.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)));
                        let quad = |pts: [egui::Pos2; 4], col: egui::Color32| {
                            let raw = [(pts[0].x, pts[0].y), (pts[1].x, pts[1].y), (pts[2].x, pts[2].y), (pts[3].x, pts[3].y)];
                            let stroke = |poly: &[(f32, f32)]| {
                                if poly_has_sharp_spike(poly) { egui::epaint::PathStroke::NONE } else { egui::epaint::PathStroke::new(0.7, col).inside() }
                            };
                            let a2 = (raw[1].0-raw[0].0)*(raw[2].1-raw[0].1) - (raw[1].1-raw[0].1)*(raw[2].0-raw[0].0);
                            let a2b = (raw[2].0-raw[0].0)*(raw[3].1-raw[0].1) - (raw[2].1-raw[0].1)*(raw[3].0-raw[0].0);
                            if a2.abs() + a2b.abs() < 0.5 || poly_has_sharp_spike(&raw) {
                                for tri in triangulate_quad(raw) {
                                    painter.add(egui::Shape::convex_polygon(tri.iter().map(|p| egui::pos2(p.0, p.1)).collect(), col, stroke(&tri)));
                                }
                                return;
                            }
                            if quad_is_convex(raw) {
                                painter.add(egui::Shape::convex_polygon(pts.to_vec(), col, stroke(&raw)));
                            } else {
                                for tri in triangulate_quad(raw) {
                                    painter.add(egui::Shape::convex_polygon(tri.iter().map(|p| egui::pos2(p.0, p.1)).collect(), col, stroke(&tri)));
                                }
                            }
                        };
                        for (_, _, x, y) in &tiles {
                            let (x, y) = (*x, *y);
                            let t = w.get(x, y).unwrap();
                            if cell >= 8.0 {
                                for sy in 0..SUB { for sx in 0..SUB {
                                    let fx0 = x as f32 + sx as f32 / SUB as f32;
                                    let fy0 = y as f32 + sy as f32 / SUB as f32;
                                    let fx1 = x as f32 + (sx + 1) as f32 / SUB as f32;
                                    let fy1 = y as f32 + (sy + 1) as f32 / SUB as f32;
                                    let e_sub = bilinear_elev(&w, (fx0+fx1)*0.5, (fy0+fy1)*0.5);
                                    let (r, g, b) = if e_sub < w.sea_level { palette::biome_rgb(Biome::Ocean) } else { palette::shade_rgb(&w, x, y, palette::biome_rgb(t.biome)) };
                                    quad([proj(fx0,fy0,bilinear_elev(&w,fx0,fy0)), proj(fx1,fy0,bilinear_elev(&w,fx1,fy0)), proj(fx1,fy1,bilinear_elev(&w,fx1,fy1)), proj(fx0,fy1,bilinear_elev(&w,fx0,fy1))], egui::Color32::from_rgb(r, g, b));
                                }}
                            } else {
                                let (r, g, b) = palette::shade_rgb(&w, x, y, palette::biome_rgb(t.effective_biome()));
                                quad([proj(x as f32,y as f32,bilinear_elev(&w,x as f32,y as f32)), proj(x as f32+1.0,y as f32,bilinear_elev(&w,x as f32+1.0,y as f32)), proj(x as f32+1.0,y as f32+1.0,bilinear_elev(&w,x as f32+1.0,y as f32+1.0)), proj(x as f32,y as f32+1.0,bilinear_elev(&w,x as f32,y as f32+1.0))], egui::Color32::from_rgb(r, g, b));
                            }
                        }
                    });
                });
                let prims = ctx.tessellate(std::mem::take(&mut out.shapes), out.pixels_per_point.max(1.0));
                out.textures_delta.clear();
                let mut bad = 0;
                for p in &prims {
                    if let egui::epaint::Primitive::Mesh(m) = &p.primitive {
                        for v in &m.vertices {
                            if v.pos.x.abs() > 5000.0 || v.pos.y.abs() > 5000.0 { bad += 1; break; }
                        }
                    }
                }
                if bad > 0 {
                    println!("BAD tilt={tilt} cell={cell} rot={rot}: {bad} shapes with huge vertices");
                }
                total_bad += bad;
            }
        }
    }
    println!("total bad shapes over whole sweep: {total_bad}");
    assert_eq!(total_bad, 0, "keine explodierenden Dreiecke erlaubt");
}
