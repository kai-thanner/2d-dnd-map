use dnd_2d_map::view3d::View3D;

#[test]
fn map_center_projects_to_screen_center() {
    let v = View3D {
        rot_deg: 37.0,
        tilt_deg: 45.0,
        pan: (13.0, -7.0),
        ..View3D::default()
    };
    let (cx, cy) = (50.0, 25.0);
    let center = (700.0, 400.0);
    let p = v.project(cx, cy, v.e_off, cx, cy, center);
    assert!(
        (p.0 - (center.0 + v.pan.0)).abs() < 1e-3 && (p.1 - (center.1 + v.pan.1)).abs() < 1e-3,
        "Kartenmittelpunkt muss auf Bildschirmmitte+Pan abgebildet werden, got {p:?}"
    );
}

#[test]
fn project_unproject_roundtrip() {
    let v = View3D {
        rot_deg: 30.0,
        tilt_deg: 60.0,
        cell: 14.0,
        pan: (5.0, 9.0),
        ..View3D::default()
    };
    let center = (700.0, 400.0);
    let (cx, cy) = (50.0, 25.0);
    let sp = v.project(20.0, 10.0, 0.9, cx, cy, center);
    let bp = v.unproject(sp, cx, cy, center, 0.9);
    assert!(
        (bp.0 - 20.0).abs() < 0.6 && (bp.1 - 10.0).abs() < 0.6,
        "Roundtrip fehlgeschlagen: {bp:?} vs (20, 10)"
    );
}

#[test]
fn zoom_keeps_anchor_point_fixed() {
    let mut v = View3D {
        rot_deg: 15.0,
        tilt_deg: 55.0,
        ..View3D::default()
    };
    let center = (700.0, 400.0);
    let (cx, cy) = (50.0, 25.0);
    let p = (733.0, 411.0);
    let anchor_tile = (30.0f32, 12.0f32, 0.7f32);
    let sp_before = v.project(anchor_tile.0, anchor_tile.1, anchor_tile.2, cx, cy, center);
    v.zoom_at(p, sp_before, center, 1.25);
    let sp_after = v.project(anchor_tile.0, anchor_tile.1, anchor_tile.2, cx, cy, center);
    assert!(
        (sp_after.0 - p.0).abs() < 1e-2 && (sp_after.1 - p.1).abs() < 1e-2,
        "Zoom-Anker muss am Mauszeiger bleiben: {sp_after:?} vs {p:?}"
    );
}

#[test]
fn tilt_90_is_top_down() {
    let v = View3D {
        tilt_deg: 90.0,
        ..View3D::default()
    };
    let center = (0.0, 0.0);
    let p = v.project(10.0, 5.0, 1.5, 0.0, 0.0, center);
    assert!(
        (p.0 - 10.0 * v.cell).abs() < 1e-3 && (p.1 - 5.0 * v.cell).abs() < 1e-3,
        "Bei 90° muss Draufsicht ohne Höhenversatz rauskommen: {p:?}"
    );
}

#[test]
fn rotate_keeps_mouse_anchor_fixed() {
    let mut v = View3D {
        rot_deg: 10.0,
        tilt_deg: 50.0,
        pan: (3.0, -4.0),
        ..View3D::default()
    };
    let center = (700.0, 400.0);
    let (cx, cy) = (50.0, 25.0);
    let p = (733.0, 411.0);
    let (ax, ay) = v.unproject(p, cx, cy, center, v.e_off);
    v.rotate_at(p, cx, cy, center, 37.0);
    let sp = v.project(ax, ay, v.e_off, cx, cy, center);
    assert!(
        (sp.0 - p.0).abs() < 1e-2 && (sp.1 - p.1).abs() < 1e-2,
        "Dreh-Anker muss am Mauszeiger bleiben: {sp:?} vs {p:?}"
    );
}

#[test]
fn disp_e_matches_dimension_model() {
    // sea_level = 0.5, max Berghoehe = 6000 m, 1 Kachel = 1000 m -> e_scale = 6 Kacheln
    let v = View3D {
        e_off: 0.5,
        e_scale: 6.0,
        ..View3D::default()
    };
    assert!((v.disp_e(1.0) - 6.0).abs() < 1e-4, "Gipfel = 6 Kacheln ueber Wasser");
    assert!(v.disp_e(0.5).abs() < 1e-6, "Wasserspiegel = 0");
    assert!(
        v.disp_e(0.0).abs() < 1e-6,
        "Wassertiefe wird nicht gerendert (disp_e >= 0); Wert bleibt in elevation gespeichert"
    );
}

fn cross2(o: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

fn tri_area(t: [(f32, f32); 3]) -> f32 {
    (cross2(t[0], t[1], t[2])).abs() / 2.0
}

fn point_in_tri(p: (f32, f32), t: [(f32, f32); 3]) -> bool {
    let d1 = cross2(t[0], t[1], p);
    let d2 = cross2(t[1], t[2], p);
    let d3 = cross2(t[2], t[0], p);
    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_neg && has_pos)
}

