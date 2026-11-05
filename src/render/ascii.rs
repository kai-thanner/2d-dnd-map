use crate::render::Renderer;
use crate::world::{Biome, Poi, WaterKind, World};

pub struct AsciiRenderer;

impl AsciiRenderer {
    pub fn biome_char(b: Biome) -> char {
        match b {
            Biome::Ocean => '~',
            Biome::Lake => 'o',
            Biome::River => '≋',
            Biome::Ice => '❄',
            Biome::Coast => ',',
            Biome::Desert => ':',
            Biome::Steppe => '\'',
            Biome::Grassland => '.',
            Biome::Forest => '♣',
            Biome::Jungle => '♠',
            Biome::Taiga => '♧',
            Biome::Tundra => '"',
            Biome::Snow => '*',
            Biome::Rock => 'n',
            Biome::Mountain => '^',
        }
    }

    pub fn legend() -> String {
        let mut s = String::from("Legende:\n");
        for b in [
            Biome::Ocean,
            Biome::Lake,
            Biome::River,
            Biome::Ice,
            Biome::Coast,
            Biome::Desert,
            Biome::Steppe,
            Biome::Grassland,
            Biome::Forest,
            Biome::Jungle,
            Biome::Taiga,
            Biome::Tundra,
            Biome::Snow,
            Biome::Rock,
            Biome::Mountain,
        ] {
            s.push_str(&format!("  {} {}\n", Self::biome_char(b), b.name()));
        }
        s.push_str("  = Straße   g Gehöft   v Dorf   t Kleinstadt   C Stadt   M Kaiserstadt   H Hafenstadt   D Dungeon   I Gasthaus   M Händler\n");
        s
    }
}

impl Renderer for AsciiRenderer {
    fn render(&self, world: &World) -> String {
        let mut out = String::with_capacity(world.tiles.len() + world.height);
        for y in 0..world.height {
            for x in 0..world.width {
                let t = world.get(x, y).unwrap();
                let c = if let Some(tier) = world.settlement_tier(x, y) {
                    tier.marker()
                } else if let Some(p) = &t.poi {
                    match p {
                        Poi::Dungeon { .. } => 'D',
                        Poi::Inn { .. } => 'I',
                        Poi::Merchant { .. } => 'M',
                    }
                } else if t.road {
                    '='
                } else if t.water == Some(WaterKind::River) {
                    if t.river_width >= 2 {
                        'Ξ'
                    } else {
                        '≋'
                    }
                } else {
                    Self::biome_char(t.effective_biome())
                };
                out.push(c);
            }
            out.push('\n');
        }
        out
    }
}
