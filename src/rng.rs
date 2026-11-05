use rand::rngs::StdRng;
use rand::SeedableRng;

pub fn from_seed(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed)
}

pub fn edit_rng(seed: u64) -> StdRng {
    from_seed(seed ^ 0xED17_5EED)
}
