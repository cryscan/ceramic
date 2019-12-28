use amethyst::{
    assets::{PrefabData, ProgressCounter},
    derive::PrefabData,
    ecs::{Component, DenseVecStorage, Entity, prelude::*},
    Error,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Serialize, Deserialize, Component, PrefabData)]
#[serde(default)]
#[storage(DenseVecStorage)]
#[prefab(Component)]
pub struct Animation {
    pub current: usize,
}
