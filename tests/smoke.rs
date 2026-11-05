use dnd_2d_map::config::{Climate, Config};
use dnd_2d_map::edit::{Edit, GameState};
use dnd_2d_map::gen;
use dnd_2d_map::rng;
use dnd_2d_map::save;
use dnd_2d_map::world::{Biome, Poi, SettlementTier, World};

fn test_cfg(seed: u64) -> Config {
    Config {
        seed,
        width: 80,
        height: 40,
        ..Config::default()
    }
}

fn first_free_land(w: &World) -> (usize, usize) {
    for y in 0..w.height {
        for x in 0..w.width {
            let t = w.get(x, y).unwrap();
            if t.is_passable() && t.settlement.is_none() && t.poi.is_none() {
                return (x, y);
            }
        }
    }
    panic!("kein freies Land gefunden");
}

#[test]
fn same_seed_same_world() {
    let cfg = test_cfg(42);
    let a = gen::generate(&cfg);
    let b = gen::generate(&cfg);
    assert_eq!(a, b);
}

#[test]
fn different_seed_different_world() {
    let a = gen::generate(&test_cfg(1));
    let b = gen::generate(&test_cfg(2));
    assert_ne!(a, b);
}

#[test]
fn north_is_colder_than_south() {
    let w = gen::generate(&test_cfg(7));
    let avg = |y0: usize, y1: usize| -> f32 {
        let mut s = 0.0;
        let mut n = 0;
        for y in y0..y1 {
            for x in 0..w.width {
                s += w.get(x, y).unwrap().temperature;
                n += 1;
            }
        }
        s / n as f32
    };
    let north = avg(0, 5);
    let south = avg(w.height - 5, w.height);
    assert!(
        north < south,
        "Norden ({north}°C) sollte kälter sein als Süden ({south}°C)"
    );
}

#[test]
fn freezing_everything_becomes_snow_or_ice() {
    let cfg = Config {
        seed: 3,
        width: 40,
        height: 20,
        climate: Climate {
            south_temp: -10.0,
            north_temp: -20.0,
            lapse_per_unit: 55.0,
        },
        ..Config::default()
    };
    let w = gen::generate(&cfg);
    for y in 0..w.height {
        for x in 0..w.width {
            let t = w.get(x, y).unwrap();
            if t.is_water() {
                assert!(
                    matches!(t.effective_biome(), Biome::Ice),
                    "Wasser bei {}°C muss Ice sein, ist {:?}",
                    t.temperature,
                    t.biome
                );
            } else {
                assert!(
                    matches!(t.effective_biome(), Biome::Snow),
                    "Land bei {}°C muss Snow sein, ist {:?}",
                    t.temperature,
                    t.biome
                );
            }
        }
    }
}

#[test]
fn save_load_roundtrip() {
    let cfg = test_cfg(9);
    let world = gen::generate(&cfg);
    let mut state = GameState::new(world);
    let mut rng = rng::from_seed(cfg.seed);
    let (lx, ly) = first_free_land(&state.world);
    state.apply(Edit::AddDungeon { x: lx, y: ly }, &mut rng);
    let mut path = std::env::temp_dir();
    path.push(format!(
        "dnd2dmap-test-{}-{}.json",
        cfg.seed,
        std::process::id()
    ));
    save::save(&path, &cfg, &state).unwrap();
    let (cfg2, state2) = save::load(&path).unwrap();
    std::fs::remove_file(&path).ok();
    assert_eq!(cfg.seed, cfg2.seed);
    assert_eq!(state, state2);
}

#[test]
fn edit_add_and_remove_dungeon() {
    let cfg = test_cfg(11);
    let world = gen::generate(&cfg);
    let mut state = GameState::new(world);
    let mut rng = rng::from_seed(1);
    let (x, y) = first_free_land(&state.world);
    state.apply(Edit::AddDungeon { x, y }, &mut rng);
    assert!(matches!(
        state.world.get(x, y).unwrap().poi,
        Some(Poi::Dungeon { .. })
    ));
    assert_eq!(state.revision, 1);
    state.apply(Edit::RemoveDungeon { x, y }, &mut rng);
    assert!(state.world.get(x, y).unwrap().poi.is_none());
    assert_eq!(state.revision, 2);
    assert_eq!(state.log.len(), 2);
}

