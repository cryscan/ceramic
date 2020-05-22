use amethyst::{
    assets::{PrefabData, ProgressCounter},
    controls::ControlTagPrefab,
    derive::PrefabData,
    ecs::prelude::*,
    Error,
    utils::auto_fov::AutoFov,
};
use serde::{Deserialize, Serialize};

use amethyst_gltf::{GltfPrefab, GltfSceneAsset, GltfSceneFormat, GltfSceneLoaderSystemDesc};

use crate::systems::{
    animal::TrackerPrefab,
    kinematics::{ChainPrefab, ConstrainPrefab},
    player::Player,
};

#[derive(Debug, Default, Clone, Serialize, Deserialize, PrefabData)]
#[serde(default)]
pub struct Extras {
    player: Option<Player>,
    tracker: Option<TrackerPrefab>,
    chain: Option<ChainPrefab>,
    constrain: Option<ConstrainPrefab>,
    auto_fov: Option<AutoFov>,
    control_tag: Option<ControlTagPrefab>,
}

pub type ScenePrefab = GltfPrefab<Extras>;
pub type SceneAsset = GltfSceneAsset<Extras>;
pub type SceneLoaderSystemDesc = GltfSceneLoaderSystemDesc<Extras>;
pub type SceneFormat = GltfSceneFormat;