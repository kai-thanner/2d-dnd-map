use dnd_2d_map::palette::{biome_rgb, cell_at};
use dnd_2d_map::world::Biome;

const ALL_BIOMES: [Biome; 15] = [
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
];

#[test]
fn cell_at_maps_pixels_to_tiles() {
    assert_eq!(cell_at(0.0, 0.0, 10.0, 5, 5), Some((0, 0)));
    assert_eq!(cell_at(9.9, 9.9, 10.0, 5, 5), Some((0, 0)));
    assert_eq!(cell_at(19.9, 29.9, 10.0, 5, 5), Some((1, 2)));
    assert_eq!(cell_at(49.9, 49.9, 10.0, 5, 5), Some((4, 4)));
    assert_eq!(cell_at(50.0, 0.0, 10.0, 5, 5), None);
    assert_eq!(cell_at(0.0, 50.0, 10.0, 5, 5), None);
    assert_eq!(cell_at(-1.0, 0.0, 10.0, 5, 5), None);
    assert_eq!(cell_at(1.0, 1.0, 0.0, 5, 5), None);
}

#[test]
fn every_biome_has_a_distinct_enough_color() {
    for b in ALL_BIOMES {
        let _ = biome_rgb(b);
    }
    assert_ne!(biome_rgb(Biome::Ocean), biome_rgb(Biome::Lake));
    assert_ne!(biome_rgb(Biome::Snow), biome_rgb(Biome::Ice));
    assert_ne!(biome_rgb(Biome::Desert), biome_rgb(Biome::Grassland));
    assert_ne!(biome_rgb(Biome::Mountain), biome_rgb(Biome::Rock));
}
