use std::collections::HashMap;

use amethyst::{
    assets::{PrefabData, ProgressCounter},
    controls::ControlTagPrefab,
    derive::PrefabData,
    ecs::prelude::*,
    Error,
    utils::auto_fov::AutoFov,
};
use serde::{Deserialize, Serialize};

use amethyst_gltf::{GltfPrefab, GltfSceneAsset, GltfSceneFormat, GltfSceneLoaderSystemDesc, Load};

use crate::systems::{
    animal::{QuadrupedPrefab, TrackerPrefab},
    kinematics::{ChainPrefab, ConstrainPrefab},
    player::Player,
};

#[derive(Debug, Default, Clone, Serialize, Deserialize, PrefabData)]
#[serde(default)]
pub struct Extras {
    player: Option<Player>,
    quadruped: Option<QuadrupedPrefab>,
    tracker: Option<TrackerPrefab>,
    chain: Option<ChainPrefab>,
    constrain: Option<ConstrainPrefab>,
    auto_fov: Option<AutoFov>,
    control_tag: Option<ControlTagPrefab>,
}

impl Load for Extras {
    fn load_index(&mut self, node_map: &HashMap<usize, usize>) {
        let entity_index = |ref node_index| *node_map.get(node_index).unwrap();

        if let Some(ref mut quadruped) = self.quadruped {
            quadruped.anchors.iter_mut().for_each(|target| *target = entity_index(*target));
            quadruped.feet.iter_mut().for_each(|target| *target = entity_index(*target));
        }
        if let Some(ref mut tracker) = self.tracker {
            tracker.target = entity_index(tracker.target);
        }
        if let Some(ref mut chain) = self.chain {
            chain.target = entity_index(chain.target);
        }
        if let Some(ref mut constrain) = self.constrain {
            match *constrain {
                ConstrainPrefab::Direction(ref mut direction) => {
                    direction.target = entity_index(direction.target);
                }
                ConstrainPrefab::Pole(ref mut pole) => {
                    pole.target = entity_index(pole.target);
                }
                _ => {}
            }
        }
    }
}

pub type ScenePrefab = GltfPrefab<Extras>;
pub type SceneAsset = GltfSceneAsset<Extras>;
pub type SceneLoaderSystemDesc = GltfSceneLoaderSystemDesc<Extras>;
pub type SceneFormat = GltfSceneFormat;