use amethyst::{
    assets::{
        AssetPrefab, Handle, Prefab, PrefabData, PrefabLoader, PrefabLoaderSystemDesc,
        ProgressCounter,
    },
    controls::ControlTagPrefab,
    core::{bundle::SystemBundle, Transform},
    derive::PrefabData,
    ecs::prelude::*,
    Error,
    gltf::{GltfSceneAsset, GltfSceneFormat},
    prelude::*,
    prelude::World,
    renderer::{camera::CameraPrefab, light::LightPrefab},
    utils::auto_fov::AutoFov,
};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct Scene {
    pub(crate) handle: Option<Handle<Prefab<ScenePrefab>>>,
}

#[derive(Serialize, Deserialize, Default, PrefabData)]
#[serde(default)]
pub struct ScenePrefab {
    transform: Option<Transform>,
    model: Option<AssetPrefab<GltfSceneAsset, GltfSceneFormat>>,
    light: Option<LightPrefab>,
    camera: Option<CameraPrefab>,
    auto_fov: Option<AutoFov>,
    control_tag: Option<ControlTagPrefab>,
}