#[test]
fn road_edit_connects_points() {
    let cfg = test_cfg(13);
    let world = gen::generate(&cfg);
    let mut state = GameState::new(world);
    let mut rng = rng::from_seed(1);
    let a = first_free_land(&state.world);
    let mut b = a;
    for y in 0..state.world.height {
        for x in 0..state.world.width {
            let t = state.world.get(x, y).unwrap();
            if t.is_passable() && World::dist((x, y), a) > 15.0 {
                b = (x, y);
            }
        }
    }
    state.apply(Edit::AddRoad { from: a, to: b }, &mut rng);
    let w = &state.world;
    assert!(w.get(a.0, a.1).unwrap().road, "Startpunkt muss Straße sein");
    assert!(w.get(b.0, b.1).unwrap().road, "Zielpunkt muss Straße sein");
    let mut seen = std::collections::HashSet::new();
    let mut stack = vec![a];
    while let Some(p) = stack.pop() {
        if !seen.insert(p) {
            continue;
        }
        if p == b {
            return;
        }
        for n in w.neighbors8(p.0, p.1) {
            if w.get(n.0, n.1).unwrap().road {
                stack.push(n);
            }
        }
    }
    panic!("Straße verbindet Start und Ziel nicht");
}

#[test]
fn burn_forest_edit() {
    let cfg = test_cfg(17);
    let world = gen::generate(&cfg);
    let mut state = GameState::new(world);
    let mut rng = rng::from_seed(1);
    let forest = (0..state.world.height)
        .flat_map(|y| (0..state.world.width).map(move |x| (x, y)))
        .find(|(x, y)| { let t = state.world.get(*x, *y).unwrap(); t.water.is_none() && t.biome == Biome::Forest });
    if let Some((x, y)) = forest {
        state.apply(Edit::BurnForest { x, y, radius: 1 }, &mut rng);
        assert!(matches!(
            state.world.get(x, y).unwrap().biome,
            Biome::Grassland
        ));
    }
}

#[test]
fn settlement_edits() {
    let cfg = test_cfg(19);
    let world = gen::generate(&cfg);
    let mut state = GameState::new(world);
    let mut rng = rng::from_seed(1);
    let (x, y) = first_free_land(&state.world);
    state.apply(
        Edit::AddSettlement {
            x,
            y,
            population: 120,
            name: Some("Testdorf".into()),
        },
        &mut rng,
    );
    let s = state.world.get(x, y).unwrap().settlement.clone().unwrap();
    assert_eq!(s.name, "Testdorf");
    assert_eq!(s.population, 120);
    assert_eq!(
        state.world.settlement_tier(x, y),
        Some(SettlementTier::Dorf)
    );
    state.apply(Edit::DestroySettlement { x, y }, &mut rng);
    assert!(state.world.get(x, y).unwrap().settlement.is_none());
}

#[test]
fn flood_then_drain_restores_original_land() {
    let cfg = test_cfg(23);
    let world = gen::generate(&cfg);
    let mut state = GameState::new(world);
    let mut rng = rng::edit_rng(cfg.seed);
    let (x, y) = first_free_land(&state.world);
    let original = state.world.get(x, y).unwrap().biome;
    state.apply(Edit::Flood { x, y, radius: 2 }, &mut rng);
    let t = state.world.get(x, y).unwrap();
    assert_eq!(t.effective_biome(), Biome::Lake);
    assert_eq!(t.biome, original, "Land-Biom darf durch Flutung nicht ueberschrieben werden");
    state.apply(Edit::DrainLake { x, y, radius: 2 }, &mut rng);
    let t = state.world.get(x, y).unwrap();
    assert!(t.water.is_none());
    assert_eq!(t.effective_biome(), original, "Nach dem Austrocknen muss das urspruengliche Land zurueckkommen");
}

#[test]
fn remove_poi_edit_removes_any_poi() {
    use dnd_2d_map::edit::PoiKind;
    let cfg = test_cfg(29);
    let world = gen::generate(&cfg);
    let mut state = GameState::new(world);
    let mut rng = rng::edit_rng(cfg.seed);
    let (x, y) = first_free_land(&state.world);
    state.apply(Edit::AddPoi { x, y, kind: PoiKind::Inn }, &mut rng);
    assert!(matches!(
        state.world.get(x, y).unwrap().poi,
        Some(Poi::Inn { .. })
    ));
    state.apply(Edit::RemovePoi { x, y }, &mut rng);
    assert!(state.world.get(x, y).unwrap().poi.is_none());
}

#[test]
fn undo_rebuilds_state_from_log() {
    use dnd_2d_map::edit::{rebuild, PoiKind};
    let cfg = test_cfg(31);
    let world = gen::generate(&cfg);
    let mut state = GameState::new(world);
    let mut rng = rng::edit_rng(cfg.seed);
    let (x, y) = first_free_land(&state.world);
    state.apply(Edit::AddDungeon { x, y }, &mut rng);
    let (x2, y2) = first_free_land(&state.world);
    state.apply(Edit::AddPoi { x: x2, y: y2, kind: PoiKind::Merchant }, &mut rng);
    assert_eq!(state.revision, 2);
    let log_before = state.log.clone();
    state.undo(&cfg);
    assert_eq!(state.revision, 1);
    let expected = rebuild(&cfg, &log_before[..1]);
    assert_eq!(state, expected);
}

