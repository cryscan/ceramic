use amethyst::{
    assets::{PrefabData, ProgressCounter},
    controls::ControlTagPrefab,
    derive::PrefabData,
    ecs::prelude::*,
    error::Error,
    utils::auto_fov::AutoFov,
};
use serde::{Deserialize, Serialize};

use amethyst_gltf::{GltfPrefab, GltfSceneAsset, GltfSceneFormat, GltfSceneLoaderSystemDesc};
use ceramic_derive::Redirect;
use redirect::Redirect;

use crate::systems::{
    animal::{QuadrupedPrefab, TrackerPrefab},
    kinematics::{ChainPrefab, ConstrainPrefab},
    particle::{ParticlePrefab, SpringPrefab},
    player::Player,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RedirectField {
    Origin(String),
    Target(usize),
}

impl Redirect<String, usize> for RedirectField {
    fn redirect<F>(self, map: &F) -> Self
        where F: Fn(String) -> usize {
        match self {
            RedirectField::Origin(origin) => RedirectField::Target(map(origin)),
            RedirectField::Target(_) => self,
        }
    }
}

impl RedirectField {
    pub fn into_entity(self, entities: &[Entity]) -> Entity {
        let index = match self {
            RedirectField::Origin(_) => panic!("Redirect field unsolved"),
            RedirectField::Target(target) => target,
        };
        entities[index]
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PrefabData, Redirect)]
#[serde(default)]
pub struct Extras {
    #[redirect(skip)]
    player: Option<Player>,
    quadruped: Option<QuadrupedPrefab>,
    tracker: Option<TrackerPrefab>,
    chain: Option<ChainPrefab>,
    constrain: Option<ConstrainPrefab>,
    #[redirect(skip)]
    particle: Option<ParticlePrefab>,
    spring: Option<SpringPrefab>,
    #[redirect(skip)]
    auto_fov: Option<AutoFov>,
    #[redirect(skip)]
    control_tag: Option<ControlTagPrefab>,
}

pub type ScenePrefab = GltfPrefab<Extras>;
pub type SceneAsset = GltfSceneAsset<Extras>;
pub type SceneLoaderSystemDesc = GltfSceneLoaderSystemDesc<Extras>;
pub type SceneFormat = GltfSceneFormat;