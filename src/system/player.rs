use std::f32::EPSILON;

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
use getset::{CopyGetters, Getters};
use num_traits::identities::Zero;
use serde::{Deserialize, Serialize};

#[derive(Getters, CopyGetters, Debug, Copy, Clone, Serialize, Deserialize, PrefabData)]
#[prefab(Component)]
pub struct Player {
    #[get_copy = "pub"]
    speed: f32,
    #[serde(skip, default = "Vector3::zero")]
    #[get = "pub"]
    movement: Vector3<f32>,
}

impl Component for Player {
    type Storage = VecStorage<Self>;
}

#[derive(Default, SystemDesc)]
pub struct PlayerSystem;

impl<'a> System<'a> for PlayerSystem {
    type SystemData = (
        WriteStorage<'a, Player>,
        WriteStorage<'a, Transform>,
        Read<'a, InputHandler<StringBindings>>,
        Read<'a, Time>,
    );

    fn run(&mut self, (mut players, mut transforms, input, time): Self::SystemData) {
        for (player, transform) in (&mut players, &mut transforms).join() {
            let movement = Vector3::new(
                input.axis_value("move_x").unwrap_or(0.0),
                0.0,
                input.axis_value("move_z").unwrap_or(0.0),
            );
            player.movement = movement.try_normalize(EPSILON).unwrap_or(Vector3::zero());
            transform.append_translation(time.delta_seconds() * player.speed * &player.movement);
        }
    }
}