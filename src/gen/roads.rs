use crate::noise::Noise;
use crate::world::{Biome, World};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::collections::HashSet;

pub fn connect(w: &mut World) {
    let setts = w.settlements();
    if setts.len() < 2 {
        return;
    }
    let mut used: std::collections::HashSet<((usize, usize), (usize, usize))> = std::collections::HashSet::new();
    let mut carve = |w: &mut World, a: (usize, usize), b: (usize, usize)| {
        let key = if a <= b { (a, b) } else { (b, a) };
        if used.contains(&key) {
            return;
        }
        if let Some(path) = carve_path(w, a, b) {
            used.insert(key);
            for (x, y) in path {
                w.get_mut(x, y).unwrap().road = true;
            }
        }
    };
    let mut in_tree: Vec<bool> = vec![false; setts.len()];
    let mut best: Vec<(f32, usize)> = vec![(f32::INFINITY, 0); setts.len()];
    in_tree[0] = true;
    for j in 1..setts.len() {
        best[j] = (World::dist(setts[0], setts[j]), 0);
    }
    for _ in 1..setts.len() {
        let mut bi = None;
        let mut bc = f32::INFINITY;
        for j in 0..setts.len() {
            if !in_tree[j] && best[j].0 < bc {
                bc = best[j].0;
                bi = Some(j);
            }
        }
        let Some(i) = bi else { break };
        in_tree[i] = true;
        carve(w, setts[i], setts[best[i].1]);
        for j in 0..setts.len() {
            if !in_tree[j] {
                let d = World::dist(setts[i], setts[j]);
                if d < best[j].0 {
                    best[j] = (d, i);
                }
            }
        }
    }
    for (i, s) in setts.iter().enumerate() {
        let mut nearest: Option<(usize, f32)> = None;
        for (j, t) in setts.iter().enumerate() {
            if i == j {
                continue;
            }
            let d = World::dist(*s, *t);
            if nearest.map_or(true, |(_, bd)| d < bd) {
                nearest = Some((j, d));
            }
        }
        if let Some((j, _)) = nearest {
            carve(w, *s, setts[j]);
        }
    }
    for _ in 0..setts.len() {
        let reached = reachable_settlements(w, &setts);
        let missing: Vec<(usize, usize)> =
            setts.iter().filter(|s| !reached.contains(s)).cloned().collect();
        if missing.is_empty() {
            break;
        }
        let mut progressed = false;
        for m in &missing {
            let mut nearest: Option<((usize, usize), f32)> = None;
            for r in &reached {
                let d = World::dist(*m, *r);
                if nearest.map_or(true, |(_, bd)| d < bd) {
                    nearest = Some((*r, d));
                }
            }
            if let Some((r, _)) = nearest {
                let before = w.tiles.iter().filter(|t| t.road).count();
                carve(w, *m, r);
                if w.tiles.iter().filter(|t| t.road).count() > before {
                    progressed = true;
                }
            }
        }
        if !progressed {
            break;
        }
    }
}

fn reachable_settlements(
    w: &World,
    setts: &[(usize, usize)],
) -> std::collections::HashSet<(usize, usize)> {
    let mut reached = std::collections::HashSet::new();
    if setts.is_empty() {
        return reached;
    }
    let start = setts[0];
    let mut seen: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    let mut stack = vec![start];
    while let Some(p) = stack.pop() {
        if !seen.insert(p) {
            continue;
        }
        if w.get(p.0, p.1).unwrap().settlement.is_some() {
            reached.insert(p);
        }
        for n in w.neighbors8(p.0, p.1) {
            if w.get(n.0, n.1).unwrap().road {
                stack.push(n);
            }
        }
    }
    reached
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
    let ports = compute_ports(w);
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

pub fn compute_ports(w: &World) -> HashSet<usize> {
    let mut ports: HashSet<usize> = HashSet::new();
    let mut visited = vec![false; w.tiles.len()];
    for y in 0..w.height {
        for x in 0..w.width {
            let i = w.idx(x, y);
            if visited[i] || w.tiles[i].elevation < w.sea_level {
                continue;
            }
            let mut comp: Vec<(usize, usize)> = Vec::new();
            let mut stack = vec![(x, y)];
            visited[i] = true;
            while let Some((cx, cy)) = stack.pop() {
                comp.push((cx, cy));
                for n in w.neighbors4(cx, cy) {
                    let ni = w.idx(n.0, n.1);
                    if !visited[ni] && w.tiles[ni].elevation >= w.sea_level {
                        visited[ni] = true;
                        stack.push(n);
                    }
                }
            }
            let settled = comp.iter().any(|(cx, cy)| w.get(*cx, *cy).unwrap().settlement.is_some());
            if !settled {
                continue;
            }
            for (cx, cy) in &comp {
                let land = w.get(*cx, *cy).unwrap();
                let at_port_water = w
                    .neighbors8(*cx, *cy)
                    .iter()
                    .any(|n| World::is_port_water_tile(w.get(n.0, n.1).unwrap()));
                if at_port_water && !land.is_water() {
                    ports.insert(w.idx(*cx, *cy));
                }
            }
            if settled {
                // schiffbares Gewaesser: von der Kueste besiedelter Landmassen erreichbares Wasser
                let mut wstack: Vec<(usize, usize)> = Vec::new();
                for (cx, cy) in &comp {
                    for n in w.neighbors8(*cx, *cy) {
                        let nt = w.get(n.0, n.1).unwrap();
                        if World::is_port_water_tile(nt) && !ports.contains(&w.idx(n.0, n.1)) {
                            ports.insert(w.idx(n.0, n.1));
                            wstack.push(n);
                        }
                    }
                }
                while let Some((x, y)) = wstack.pop() {
                    for n in w.neighbors8(x, y) {
                        let ni = w.idx(n.0, n.1);
                        if !ports.contains(&ni) && World::is_port_water_tile(w.get(n.0, n.1).unwrap()) {
                            ports.insert(ni);
                            wstack.push(n);
                        }
                    }
                }
            }
        }
    }
    ports
}
