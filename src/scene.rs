use amethyst::{
    assets::{AssetPrefab, Handle, Prefab, PrefabData, ProgressCounter},
    controls::ControlTagPrefab,
    core::Transform,
    derive::PrefabData,
    ecs::prelude::*,
    Error,
    gltf::{GltfSceneAsset, GltfSceneFormat},
    renderer::{camera::CameraPrefab, light::LightPrefab},
    utils::{auto_fov::AutoFov, tag::Tag},
};
use serde::{Deserialize, Serialize};

use crate::component::animation::Animation;

#[derive(Clone, Serialize, Deserialize)]
pub struct PlayerTag;

#[derive(Default)]
pub struct Scene {
    pub handle: Option<Handle<Prefab<ScenePrefab>>>,
}

#[derive(Serialize, Deserialize, Default, PrefabData)]
#[serde(default)]
pub struct ScenePrefab {
    transform: Option<Transform>,
    model: Option<AssetPrefab<GltfSceneAsset, GltfSceneFormat>>,
    animation: Option<Animation>,
    light: Option<LightPrefab>,
    camera: Option<CameraPrefab>,
    auto_fov: Option<AutoFov>,
    control_tag: Option<ControlTagPrefab>,
    player_tag: Option<Tag<PlayerTag>>,
}
