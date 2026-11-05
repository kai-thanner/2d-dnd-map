use crate::config::Config;
use crate::world::World;
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
    let n_rivers = ((w.width * w.height) as f32 / 1200.0).max(3.0) as usize;
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
        walk(w, cfg, start);
    }
}

fn walk(w: &mut World, cfg: &Config, start: (usize, usize)) {
    let mut visited: HashSet<(usize, usize)> = HashSet::new();
    let mut path: Vec<(usize, usize)> = Vec::new();
    let mut cur = start;
    loop {
        if w.get(cur.0, cur.1).unwrap().elevation < cfg.sea_level {
            break;
        }
        if !visited.insert(cur) {
            break;
        }
        path.push(cur);
        if path.len() > 600 {
            break;
        }
        let cur_e = w.get(cur.0, cur.1).unwrap().elevation;
        let mut next: Option<(usize, usize)> = None;
        let mut next_e = cur_e;
        for n in w.neighbors8(cur.0, cur.1) {
            let e = w.get(n.0, n.1).unwrap().elevation;
            if e < next_e {
                next_e = e;
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
                t.moisture = (t.moisture + 0.2).min(1.0);
            }
        }
    }
}
