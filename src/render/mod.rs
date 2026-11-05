pub mod ascii;

use crate::world::World;

pub trait Renderer {
    fn render(&self, world: &World) -> String;
}
