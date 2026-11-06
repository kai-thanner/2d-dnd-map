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
