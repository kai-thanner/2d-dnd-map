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
                t.river = false;
                t.river_width = 0;
                t.water = Some(if t.temperature <= cfg.climate.snow_temp {
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
    fill_depression_lakes(w, cfg, &ocean);
    connect_lake_outflows(w, cfg);
}

/// Senken ÜBER dem Meeresspiegel auffüllen (wie das Meer, nur lokal):
/// Priority-Flood von den Kartenrändern aus liefert pro Zelle das Stauziel
/// (filled). Zellen, die nicht mit dem Ozean verbunden sind und deren Stauziel
/// deutlich über der echten Höhe liegt, werden zu Seen (Aufüllung bis zum
/// Überlaufpunkt — flache Wasseroberfläche statt "Kritzelei in der Senke").
fn fill_depression_lakes(w: &mut World, cfg: &Config, ocean: &[bool]) {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let n = w.tiles.len();
    let mut filled = vec![f32::INFINITY; n];
    let mut heap: BinaryHeap<Reverse<(u32, usize)>> = BinaryHeap::new();
    let mut border = Vec::new();
    for x in 0..w.width {
        border.push((x, 0));
        border.push((x, w.height - 1));
    }
    for y in 0..w.height {
        border.push((0, y));
        border.push((w.width - 1, y));
    }
    for (x, y) in border {
        let i = w.idx(x, y);
        let e = w.tiles[i].elevation;
        filled[i] = e;
        heap.push(Reverse((e.to_bits(), i)));
    }
    while let Some(Reverse((_, i))) = heap.pop() {
        let x = i % w.width;
        let y = i / w.width;
        for nb in w.neighbors4(x, y) {
            let ni = w.idx(nb.0, nb.1);
            let f = w.tiles[ni].elevation.max(filled[i]);
            if f < filled[ni] {
                filled[ni] = f;
                heap.push(Reverse((f.to_bits(), ni)));
            }
        }
    }
    // Kandidaten: Land über Meeresspiegel, nicht ozeanverbunden, Stauziel deutlich höher
    let mut cand = vec![false; n];
    for i in 0..n {
        if ocean[i] || w.tiles[i].elevation < cfg.sea_level {
            continue;
        }
        if filled[i] > w.tiles[i].elevation + 0.015 {
            cand[i] = true;
        }
    }
    // Komponenten bilden; nur hinreichend große/tiefe Senken werden Seen
    let mut visited = vec![false; n];
    for i in 0..n {
        if !cand[i] || visited[i] {
            continue;
        }
        let mut comp: Vec<usize> = Vec::new();
        let mut stack = vec![i];
        visited[i] = true;
        while let Some(ci) = stack.pop() {
            comp.push(ci);
            let cx = ci % w.width;
            let cy = ci / w.width;
            for nb in w.neighbors8(cx, cy) {
                let ni = w.idx(nb.0, nb.1);
                if cand[ni] && !visited[ni] {
                    visited[ni] = true;
                    stack.push(ni);
                }
            }
        }
        if comp.len() < 6 {
            continue;
        }
        let min_e = comp.iter().map(|&c| w.tiles[c].elevation).fold(f32::INFINITY, f32::min);
        let spill = comp.iter().map(|&c| filled[c]).fold(f32::INFINITY, f32::min);
        if spill - min_e < 0.02 {
            continue;
        }
        for &c in &comp {
            let t = &mut w.tiles[c];
            t.elevation = spill; // Wasseroberfläche = Überlaufhöhe (flach)
            t.biome = Biome::Coast;
            t.river = false;
            t.river_width = 0;
            t.moisture = 1.0;
            t.water = Some(if t.temperature <= cfg.climate.snow_temp {
                WaterKind::Ice
            } else {
                WaterKind::Lake
            });
        }
    }
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
                rivers::carve_downhill(w, cfg.sea_level, s);
            }
        }
    }
}
