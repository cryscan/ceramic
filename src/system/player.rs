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

use crate::prefab::scene::PlayerTag;

#[derive(Default)]
pub struct PlayerAnimation {
    animation_index: usize,
}

#[derive(SystemDesc)]
pub struct PlayerSystem;

impl<'a> System<'a> for PlayerSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, AnimationSet<usize, Transform>>,
        WriteStorage<'a, AnimationControlSet<usize, Transform>>,
        ReadStorage<'a, Tag<PlayerTag>>,
        Write<'a, PlayerAnimation>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (entities, sets, mut controls, tags, player_animation) = data;
        for (entity, set, _) in (&*entities, &sets, &tags).join() {
            let entity: Entity = entity;
            let set: &AnimationSet<usize, Transform> = set;

            let control = get_animation_set(&mut controls, entity).unwrap();
            let ref index = player_animation.animation_index;
            if control.has_animation(*index) {
                control.toggle(*index);
            } else {
                if let Some(animation) = set.get(index) {
                    control.add_animation(
                        *index,
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
