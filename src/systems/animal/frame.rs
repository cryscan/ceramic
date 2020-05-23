use amethyst::{
    derive::SystemDesc,
    ecs::prelude::*,
};

#[derive(Debug, SystemDesc)]
pub struct FrameSystem;

impl<'a> System<'a> for FrameSystem {
    type SystemData = ();

    fn run(&mut self, _data: Self::SystemData) {
        unimplemented!()
    }
}