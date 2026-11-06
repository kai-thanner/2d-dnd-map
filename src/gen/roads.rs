use crate::noise::Noise;
use crate::world::{Biome, World};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::collections::HashSet;

pub fn connect(w: &mut World) {
    let points = w.settlements();
    if points.len() < 2 {
        return;
    }
    let mut connected = vec![points[0]];
    let mut rest = points[1..].to_vec();
    while !rest.is_empty() {
        let mut best: Option<(f32, (usize, usize), (usize, usize))> = None;
        for c in &connected {
            for r in &rest {
                let d = World::dist(*c, *r);
                if best.map_or(true, |(bd, _, _)| d < bd) {
                    best = Some((d, *c, *r));
                }
            }
        }
        if let Some((_, c, r)) = best {
            if let Some(path) = carve_path(w, c, r) {
                for (x, y) in path {
                    if let Some(t) = w.get_mut(x, y) {
                        t.road = true;
                    }
                }
            }
            connected.push(r);
            rest.retain(|p| *p != r);
        } else {
            break;
        }
    }
}

fn step_cost(w: &World, ni: usize, x: usize, y: usize, from_e: f32, jitter: &Noise, ports: &HashSet<usize>) -> f32 {
    let t = w.get(x, y).unwrap();
    let base = match t.effective_biome() {
        Biome::Ocean | Biome::Lake | Biome::Ice => {
            if ports.contains(&ni) {
                6.0
            } else {
                f32::INFINITY
            }
        }
        Biome::River => 15.0,
        Biome::Mountain => 100.0,
        Biome::Rock => 25.0,
        Biome::Forest | Biome::Jungle | Biome::Taiga => 3.0,
        Biome::Desert => 2.0,
        _ => 1.0,
    };
    let discount = if t.road || t.settlement.is_some() {
        0.5
    } else {
        1.0
    };
    let hill = (t.elevation - from_e).abs() * 6.0;
    (base + hill + jitter.fbm(x as f32 * 0.18, y as f32 * 0.18, 3) * 6.0) * discount
}

pub fn carve_path(w: &World, from: (usize, usize), to: (usize, usize)) -> Option<Vec<(usize, usize)>> {
    if !w.in_bounds(from.0, from.1) || !w.in_bounds(to.0, to.1) {
        return None;
    }
    let jitter = Noise::new(
        0x5EED_1234
            ^ ((from.0 as u64) << 20)
            ^ ((from.1 as u64) << 10)
            ^ ((to.0 as u64) << 5)
            ^ (to.1 as u64),
    );
    let mut ports: HashSet<usize> = HashSet::new();
    for (sx, sy) in w.settlements() {
        if w.near_water(sx, sy) {
            ports.insert(w.idx(sx, sy));
            for n in w.neighbors8(sx, sy) {
                ports.insert(w.idx(n.0, n.1));
            }
        }
    }
    let n = w.tiles.len();
    let mut dist = vec![f32::INFINITY; n];
    let mut prev: Vec<Option<usize>> = vec![None; n];
    let mut heap: BinaryHeap<Reverse<(u32, usize)>> = BinaryHeap::new();
    let start = w.idx(from.0, from.1);
    let goal = w.idx(to.0, to.1);
    dist[start] = 0.0;
    heap.push(Reverse((0, start)));
    while let Some(Reverse((cd, i))) = heap.pop() {
        if i == goal {
            break;
        }
        let c = dist[i];
        if (cd as f32) / 1000.0 > c + 1e-4 {
            continue;
        }
        let x = i % w.width;
        let y = i / w.width;
        let from_e = w.tiles[i].elevation;
        for (nx, ny) in w.neighbors8(x, y) {
            let ni = w.idx(nx, ny);
            let nd = c + step_cost(w, ni, nx, ny, from_e, &jitter, &ports);
            if nd < dist[ni] {
                dist[ni] = nd;
                prev[ni] = Some(i);
                heap.push(Reverse(((nd * 1000.0) as u32, ni)));
            }
        }
    }
    if dist[goal].is_infinite() {
        return None;
    }
    let mut path = Vec::new();
    let mut cur = Some(goal);
    while let Some(i) = cur {
        path.push((i % w.width, i / w.width));
        cur = prev[i];
    }
    path.reverse();
    Some(path)
}

pub fn nearest_road(w: &World, p: (usize, usize)) -> Option<(usize, usize)> {
    let mut best: Option<((usize, usize), f32)> = None;
    for y in 0..w.height {
        for x in 0..w.width {
            if w.get(x, y).unwrap().road {
                let d = World::dist((x, y), p);
                if best.map_or(true, |(_, bd)| d < bd) {
                    best = Some(((x, y), d));
                }
            }
        }
    }
    best.map(|(p, _)| p)
}

pub fn has_adjacent_road(w: &World, p: (usize, usize)) -> bool {
    w.get(p.0, p.1).unwrap().road
        || w.neighbors8(p.0, p.1)
            .iter()
            .any(|n| w.get(n.0, n.1).unwrap().road)
}
