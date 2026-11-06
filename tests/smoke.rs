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
            snow_temp: 0.0,
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
    let mut world = World::new(30, 20);
    world.sea_level = 0.0;
    for t in &mut world.tiles {
        t.elevation = 0.5;
    }
    let mut state = GameState::new(world);
    let mut rng = rng::from_seed(1);
    let a = (0, 0);
    let b = (29, 19);
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
fn road_on_empty_land_always_connects() {
    let mut w = World::new(20, 10);
    w.sea_level = 0.0;
    for t in &mut w.tiles {
        t.elevation = 0.5;
    }
    let path = dnd_2d_map::gen::roads::carve_path(&w, (0, 0), (19, 9)).unwrap();
    assert_eq!(path.first(), Some(&(0, 0)));
    assert_eq!(path.last(), Some(&(19, 9)));
}

#[test]
fn big_map_does_not_overflow_river_widths() {
    let cfg = Config {
        seed: 1,
        width: 320,
        height: 320,
        ..Config::default()
    };
    let w = gen::generate(&cfg);
    assert!(w.tiles.iter().all(|t| t.river_width <= 8));
}

#[test]
fn sea_level_zero_does_not_crash() {
    let cfg = Config {
        seed: 1,
        width: 150,
        height: 150,
        sea_level: 0.0,
        ..Config::default()
    };
    let w = gen::generate(&cfg);
    assert!(w.tiles.iter().all(|t| t.river_width <= 8));
}

