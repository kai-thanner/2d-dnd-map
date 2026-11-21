use dnd_2d_map::config::Config;
use dnd_2d_map::gen;
use dnd_2d_map::terrain3d::{build_terrain_mesh, mat4_mul, mat4_transform, orbit_camera, perspective};

#[test]
fn mesh_counts_and_index_range() {
    let cfg = Config { seed: 3, width: 100, height: 100, ..Config::default() };
    let w = gen::generate(&cfg);
    let mesh = build_terrain_mesh(&w, w.sea_level, cfg.max_elevation_m / 1000.0, 1);
    assert_eq!(mesh.vertices.len(), 101 * 101);
    assert_eq!(mesh.indices.len(), 100 * 100 * 6);
    let max_idx = *mesh.indices.iter().max().unwrap();
    assert!(max_idx < mesh.vertices.len() as u32);
    // Mesh zentriert um (0,0)
    let xs: Vec<f32> = mesh.vertices.iter().map(|v| v.pos[0]).collect();
    assert!((xs.iter().cloned().fold(f32::INFINITY, f32::min) + 50.0).abs() < 1e-3);
    // Normalen zeigen überwiegend nach oben (z > 0)
    let up = mesh.vertices.iter().filter(|v| v.normal[2] > 0.5).count();
    assert!(up > mesh.vertices.len() * 9 / 10, "zu viele schiefe Normalen: {up}");
    // Höhen im erwarteten Bereich
    let zmax = mesh.vertices.iter().map(|v| v.pos[2]).fold(f32::NEG_INFINITY, f32::max);
    assert!(zmax <= 64.0 && zmax >= 0.0, "zmax={zmax}");
}

#[test]
fn camera_top_down_and_horizon() {
    // Draufsicht (tilt 90): Auge direkt über Zentrum, Zentrum landet in NDC-Mitte
    let (vp, eye) = orbit_camera(0.0, 90.0, 200.0, [0.0, 0.0, 0.0], 1.5, 141.0);
    assert!((eye[0]).abs() < 1.0 && (eye[1]).abs() < 1.0 && (eye[2] - 200.0).abs() < 1.0);
    let clip = mat4_transform(&vp, [0.0, 0.0, 0.0, 1.0]);
    let ndc = [clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]];
    assert!(ndc[0].abs() < 1e-3 && ndc[1].abs() < 1e-3, "Zentrum nicht in Bildmitte: {ndc:?}");
    assert!(ndc[2] > 0.0 && ndc[2] < 1.0, "Zentrum nicht im Tiefenbereich: {ndc:?}");
    // Horizont (tilt 1°): Auge fast auf Kartenhöhe, weit seitlich
    let (_, eye2) = orbit_camera(0.0, 1.0, 200.0, [0.0, 0.0, 0.0], 1.5, 141.0);
    assert!(eye2[2] < 5.0, "bei tilt=1° darf das Auge nicht hoch sein: {eye2:?}");
    assert!(eye2[1] > 190.0);
}

#[test]
fn projection_monotonic_depth() {
    let p = perspective(50.0, 1.0, 1.0, 100.0);
    let near = mat4_transform(&p, [0.0, 0.0, -1.0, 1.0]);
    let far = mat4_transform(&p, [0.0, 0.0, -100.0, 1.0]);
    assert!(near[2] / near[3] < far[2] / far[3], "near muss kleinere Clip-Z als far haben");
}

#[test]
fn rotation_360_is_identity() {
    let (vp0, _) = orbit_camera(0.0, 60.0, 200.0, [0.0, 0.0, 0.0], 1.5, 141.0);
    let (vp360, _) = orbit_camera(360.0, 60.0, 200.0, [0.0, 0.0, 0.0], 1.5, 141.0);
    for c in 0..4 {
        for r in 0..4 {
            assert!((vp0[c][r] - vp360[c][r]).abs() < 1e-3);
        }
    }
    let _ = mat4_mul(&vp0, &perspective(50.0, 1.5, 4.0, 1000.0));
}

#[test]
fn wgsl_shader_parst() {
    // SHADER ist privat — über include_str nicht erreichbar; deshalb hier
    // gespiegelt: der Shader-String wird über eine pub Konstante geteilt.
    let src = dnd_2d_map::terrain3d::SHADER_WGSL;
    let module = naga::front::wgsl::parse_str(src).expect("WGSL muss parsen");
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator.validate(&module).expect("WGSL muss validieren");
}
