use amethyst::{
    animation::{
        AnimationCommand, AnimationControlSet, AnimationSet, EndControl, get_animation_set,
    },
    core::SystemDesc,
    core::Transform,
    derive::SystemDesc,
    ecs::prelude::*,
    utils::tag::Tag,
};

use crate::{component::animation::Animation, scene::PlayerTag};

#[derive(SystemDesc)]
pub struct AnimationPlaySystem;

impl<'a> System<'a> for AnimationPlaySystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, AnimationSet<usize, Transform>>,
        WriteStorage<'a, AnimationControlSet<usize, Transform>>,
        ReadStorage<'a, Animation>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (entities, sets, mut controls, animations) = data;
        for (entity, set, animation) in (&*entities, &sets, &animations).join() {
            let entity: Entity = entity;
            let set: &AnimationSet<usize, Transform> = set;
            let animation: &Animation = animation;

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