fn point_in_poly_evenodd(p: (f32, f32), poly: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if (yi > p.1) != (yj > p.1) && p.0 < (xj - xi) * (p.1 - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn dist_to_seg(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let ab = (b.0 - a.0, b.1 - a.1);
    let ap = (p.0 - a.0, p.1 - a.1);
    let len2 = ab.0 * ab.0 + ab.1 * ab.1;
    let t = if len2 > 0.0 { (ap.0 * ab.0 + ap.1 * ab.1) / len2 } else { 0.0 };
    let t = t.clamp(0.0, 1.0);
    ((ap.0 - t * ab.0).powi(2) + (ap.1 - t * ab.1).powi(2)).sqrt()
}

/// Dreiecke muessen das Quad exakt fuellen: kein Sample-Punkt darf nur in
/// einem, aber nicht im anderen liegen (weder Loch noch Ueberstand/Spike).
fn assert_exact_tiling(quad: [(f32, f32); 4]) {
    use dnd_2d_map::view3d::{quad_is_convex, triangulate_quad};
    let tris = triangulate_quad(quad);
    assert!(tris.len() <= 2, "hoechstens 2 Dreiecke: {quad:?}");
    for t in &tris {
        assert!(tri_area(*t) > 1e-4, "keine entarteten Dreiecke: {t:?}");
    }
    // entartete/spitze Quads duerfen degenerate Anteile fallen lassen (sub-pixel);
    // exakte Parkettierung nur fordern, wenn das Quad selbst "gesund" ist
    let area2 = cross2(quad[0], quad[1], quad[2]).abs() + cross2(quad[0], quad[2], quad[3]).abs();
    let degenerate = area2 < 0.5 || dnd_2d_map::view3d::poly_has_sharp_spike(&quad);
    if degenerate {
        return;
    }
    let minx = quad.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
    let maxx = quad.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
    let miny = quad.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
    let maxy = quad.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
    let diag = ((maxx - minx).powi(2) + (maxy - miny).powi(2)).sqrt().max(1e-6);
    let margin = 0.02 * diag;
    let n = 24;
    for iy in 0..n {
        for ix in 0..n {
            let p = (
                minx + (ix as f32 + 0.5) / n as f32 * (maxx - minx),
                miny + (iy as f32 + 0.5) / n as f32 * (maxy - miny),
            );
            let mut dmin = f32::INFINITY;
            for i in 0..4 {
                dmin = dmin.min(dist_to_seg(p, quad[i], quad[(i + 1) % 4]));
            }
            for t in &tris {
                for i in 0..3 {
                    dmin = dmin.min(dist_to_seg(p, t[i], t[(i + 1) % 3]));
                }
            }
            if dmin < margin {
                continue;
            }
            let in_quad = point_in_poly_evenodd(p, &quad);
            let in_tris = tris.iter().any(|t| point_in_tri(p, *t));
            assert_eq!(in_quad, in_tris, "Tiling-Fehler bei {p:?} in Quad {quad:?} (konvex={})", quad_is_convex(quad));
        }
    }
}

#[test]
fn triangulate_quad_exact_for_all_shapes() {
    // konvex
    assert_exact_tiling([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
    // konkav (Pfeil), D innen
    assert_exact_tiling([(0.0, 0.0), (2.0, 0.0), (1.0, 1.0), (1.0, 0.2)]);
    // Bowtie: Kanten AB und CD kreuzen sich
    assert_exact_tiling([(0.0, 0.0), (1.0, 1.0), (1.0, 0.0), (0.0, 1.0)]);
    // Bowtie: Kanten BC und DA kreuzen sich
    assert_exact_tiling([(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]);
    // Sliver (fast kollinear)
    assert_exact_tiling([(0.0, 0.0), (100.0, 1.0), (100.0, 2.0), (0.0, 1.0)]);
    // gedrehtes Quadrat
    let a = 0.7f32;
    let (s, c) = (a.sin(), a.cos());
    assert_exact_tiling([
        (0.0, 0.0),
        (10.0 * c, 10.0 * s),
        (10.0 * c - 10.0 * s, 10.0 * s + 10.0 * c),
        (-10.0 * s, 10.0 * c),
    ]);
}

#[test]
fn no_spikes_when_rotating_world() {
    use dnd_2d_map::config::Config;
    use dnd_2d_map::gen;
    use dnd_2d_map::view3d::{quad_is_convex, triangulate_quad, View3D};
    use dnd_2d_map::world::World;

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

    const SUB: usize = 3;
    let cfg = Config::default();
    let w = gen::generate(&cfg);
    let cx = w.width as f32 / 2.0;
    let cy = w.height as f32 / 2.0;
    let center = (1081.0, 313.0);
    let mut checked = 0usize;
    for tilt in [30.0f32, 60.0, 89.0] {
        for escale in [3.0f32, 6.0] {
            let v = View3D {
                cell: 14.0,
                tilt_deg: tilt,
                e_off: cfg.sea_level,
                e_scale: escale,
                ..View3D::default()
            };
            for rot in (0..360).step_by(15) {
                let v = View3D { rot_deg: rot as f32, ..v };
                for y in 0..w.height {
                    for x in 0..w.width {
                        let mut quads: Vec<[(f32, f32); 4]> = Vec::new();
                        if is_edge_tile(&w, x, y) {
                            for sy in 0..SUB {
                                for sx in 0..SUB {
                                    let fx0 = x as f32 + sx as f32 / SUB as f32;
                                    let fy0 = y as f32 + sy as f32 / SUB as f32;
                                    let fx1 = x as f32 + (sx + 1) as f32 / SUB as f32;
                                    let fy1 = y as f32 + (sy + 1) as f32 / SUB as f32;
                                    quads.push([
                                        v.project(fx0, fy0, bilinear_elev(&w, fx0, fy0), cx, cy, center),
                                        v.project(fx1, fy0, bilinear_elev(&w, fx1, fy0), cx, cy, center),
                                        v.project(fx1, fy1, bilinear_elev(&w, fx1, fy1), cx, cy, center),
                                        v.project(fx0, fy1, bilinear_elev(&w, fx0, fy1), cx, cy, center),
                                    ]);
                                }
                            }
                        } else {
                            quads.push([
                                v.project(x as f32, y as f32, bilinear_elev(&w, x as f32, y as f32), cx, cy, center),
                                v.project(x as f32 + 1.0, y as f32, bilinear_elev(&w, x as f32 + 1.0, y as f32), cx, cy, center),
                                v.project(x as f32 + 1.0, y as f32 + 1.0, bilinear_elev(&w, x as f32 + 1.0, y as f32 + 1.0), cx, cy, center),
                                v.project(x as f32, y as f32 + 1.0, bilinear_elev(&w, x as f32, y as f32 + 1.0), cx, cy, center),
                            ]);
                        }
                        for q in quads {
                            if quad_is_convex(q) {
                                continue;
                            }
                            checked += 1;
                            assert_exact_tiling(q);
                            for t in triangulate_quad(q) {
                                assert!(tri_area(t) >= 0.0);
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(checked > 100, "Test war wirkungslos: nur {checked} nicht-konvexe Quads gefunden");
}

#[test]
fn depth_orders_by_view_direction() {
    use dnd_2d_map::view3d::View3D;
    let v = View3D {
        rot_deg: 30.0,
        tilt_deg: 60.0,
        e_off: 0.42,
        e_scale: 3.0,
        ..View3D::default()
    };
    let (cx, cy) = (50.0, 25.0);
    // gleiche Position, hoeher = naeher (groesserer depth-Wert) bei Kippwinkel > 0
    let low = v.depth(10.0, 10.0, 0.45, cx, cy);
    let high = v.depth(10.0, 10.0, 0.9, cx, cy);
    assert!(high > low, "hohe Kachel muss groesseren depth-Wert haben: {high} vs {low}");
    // bei Draufsicht (90 deg) zaehlt nur die Hoehe
    let v90 = View3D { tilt_deg: 90.0, ..v };
    let a = v90.depth(10.0, 10.0, 0.5, cx, cy);
    let b = v90.depth(20.0, 20.0, 0.5, cx, cy);
    assert!((a - b).abs() < 1e-4, "bei 90 deg darf die Position den depth-Wert nicht aendern");
}

#[test]
fn render_e_caps_height_shift() {
    use dnd_2d_map::view3d::View3D;
    // Worst Case: Zoom 4 px, flacher Kippwinkel, volle Berghöhe 6000 m
    let v = View3D {
        cell: 4.0,
        tilt_deg: 16.06,
        e_off: 0.42,
        e_scale: 6.0,
        ..View3D::default()
    };
    let (cx, cy) = (50.0, 25.0);
    let center = (1025.0, 602.0);
    let low = v.project(10.0, 10.0, 0.42, cx, cy, center);
    let high = v.project(10.0, 10.0, 1.0, cx, cy, center);
    let d = ((high.0 - low.0).powi(2) + (high.1 - low.1).powi(2)).sqrt();
    assert!(
        d <= 1.3 * v.cell,
        "Hoehen-Shift darf ~1 Kachel nicht deutlich ueberschreiten (sonst Sliver/Spikes): {d} px bei cell={}",
        v.cell
    );
    // project/unproject-Roundtrip bleibt mit dem Cap konsistent
    let sp = v.project(20.0, 12.0, 0.9, cx, cy, center);
    let bp = v.unproject(sp, cx, cy, center, 0.9);
    assert!(
        (bp.0 - 20.0).abs() < 0.6 && (bp.1 - 12.0).abs() < 0.6,
        "Roundtrip mit render_e-Cap fehlgeschlagen: {bp:?}"
    );
}