#[test]
fn roads_never_on_ocean_and_only_on_navigable_water() {
    use dnd_2d_map::world::WaterKind;
    for seed in 1..=10u64 {
        let cfg = Config {
            seed,
            width: 100,
            height: 50,
            ..Config::default()
        };
        let w = gen::generate(&cfg);
        let ports = dnd_2d_map::gen::roads::compute_ports(&w);
        for y in 0..w.height {
            for x in 0..w.width {
                let t = w.get(x, y).unwrap();
                if !t.road {
                    continue;
                }
                match t.water {
                    Some(WaterKind::Ocean) | Some(WaterKind::Ice) => {
                        panic!("STRASSE_AUF_MEER bei ({x},{y}) seed {seed}");
                    }
                    Some(WaterKind::Lake) => {
                        assert!(
                            ports.contains(&w.idx(x, y)),
                            "STRASSE_AUF_UNNAVIGABLEM_SEE bei ({x},{y}) seed {seed}"
                        );
                    }
                    Some(WaterKind::River) | None => {}
                }
            }
        }
    }
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
                        nt.river
                            || nt.water == Some(WaterKind::River)
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

#[test]
fn drain_removes_small_streams() {
    use dnd_2d_map::world::WaterKind;
    let cfg = test_cfg(9);
    let world = gen::generate(&cfg);
    let mut state = GameState::new(world);
    let mut rng = rng::edit_rng(cfg.seed);
    let stream = (0..state.world.height)
        .flat_map(|y| (0..state.world.width).map(move |x| (x, y)))
        .find(|(x, y)| {
            let t = state.world.get(*x, *y).unwrap();
            t.water == Some(WaterKind::River) && t.river_width == 1
        })
        .expect("kein 1px-Bach gefunden");
    state.apply(Edit::DrainLake { x: stream.0, y: stream.1, radius: 1 }, &mut rng);
    let t = state.world.get(stream.0, stream.1).unwrap();
    assert!(t.water.is_none(), "kleiner Bach muss austrocknen");
    assert!(!t.river);
}

#[test]
fn flood_swallows_stream() {
    use dnd_2d_map::world::WaterKind;
    let cfg = test_cfg(9);
    let world = gen::generate(&cfg);
    let mut state = GameState::new(world);
    let mut rng = rng::edit_rng(cfg.seed);
    let stream = (0..state.world.height)
        .flat_map(|y| (0..state.world.width).map(move |x| (x, y)))
        .find(|(x, y)| state.world.get(*x, *y).unwrap().water == Some(WaterKind::River))
        .expect("kein Bach gefunden");
    state.apply(Edit::Flood { x: stream.0, y: stream.1, radius: 2 }, &mut rng);
    let t = state.world.get(stream.0, stream.1).unwrap();
    assert_eq!(t.water, Some(WaterKind::Lake), "See muss den Bach innerhalb des Radius ueberdecken");
    assert!(!t.river);
}

#[test]
fn every_settled_landmass_has_a_harbor() {
    for seed in 1..=8u64 {
        let cfg = Config {
            seed,
            width: 120,
            height: 60,
            ..Config::default()
        };
        let w = gen::generate(&cfg);
        let sea = cfg.sea_level;
        let mut visited = vec![false; w.tiles.len()];
        for y in 0..w.height {
            for x in 0..w.width {
                let i = w.idx(x, y);
                if visited[i] || w.tiles[i].elevation < sea {
                    continue;
                }
                let mut comp = Vec::new();
                let mut stack = vec![(x, y)];
                visited[i] = true;
                while let Some((cx, cy)) = stack.pop() {
                    comp.push((cx, cy));
                    for n in w.neighbors4(cx, cy) {
                        let ni = w.idx(n.0, n.1);
                        if !visited[ni] && w.tiles[ni].elevation >= sea {
                            visited[ni] = true;
                            stack.push(n);
                        }
                    }
                }
                let setts: Vec<(usize, usize)> = comp
                    .iter()
                    .filter(|(cx, cy)| w.get(*cx, *cy).unwrap().settlement.is_some())
                    .cloned()
                    .collect();
                if setts.is_empty() {
                    continue;
                }
                let total_pop: u64 = setts
                    .iter()
                    .map(|s| w.get(s.0, s.1).unwrap().settlement.as_ref().unwrap().population as u64)
                    .sum();
                if total_pop < 2000 {
                    continue;
                }
                let has_harbor = setts.iter().any(|s| {
                    w.get(s.0, s.1)
                        .unwrap()
                        .settlement
                        .as_ref()
                        .map_or(false, |st| st.has_port)
                });
                assert!(has_harbor, "Landmasse mit {total_pop} Ew. (Seed {seed}) hat keinen Hafen");
            }
        }
    }
}

#[test]
fn dry_south_creates_more_desert_in_south() {
    let cfg = Config {
        seed: 3,
        width: 100,
        height: 60,
        moisture_scale: 0.55,
        dry_south: 1.0,
        ..Config::default()
    };
    let w = gen::generate(&cfg);
    let count_desert = |y0: usize, y1: usize| -> usize {
        (y0..y1)
            .flat_map(|y| (0..w.width).map(move |x| (x, y)))
            .filter(|(x, y)| w.get(*x, *y).unwrap().biome == Biome::Desert)
            .count()
    };
    let north = count_desert(0, w.height / 4);
    let south = count_desert(w.height - w.height / 4, w.height);
    assert!(south > 0, "im Süden muss Wüste entstehen");
    assert!(south > north, "Süden ({south}) muss mehr Wüste haben als Norden ({north})");
}

#[test]
fn river_sprawl_is_bounded() {
    use dnd_2d_map::world::WaterKind;
    for seed in 1..=5u64 {
        let cfg = Config {
            seed,
            width: 100,
            height: 50,
            ..Config::default()
        };
        let w = gen::generate(&cfg);
        let land = w.tiles.iter().filter(|t| !t.is_water()).count();
        let rivers = w.tiles.iter().filter(|t| t.water == Some(WaterKind::River)).count();
        assert!(
            rivers * 100 <= land * 12,
            "Flussflaeche zu gross: {rivers} von {land} Landkacheln (Seed {seed})"
        );
    }
}

#[test]
fn river_amount_scales_river_count() {
    use dnd_2d_map::world::WaterKind;
    let count_rivers = |amount: f32| {
        let cfg = Config {
            seed: 9,
            width: 100,
            height: 50,
            river_amount: amount,
            ..Config::default()
        };
        gen::generate(&cfg)
            .tiles
            .iter()
            .filter(|t| t.water == Some(WaterKind::River))
            .count()
    };
    let zero = count_rivers(0.0);
    let one = count_rivers(1.0);
    let two = count_rivers(2.0);
    assert!(one > 0, "Default muss Fluesse erzeugen");
    assert!(two >= one, "mehr Flussmenge darf nicht weniger Fluesse ergeben ({two} vs {one})");
    assert!(zero <= one, "Flussmenge 0 darf hoechstens gleich viel sein ({zero} vs {one})");
}

#[test]
fn snow_temp_controls_snow_line() {
    let mut cfg = Config {
        seed: 3,
        width: 60,
        height: 40,
        ..Config::default()
    };
    let count_snow = |cfg: &Config| {
        gen::generate(cfg)
            .tiles
            .iter()
            .filter(|t| !t.is_water() && t.biome == Biome::Snow)
            .count()
    };
    let cold = count_snow(&cfg);
    cfg.climate.snow_temp = 20.0;
    let warm_line = count_snow(&cfg);
    assert!(
        warm_line > cold,
        "höhere Schneegrenze muss mehr Schnee ergeben ({warm_line} vs {cold})"
    );
}

#[test]
fn treeline_in_meters_wiese_above_peak_rocky() {
    use dnd_2d_map::gen::biomes;
    let mut cfg = Config {
        seed: 1,
        width: 20,
        height: 10,
        sea_level: 0.5,
        max_elevation_m: 3000.0,
        treeline_m: 900.0,
        ..Config::default()
    };
    cfg.climate.south_temp = 15.0;
    cfg.climate.north_temp = 15.0;
    cfg.climate.lapse_per_unit = 0.0;
    let mut w = World::new(20, 10);
    w.sea_level = 0.5;
    for t in &mut w.tiles {
        t.elevation = 0.7; // h=0.2 -> 600m, unter Baumgrenze
        t.moisture = 0.8;
        t.temperature = 15.0;
    }
    w.get_mut(15, 5).unwrap().elevation = 0.82; // h=0.32 -> 960m > 900, h<0.34 -> Wiese
    w.get_mut(17, 5).unwrap().elevation = 1.0; // h=0.5 -> 1500m, h>=0.34 -> Gipfel
    biomes::assign(&mut w, &cfg);
    assert_eq!(w.get(5, 5).unwrap().biome, Biome::Forest, "unter Baumgrenze mit Feuchte = Wald");
    assert!(
        matches!(w.get(15, 5).unwrap().biome, Biome::Grassland | Biome::Steppe),
        "ueber Baumgrenze = Wiese, nicht Wald: {:?}",
        w.get(15, 5).unwrap().biome
    );
    assert_eq!(w.get(17, 5).unwrap().biome, Biome::Mountain, "hoher Gipfel = Mountain");
}

#[test]
fn all_settlements_connected_to_road_network() {
    use dnd_2d_map::world::WaterKind;
    for seed in 1..=8u64 {
        let cfg = Config {
            seed,
            width: 120,
            height: 60,
            ..Config::default()
        };
        let w = gen::generate(&cfg);
        let setts = w.settlements();
        if setts.len() < 2 {
            continue;
        }
        let start = setts[0];
        let is_landmass = |x: usize, y: usize| {
            !matches!(
                w.get(x, y).unwrap().water,
                Some(WaterKind::Ocean) | Some(WaterKind::Lake) | Some(WaterKind::Ice)
            )
        };
        let mut comp = std::collections::HashSet::new();
        let mut stack = vec![start];
        comp.insert(start);
        while let Some((x, y)) = stack.pop() {
            for n in w.neighbors4(x, y) {
                if is_landmass(n.0, n.1) && comp.insert(n) {
                    stack.push(n);
                }
            }
        }
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![start];
        while let Some(p) = stack.pop() {
            if !seen.insert(p) {
                continue;
            }
            for n in w.neighbors8(p.0, p.1) {
                if w.get(n.0, n.1).unwrap().road {
                    stack.push(n);
                }
            }
        }
        let unreachable: Vec<_> = setts
            .iter()
            .filter(|s| comp.contains(s) && !seen.contains(s))
            .collect();
        assert!(
            unreachable.is_empty(),
            "Seed {seed}: Siedlungen ohne Strassenanschluss auf derselben Landmasse: {unreachable:?}"
        );
    }
}

#[test]
fn no_river_flags_inside_lakes() {
    use dnd_2d_map::world::WaterKind;
    for seed in 1..=10u64 {
        let cfg = Config {
            seed,
            width: 100,
            height: 50,
            ..Config::default()
        };
        let w = gen::generate(&cfg);
        for t in &w.tiles {
            if t.water == Some(WaterKind::Lake) {
                assert!(!t.river, "Seed {seed}: Fluss-Flag auf See-Kachel");
            }
        }
    }
}
