use crate::config::Config;
use crate::noise::Noise;
use crate::world::World;
use std::collections::HashMap;
use std::collections::HashSet;

pub fn carve(w: &mut World, cfg: &Config) {
    let mut sources: Vec<(usize, usize)> = Vec::new();
    for y in 0..w.height {
        for x in 0..w.width {
            let e = w.get(x, y).unwrap().elevation;
            if e > cfg.sea_level + 0.12 {
                sources.push((x, y));
            }
        }
    }
    sources.sort_by(|a, b| {
        let ea = w.get(a.0, a.1).unwrap().elevation;
        let eb = w.get(b.0, b.1).unwrap().elevation;
        eb.partial_cmp(&ea).unwrap()
    });
    let n_rivers = if cfg.river_amount <= 0.0 {
        0
    } else {
        (((w.width * w.height) as f32 / 1800.0) * cfg.river_amount).max(3.0) as usize
    };
    let mut chosen: Vec<(usize, usize)> = Vec::new();
    for s in sources {
        if chosen.len() >= n_rivers {
            break;
        }
        if chosen.iter().any(|c| World::dist(*c, s) < 8.0) {
            continue;
        }
        chosen.push(s);
    }
    for start in chosen {
        carve_downhill(w, cfg.sea_level, start);
    }
}

pub fn carve_downhill(w: &mut World, sea_level: f32, start: (usize, usize)) -> Vec<(usize, usize)> {
    let jitter = Noise::new(
        0xBEEF_5EED ^ ((start.0 as u64) << 12) ^ (start.1 as u64),
    );
    let mut visited: HashSet<(usize, usize)> = HashSet::new();
    let mut path: Vec<(usize, usize)> = Vec::new();
    let mut cur = start;
    loop {
        if w.get(cur.0, cur.1).unwrap().elevation < sea_level {
            break;
        }
        if !visited.insert(cur) {
            break;
        }
        path.push(cur);
        if path.len() > 350 {
            break;
        }
        let cur_e = w.get(cur.0, cur.1).unwrap().elevation;
        let mut next: Option<(usize, usize)> = None;
        let mut next_score = f32::INFINITY;
        for n in w.neighbors8(cur.0, cur.1) {
            if visited.contains(&n) {
                continue;
            }
            let e = w.get(n.0, n.1).unwrap().elevation;
            let uphill = (e - cur_e).max(0.0) * 3.0;
            let score = e + uphill + jitter.fbm(n.0 as f32 * 0.55, n.1 as f32 * 0.55, 2) * 0.035;
            if score < next_score {
                next_score = score;
                next = Some(n);
            }
        }
        match next {
            Some(n) => cur = n,
            None => break,
        }
    }
    for (x, y) in &path {
        let t = w.get_mut(*x, *y).unwrap();
        t.river = true;
        t.moisture = (t.moisture + 0.4).min(1.0);
    }
    for (x, y) in &path {
        for n in w.neighbors8(*x, *y) {
            let t = w.get_mut(n.0, n.1).unwrap();
            if !t.river {
                t.moisture = (t.moisture + 0.25).min(1.0);
            }
        }
    }
    path
}

const MAX_RIVER_WIDTH: u64 = 8;

pub fn recompute_river_widths(w: &mut World) {
    let mut river_tiles: Vec<(usize, usize)> = Vec::new();
    for y in 0..w.height {
        for x in 0..w.width {
            if w.get(x, y).unwrap().river {
                river_tiles.push((x, y));
            }
        }
    }
    let mut downstream: HashMap<(usize, usize), Option<(usize, usize)>> = HashMap::new();
    for r in &river_tiles {
        let e_r = w.get(r.0, r.1).unwrap().elevation;
        let mut best: Option<(usize, usize)> = None;
        let mut best_e = e_r;
        for n in w.neighbors8(r.0, r.1) {
            let nt = w.get(n.0, n.1).unwrap();
            if nt.river && nt.elevation < best_e {
                best_e = nt.elevation;
                best = Some(n);
            }
        }
        downstream.insert(*r, best);
    }
    let mut memo: HashMap<(usize, usize), u32> = HashMap::new();
    let mut visiting: HashSet<(usize, usize)> = HashSet::new();
    for r in &river_tiles {
        width_of(w, *r, &downstream, &mut memo, &mut visiting);
    }
    for (r, v) in &memo {
        if let Some(t) = w.get_mut(r.0, r.1) {
            t.river_width = *v;
        }
    }
}

fn width_of(
    w: &World,
    r: (usize, usize),
    downstream: &HashMap<(usize, usize), Option<(usize, usize)>>,
    memo: &mut HashMap<(usize, usize), u32>,
    visiting: &mut HashSet<(usize, usize)>,
) -> u32 {
    if let Some(v) = memo.get(&r) {
        return *v;
    }
    if !visiting.insert(r) {
        return 1;
    }
    let mut sum: u64 = 0;
    for n in w.neighbors8(r.0, r.1) {
        if w.get(n.0, n.1).unwrap().river && downstream.get(&n) == Some(&Some(r)) {
            sum = sum.saturating_add(width_of(w, n, downstream, memo, visiting) as u64);
        }
    }
    visiting.remove(&r);
    let v = if sum == 0 {
        1
    } else {
        sum.min(MAX_RIVER_WIDTH) as u32
    };
    memo.insert(r, v);
    v
}
