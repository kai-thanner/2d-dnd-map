use crate::gen::rivers;
use crate::world::{Biome, WaterKind, World};

pub fn fill(w: &mut World, sea_level: f32, snow_temp: f32) {
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
        if ocean[i] || w.tiles[i].elevation >= sea_level {
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
            if t.elevation < sea_level {
                t.biome = Biome::Coast;
                t.river = false;
                t.river_width = 0;
                t.water = Some(if t.temperature <= snow_temp {
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
    // Flüsse + Seen iterativ bis stabil: Senke auffüllen -> See -> Überlauf
    // -> neuer Fluss -> ... bis jeder Fluss im Meer endet (max. 32 Runden).
    for _ in 0..32 {
        let filled_new = fill_depression_lakes(w, sea_level, snow_temp, &ocean);
        let carved = connect_lake_outflows(w, sea_level);
        if !filled_new && !carved {
            break;
        }
    }
}

/// Senken ÜBER dem Meeresspiegel auffüllen (wie das Meer, nur lokal):
/// Priority-Flood von den Kartenrändern aus liefert pro Zelle das Stauziel
/// (filled). Zellen, die nicht mit dem Ozean verbunden sind und deren Stauziel
/// deutlich über der echten Höhe liegt, werden zu Seen (Aufüllung bis zum
/// Überlaufpunkt — flache Wasseroberfläche statt "Kritzelei in der Senke").
fn fill_depression_lakes(w: &mut World, sea_level: f32, snow_temp: f32, ocean: &[bool]) -> bool {
    use std::cmp::Reverse;
    let mut filled_any = false;
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
        if ocean[i] || w.tiles[i].elevation < sea_level {
            continue;
        }
        if filled[i] > w.tiles[i].elevation + 0.015 {
            cand[i] = true;
        }
    }
    // Komponenten bilden; Senken mit Flussende IMMER zu See machen
    // (Fluss darf nicht "im Nichts" enden), sonstige nur wenn groß/tief genug.
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
        let has_river_end = comp.iter().any(|&c| {
            let t = &w.tiles[c];
            if !t.river {
                return false;
            }
            // Flussende: kein tieferer Fluss-Nachbar und kein Gewässer nebenan
            let x = c % w.width;
            let y = c / w.width;
            !w.neighbors8(x, y).iter().any(|nb| {
                let nt = &w.tiles[w.idx(nb.0, nb.1)];
                (nt.river && nt.elevation < t.elevation) || nt.is_water()
            })
        });
        let min_e = comp.iter().map(|&c| w.tiles[c].elevation).fold(f32::INFINITY, f32::min);
        let spill = comp.iter().map(|&c| filled[c]).fold(f32::INFINITY, f32::min);
        let depth = spill - min_e;
        if has_river_end {
            if comp.len() < 2 || depth < 0.005 {
                continue;
            }
        } else if comp.len() < 6 || depth < 0.02 {
            continue;
        }
        for &c in &comp {
            let t = &mut w.tiles[c];
            t.elevation = spill; // Wasseroberfläche = Überlaufhöhe (flach)
            t.biome = Biome::Coast;
            t.river = false;
            t.river_width = 0;
            t.moisture = 1.0;
            t.water = Some(if t.temperature <= snow_temp {
                WaterKind::Ice
            } else {
                WaterKind::Lake
            });
        }
        filled_any = true;
    }
    filled_any
}

fn connect_lake_outflows(w: &mut World, sea_level: f32) -> bool {
    let mut carved_any = false;
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
            // niedrigsten Überlaufpunkt (Randnachbarn mit min. Höhe) wählen
            let mut start: Option<((usize, usize), f32)> = None;
            for (lx, ly) in &comp {
                for nn in w.neighbors8(*lx, *ly) {
                    let nt = w.get(nn.0, nn.1).unwrap();
                    if nt.elevation >= sea_level && !nt.is_water() {
                        let better = start.map_or(true, |(_, be)| nt.elevation < be);
                        if better {
                            start = Some((nn, nt.elevation));
                        }
                    }
                }
            }
            if let Some((s, _)) = start {
                let before = w.tiles.iter().filter(|t| t.river).count();
                rivers::carve_downhill(w, sea_level, s);
                if w.tiles.iter().filter(|t| t.river).count() > before {
                    carved_any = true;
                }
            }
        }
    }
    carved_any
}
