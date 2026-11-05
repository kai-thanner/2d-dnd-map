use crate::config::Config;
use crate::gen::rivers;
use crate::world::{Biome, WaterKind, World};

pub fn fill(w: &mut World, cfg: &Config) {
    let mut ocean = vec![false; w.tiles.len()];
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for x in 0..w.width {
        stack.push((x, 0));
        stack.push((x, w.height - 1));
    }
    for y in 0..w.height {
        stack.push((0, y));
        stack.push((w.width - 1, y));
    }
    while let Some((x, y)) = stack.pop() {
        let i = w.idx(x, y);
        if ocean[i] || w.tiles[i].elevation >= cfg.sea_level {
            continue;
        }
        ocean[i] = true;
        for n in w.neighbors4(x, y) {
            stack.push(n);
        }
    }
    for y in 0..w.height {
        for x in 0..w.width {
            let i = w.idx(x, y);
            let is_ocean = ocean[i];
            let t = w.get_mut(x, y).unwrap();
            if t.elevation < cfg.sea_level {
                t.biome = Biome::Coast;
                t.water = Some(if t.temperature <= 0.0 {
                    WaterKind::Ice
                } else if t.river {
                    WaterKind::River
                } else if is_ocean {
                    WaterKind::Ocean
                } else {
                    WaterKind::Lake
                });
            }
        }
    }
    connect_lake_outflows(w, cfg);
}

fn connect_lake_outflows(w: &mut World, cfg: &Config) {
    let mut visited = vec![false; w.tiles.len()];
    for y in 0..w.height {
        for x in 0..w.width {
            let i = w.idx(x, y);
            if visited[i] || w.tiles[i].water != Some(WaterKind::Lake) {
                continue;
            }
            let mut comp: Vec<(usize, usize)> = Vec::new();
            let mut stack = vec![(x, y)];
            visited[i] = true;
            while let Some((cx, cy)) = stack.pop() {
                comp.push((cx, cy));
                for nn in w.neighbors8(cx, cy) {
                    let ni = w.idx(nn.0, nn.1);
                    if !visited[ni] && w.tiles[ni].water == Some(WaterKind::Lake) {
                        visited[ni] = true;
                        stack.push(nn);
                    }
                }
            }
            let mut start = None;
            'outer: for (lx, ly) in &comp {
                for nn in w.neighbors8(*lx, *ly) {
                    if w.get(nn.0, nn.1).unwrap().elevation >= cfg.sea_level {
                        start = Some(nn);
                        break 'outer;
                    }
                }
            }
            if let Some(s) = start {
                rivers::carve_downhill(w, cfg, s);
            }
        }
    }
}
