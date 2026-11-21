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
                panic!("seed {seed}: Straße auf Wasser bei ({x},{y}): {:?}", t.water);
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
    // 4) Flüsse nie bergauf: jedes Flusstile hat einen tieferen Fluss-Nachbarn
    //    oder mündet in Gewässer
    for y in 0..w.height {
        for x in 0..w.width {
            let t = w.get(x, y).unwrap();
            if !t.river || t.is_water() {
                continue;
            }
            let e = t.elevation;
            let ok = w.neighbors8(x, y).iter().any(|n| {
                let nt = w.get(n.0, n.1).unwrap();
                (nt.river && nt.elevation < e) || nt.is_water()
            });
            assert!(ok, "seed {seed}: Fluss endet bergauf bei ({x},{y}) e={e}");
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
fn depression_lakes_are_filled_flat() {
    // Mindestens ein Seed mit Senken-See: Seefläche über Meeresspiegel muss
    // es geben können; prüfe, dass Seen flache Oberfläche haben (gleiche Höhe in Komponente)
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
                // Komponenten unter Meeresspiegel haben naturgemäß variable Tiefe —
                // nur reine Senken-Seen (alle Zellen >= Meeresspiegel) prüfen
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
                    let e0 = w.tiles[comp[0]].elevation;
                    for &c in &comp {
                        assert!(
                            (w.tiles[c].elevation - e0).abs() < 1e-4,
                            "seed {seed}: See nicht flach bei ({},{})",
                            c % w.width,
                            c / w.width
                        );
                    }
                }
            }
        }
    }
    println!("depression lakes gefunden: {found}");
}
