use amethyst::{
    assets::{PrefabData, ProgressCounter},
    controls::ControlTagPrefab,
    derive::PrefabData,
    ecs::prelude::*,
    error::Error,
    utils::auto_fov::AutoFov,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use amethyst_gltf::{GltfPrefab, GltfSceneAsset, GltfSceneFormat, GltfSceneLoaderSystemDesc};
use redirect::{Redirect, RedirectItem as GenericRedirectItem};

use crate::systems::{
    animal::{QuadrupedPrefab, TrackerPrefab},
    kinematics::{ChainPrefab, ConstrainPrefab},
    player::Player,
};
use crate::systems::kinematics::{DirectionPrefab, PolePrefab};

type RedirectItem = GenericRedirectItem<String, usize>;

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

impl Redirect<String, usize> for QuadrupedPrefab {
    fn redirect<F>(self, map: &F) -> Self
        where F: Fn(String) -> usize {
        let map = |item: RedirectItem| item.redirect(map);

        let anchors = self.anchors.into_iter().map(map).collect_vec();
        let pivots = self.roots.into_iter().map(map).collect_vec();
        let feet = self.feet.into_iter().map(map).collect_vec();
        let root = map(self.root);

        QuadrupedPrefab {
            feet,
            anchors,
            roots: pivots,
            root,
            config: self.config,
        }
    }
}

impl Redirect<String, usize> for TrackerPrefab {
    fn redirect<F>(self, map: &F) -> Self
        where F: Fn(String) -> usize {
        let target = self.target.redirect(map);
        TrackerPrefab {
            target,
            limit: self.limit,
            speed: self.speed,
        }
    }
}

impl Redirect<String, usize> for ChainPrefab {
    fn redirect<F>(self, map: &F) -> Self
        where F: Fn(String) -> usize {
        let target = self.target.redirect(map);
        ChainPrefab {
            target,
            length: self.length,
        }
    }
}

impl Redirect<String, usize> for ConstrainPrefab {
    fn redirect<F>(self, map: &F) -> Self
        where F: Fn(String) -> usize {
        match self {
            ConstrainPrefab::Direction(direction) => {
                let target = direction.target.redirect(map);
                ConstrainPrefab::Direction(DirectionPrefab { target })
            }
            ConstrainPrefab::Pole(pole) => {
                let target = pole.target.redirect(map);
                ConstrainPrefab::Pole(PolePrefab { target })
            }
            _ => self,
        }
    }
}

impl Redirect<String, usize> for Extras {
    fn redirect<F>(self, map: &F) -> Self
        where F: Fn(String) -> usize {
        let mut extras = self.clone();

        if let Some(quadruped) = self.quadruped {
            extras.quadruped.replace(quadruped.redirect(map));
        };
        if let Some(tracker) = self.tracker {
            extras.tracker.replace(tracker.redirect(map));
        }
        if let Some(chain) = self.chain {
            extras.chain.replace(chain.redirect(map));
        }
        if let Some(constrain) = self.constrain {
            extras.constrain.replace(constrain.redirect(map));
        }

        extras
    }
}

pub type ScenePrefab = GltfPrefab<Extras>;
pub type SceneAsset = GltfSceneAsset<Extras>;
pub type SceneLoaderSystemDesc = GltfSceneLoaderSystemDesc<Extras>;
pub type SceneFormat = GltfSceneFormat;