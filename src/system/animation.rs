use amethyst::{
    animation::{
        AnimationCommand, AnimationControlSet, AnimationSet, EndControl, get_animation_set,
    },
    assets::PrefabData,
    core::{SystemDesc, Transform},
    derive::{PrefabData, SystemDesc},
    ecs::prelude::*,
    Error,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Serialize, Deserialize, PrefabData)]
#[serde(default)]
#[prefab(Component)]
pub struct Animation {
    pub current: usize,
}

impl Component for Animation {
    type Storage = DenseVecStorage<Self>;
}

#[derive(SystemDesc)]
pub struct AnimationPlaySystem;

impl<'a> System<'a> for AnimationPlaySystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, AnimationSet<usize, Transform>>,
        WriteStorage<'a, AnimationControlSet<usize, Transform>>,
        ReadStorage<'a, Animation>,
    );

    fn run(&mut self, (entities, sets, mut controls, animations): Self::SystemData) {
        for (entity, set, animation) in (&*entities, &sets, &animations).join() {
            let control = get_animation_set(&mut controls, entity).unwrap();
            if control.has_animation(animation.current) {
                control.toggle(animation.current);
            } else {
                let ref current = animation.current;
                if let Some(animation) = set.get(current) {
                    control.add_animation(
                        *current,
                        animation,
                        EndControl::Normal,
                        1.0,
                        AnimationCommand::Start,
                    );
                }
            }
        }
    }
}
