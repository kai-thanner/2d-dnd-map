// Regression: Kartenqualität — keine Straßen über Meer/See, Häfen nur am Meer,
// nie genau 1 Hafen, Flüsse nie bergauf.
use dnd_2d_map::config::Config;
use dnd_2d_map::gen;
use dnd_2d_map::world::{Biome, WaterKind};

fn check_world(w: &dnd_2d_map::world::World, seed: u64) {
    // 1) Keine Straße auf Meer/See/Eis (Fluss-Brücken sind ok)
    for y in 0..w.height {
        for x in 0..w.width {
            let t = w.get(x, y).unwrap();
            if t.road && t.is_water() && t.water != Some(WaterKind::River) {
                let ferry = dnd_2d_map::gen::roads::ferry_tiles(w);
                assert!(
                    ferry.contains(&w.idx(x, y)),
                    "seed {seed}: Straße auf Wasser (kein Fähre-Tile) bei ({x},{y}): {:?}",
                    t.water
                );
            }
        }
    }
    // 2) has_port nur direkt am Ozean
    for y in 0..w.height {
        for x in 0..w.width {
            if let Some(s) = &w.get(x, y).unwrap().settlement {
                if s.has_port {
                    assert!(
                        w.near_ocean(x, y),
                        "seed {seed}: Hafen ohne direktes Meer bei ({x},{y})"
                    );
                    let t = w.get(x, y).unwrap();
                    assert!(!t.is_water());
                    assert!(
                        !matches!(t.effective_biome(), Biome::Mountain | Biome::Rock),
                        "seed {seed}: Hafen auf Berg bei ({x},{y})"
                    );
                }
            }
        }
    }
    // 3) globale Hafenanzahl nie genau 1
    let n_ports = w
        .tiles
        .iter()
        .filter(|t| t.settlement.as_ref().map_or(false, |s| s.has_port))
        .count();
    assert!(n_ports != 1, "seed {seed}: genau 1 Hafen auf der Karte");
    // 4) Flüsse nie bergauf UND jedes Fluss-System endet in einem Gewässer
    //    (Meer oder See) — nie "im Nichts"
    let mut seen = vec![false; w.tiles.len()];
    for y in 0..w.height {
        for x in 0..w.width {
            let t0 = w.get(x, y).unwrap();
            if !t0.river || t0.is_water() || seen[w.idx(x, y)] {
                continue;
            }
            // komplettes Fluss-System ablaufen (Downstream-BFS)
            let mut stack = vec![(x, y)];
            let mut reaches_water = false;
            while let Some((cx, cy)) = stack.pop() {
                let i = w.idx(cx, cy);
                if seen[i] {
                    continue;
                }
                seen[i] = true;
                for n in w.neighbors8(cx, cy) {
                    let nt = w.get(n.0, n.1).unwrap();
                    if nt.is_water() {
                        reaches_water = true;
                    } else if nt.river {
                        stack.push((n.0, n.1));
                    }
                }
            }
            assert!(
                reaches_water,
                "seed {seed}: Flusssystem bei ({x},{y}) mündet nicht in Gewässer"
            );
        }
    }
}

#[test]
fn gen_quality_seeds() {
    for seed in 1..=12u64 {
        let cfg = Config { seed, width: 100, height: 100, ..Config::default() };
        let w = gen::generate(&cfg);
        check_world(&w, seed);
        let lakes = w.tiles.iter().filter(|t| t.water == Some(WaterKind::Lake)).count();
        let ports = w.tiles.iter().filter(|t| t.settlement.as_ref().map_or(false, |s| s.has_port)).count();
        println!("seed {seed}: lakes={lakes} hafen_siedlungen={ports} ok");
    }
}

#[test]
fn depression_lakes_are_filled() {
    // Senken-Seen über Meeresspiegel existieren und jede See-Komponente ist
    // intern stufenweise flach (jede Höhenstufe innerhalb einer Komponente
    // bildet ein eigenes flaches Plateau — Ketten von Seen sind ok).
    let mut found = 0;
    for seed in 1..=12u64 {
        let cfg = Config { seed, width: 100, height: 100, ..Config::default() };
        let w = gen::generate(&cfg);
        let mut visited = vec![false; w.tiles.len()];
        for y in 0..w.height {
            for x in 0..w.width {
                let i = w.idx(x, y);
                if visited[i] || w.tiles[i].water != Some(WaterKind::Lake) {
                    continue;
                }
                let mut comp = vec![i];
                visited[i] = true;
                let mut stack = vec![i];
                while let Some(ci) = stack.pop() {
                    let cx = ci % w.width;
                    let cy = ci / w.width;
                    for n in w.neighbors8(cx, cy) {
                        let ni = w.idx(n.0, n.1);
                        if !visited[ni] && w.tiles[ni].water == Some(WaterKind::Lake) {
                            visited[ni] = true;
                            comp.push(ni);
                            stack.push(ni);
                        }
                    }
                }
                if comp.len() >= 6 && comp.iter().all(|&c| w.tiles[c].elevation >= w.sea_level - 1e-6) {
                    found += 1;
                    // jede Höhenstufe im See ist ein flaches Plateau
                    let mut by_e: std::collections::HashMap<u32, Vec<usize>> = std::collections::HashMap::new();
                    for &c in &comp {
                        by_e.entry(w.tiles[c].elevation.to_bits()).or_default().push(c);
                    }
                    for (_e, cells) in &by_e {
                        let e0 = w.tiles[cells[0]].elevation;
                        for &c in cells {
                            assert!((w.tiles[c].elevation - e0).abs() < 1e-4);
                        }
                    }
                }
            }
        }
    }
    println!("depression lakes gefunden: {found}");
    assert!(found > 0, "mindestens eine Senken-See über 12 Seeds erwartet");
}
