use rand::rngs::StdRng;
use rand::SeedableRng;

pub fn from_seed(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed)
}
