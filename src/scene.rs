use amethyst::{
    assets::{AssetPrefab, Handle, Prefab, PrefabData, ProgressCounter},
    controls::ControlTagPrefab,
    core::{Named, Transform},
    derive::PrefabData,
    ecs::prelude::*,
    Error,
    gltf::{GltfSceneAsset, GltfSceneFormat},
    renderer::{camera::CameraPrefab, light::LightPrefab},
    utils::auto_fov::AutoFov,
};
use serde::{Deserialize, Serialize};

use crate::system::{
    kinematics::{Binder, ChainPrefab, Hinge, PolePrefab},
    player::Player,
};

#[derive(Default)]
pub struct Scene {
    pub handle: Option<Handle<Prefab<ScenePrefab>>>,
}

#[derive(Serialize, Deserialize, Default, PrefabData)]
#[serde(default)]
pub struct ScenePrefab {
    transform: Option<Transform>,
    name: Option<Named>,
    model: Option<AssetPrefab<GltfSceneAsset, GltfSceneFormat>>,
    player: Option<Player>,
    binder: Option<Binder>,
    chain: Option<ChainPrefab>,
    hinge: Option<Hinge>,
    ball_joint: Option<PolePrefab>,
    light: Option<LightPrefab>,
    camera: Option<CameraPrefab>,
    auto_fov: Option<AutoFov>,
    control_tag: Option<ControlTagPrefab>,
}