#[test]
fn lakes_have_outflow_river() {
    use dnd_2d_map::world::WaterKind;
    let mut found = false;
    for seed in 1..=30u64 {
        let cfg = Config {
            seed,
            width: 100,
            height: 50,
            ..Config::default()
        };
        let w = gen::generate(&cfg);
        if !w.tiles.iter().any(|t| t.water == Some(WaterKind::Lake)) {
            continue;
        }
        found = true;
        let mut visited = vec![false; w.tiles.len()];
        for y in 0..w.height {
            for x in 0..w.width {
                let i = w.idx(x, y);
                if visited[i] || w.tiles[i].water != Some(WaterKind::Lake) {
                    continue;
                }
                let mut comp = Vec::new();
                let mut stack = vec![(x, y)];
                visited[i] = true;
                while let Some((cx, cy)) = stack.pop() {
                    comp.push((cx, cy));
                    for n in w.neighbors8(cx, cy) {
                        let ni = w.idx(n.0, n.1);
                        if !visited[ni] && w.tiles[ni].water == Some(WaterKind::Lake) {
                            visited[ni] = true;
                            stack.push(n);
                        }
                    }
                }
                let has_outflow = comp.iter().any(|(lx, ly)| {
                    w.neighbors8(*lx, *ly).iter().any(|n| {
                        let nt = w.get(n.0, n.1).unwrap();
                        nt.water == Some(WaterKind::River)
                            || nt.water == Some(WaterKind::Ocean)
                            || nt.elevation < cfg.sea_level
                    })
                });
                assert!(has_outflow, "See-Komponente bei ({x},{y}) (Seed {seed}) hat keinen Zu-/Abfluss");
            }
        }
        break;
    }
    assert!(found, "kein Seed mit See gefunden - Test greift nicht");
}

#[test]
fn river_width_accumulates_at_confluence() {
    use dnd_2d_map::gen::rivers::recompute_river_widths;
    let mut w = World::new(5, 3);
    w.sea_level = 0.5;
    let set = |w: &mut World, x: usize, y: usize, e: f32| {
        let t = w.get_mut(x, y).unwrap();
        t.elevation = e;
        t.river = true;
    };
    set(&mut w, 0, 0, 0.9);
    set(&mut w, 0, 2, 0.9);
    set(&mut w, 1, 1, 0.8);
    set(&mut w, 2, 1, 0.7);
    recompute_river_widths(&mut w);
    assert_eq!(w.get(0, 0).unwrap().river_width, 1);
    assert_eq!(w.get(0, 2).unwrap().river_width, 1);
    assert_eq!(w.get(1, 1).unwrap().river_width, 2, "zwei 1er-Fluesse muessen 2er ergeben");
    assert_eq!(w.get(2, 1).unwrap().river_width, 2);
}

#[test]
fn carved_river_reaches_water() {
    use dnd_2d_map::gen::rivers::carve_downhill;
    let cfg = test_cfg(5);
    let mut w = gen::generate(&cfg);
    let mut best = (0, 0);
    let mut best_e = -1.0f32;
    for y in 0..w.height {
        for x in 0..w.width {
            let e = w.get(x, y).unwrap().elevation;
            if e > best_e {
                best_e = e;
                best = (x, y);
            }
        }
    }
    let path = carve_downhill(&mut w, cfg.sea_level, best);
    assert!(!path.is_empty(), "Flusspfad darf nicht leer sein");
    let last = path.last().unwrap();
    let reaches = w.get(last.0, last.1).unwrap().elevation < cfg.sea_level
        || w.neighbors8(last.0, last.1)
            .iter()
            .any(|n| w.get(n.0, n.1).unwrap().elevation < cfg.sea_level);
    assert!(reaches, "Fluss muss im Gewaesser enden, nicht im Nirgendwo");
}

#[test]
fn settlement_tiers_by_population() {
    use dnd_2d_map::world::SettlementTier::*;
    assert_eq!(SettlementTier::from_population(10, false), Gehoeft);
    assert_eq!(SettlementTier::from_population(50, false), Gehoeft);
    assert_eq!(SettlementTier::from_population(51, false), Dorf);
    assert_eq!(SettlementTier::from_population(500, false), Dorf);
    assert_eq!(SettlementTier::from_population(2000, false), Kleinstadt);
    assert_eq!(SettlementTier::from_population(20000, false), Stadt);
    assert_eq!(SettlementTier::from_population(20001, false), Kaiserstadt);
    assert_eq!(SettlementTier::from_population(30000, true), Hafenstadt);
    assert_eq!(SettlementTier::from_population(1000, true), Kleinstadt, "Kleinstadt an der Kueste ist noch kein Hafen");
    assert_eq!(SettlementTier::from_population(300, true), Dorf, "klein + Kueste = kein Hafen");
    assert_eq!(Gehoeft.marker(), 'g');
    assert_eq!(Hafenstadt.marker(), 'H');
}
