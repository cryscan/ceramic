use amethyst::{
    assets::PrefabData,
    core::{
        math::Vector3,
        timing::Time,
        transform::Transform,
    },
    derive::{PrefabData, SystemDesc},
    ecs::prelude::*,
    error::Error,
    input::{InputHandler, StringBindings},
};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Default, Serialize, Deserialize, PrefabData)]
#[prefab(Component)]
pub struct Player {
    pub speed: f32,
}

impl Component for Player {
    type Storage = VecStorage<Self>;
}

#[derive(Default, SystemDesc)]
pub struct PlayerSystem;

impl<'a> System<'a> for PlayerSystem {
    type SystemData = (
        ReadStorage<'a, Player>,
        WriteStorage<'a, Transform>,
        Read<'a, InputHandler<StringBindings>>,
        Read<'a, Time>,
    );

    fn run(&mut self, (players, mut transforms, input, time): Self::SystemData) {
        for (player, transform) in (&players, &mut transforms).join() {
            let movement = Vector3::new(
                input.axis_value("move_x").unwrap_or(0.0),
                input.axis_value("move_y").unwrap_or(0.0),
                input.axis_value("move_z").unwrap_or(0.0),
            );
            transform.append_translation(time.delta_seconds() * player.speed * movement);
        }
    }
}