use std::{
    convert::TryInto,
    f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU},
    ops::Deref,
};

use amethyst::{
    assets::PrefabData,
    core::{math::{Complex, Point3, UnitQuaternion, Vector3}, Transform},
    ecs::{Component, prelude::*, storage::MaskedStorage},
    error::Error,
};
use itertools::{Itertools, multizip};
use serde::{Deserialize, Serialize};

pub use bounce::BounceSystem;
use ceramic_derive::Redirect;
pub use locomotion::{LocomotionSystem, OscillatorSystem};
use redirect::Redirect;
pub use track::TrackSystem;

use crate::{scene::RedirectField};
use crate::utils::transform::TransformTrait;

use super::player::Player;

pub mod bounce;
pub mod locomotion;
pub mod track;

#[derive(Debug, Copy, Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Tracker {
    target: Entity,
    limit: Option<f32>,
    speed: f32,
    rotation: Option<UnitQuaternion<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Redirect)]
pub struct TrackerPrefab {
    pub target: RedirectField,
    #[redirect(skip)]
    pub limit: Option<f32>,
    #[redirect(skip)]
    pub speed: f32,
}

impl<'a> PrefabData<'a> for TrackerPrefab {
    type SystemData = WriteStorage<'a, Tracker>;
    type Result = ();

    fn add_to_entity(
        &self,
        entity: Entity,
        data: &mut Self::SystemData,
        entities: &[Entity],
        _children: &[Entity],
    ) -> Result<Self::Result, Error> {
        let component = Tracker {
            target: self.target.clone().into_entity(entities),
            limit: self.limit.clone(),
            speed: self.speed,
            rotation: None,
        };
        data.insert(entity, component).map(|_| ()).map_err(Into::into)
    }
}

#[derive(Debug, Copy, Clone)]
enum State {
    Stance,
    Flight { stance: Point3<f32>, time: f32 },
}

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub max_angular_velocity: f32,
    pub max_duty_factor: f32,
    pub step_limit: [f32; 2],
    pub flight_time: f32,
    pub flight_factor: f32,
    pub stance_height: f32,
    pub bounce_factor: f32,
}

#[derive(Debug, Copy, Clone)]
pub struct Limb {
    foot: Entity,
    anchor: Entity,
    root: Entity,
    origin: Entity,
    home: Entity,

    state: State,
    radius: f32,
    angular_velocity: f32,
    /// The minimum angular velocity whose flight time is greater than `flight_time`.
    threshold: f32,
    duty_factor: f32,

    config: Config,

    signal: Complex<f32>,
    transition: bool,
}

impl Limb {
    fn match_speed(&mut self, speed: f32) {
        let ref config = self.config;
        let [min_step, max_step] = self.config.step_limit;

        // Increase angular speed to be maximum, and then increase radius.
        let min_radius = min_step / config.max_duty_factor / TAU;
        self.angular_velocity = (speed / min_radius).min(config.max_angular_velocity);
        self.radius = if self.angular_velocity > 0.0 { speed / self.angular_velocity } else { min_radius };

        // The step length at this situation to ensure the maximum duty factor and the maximum step length.
        let step_length = (TAU * self.radius * config.max_duty_factor).min(max_step);
        self.duty_factor = step_length / (TAU * self.radius);
        self.threshold = TAU * (1.0 - config.max_duty_factor) / config.flight_time;
    }

    fn step_radius(&self) -> f32 {
        PI * self.radius * self.duty_factor
    }

    fn flight_time(&self) -> f32 {
        if self.angular_velocity > self.threshold {
            TAU * (1.0 - self.duty_factor) / self.angular_velocity
        } else {
            self.config.flight_time
        }
    }
}

#[derive(Debug, Copy, Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Quadruped {
    limbs: [Limb; 4],
    root: Entity,
}

#[derive(Debug, Clone, Serialize, Deserialize, Redirect)]
pub struct QuadrupedPrefab {
    pub feet: Vec<RedirectField>,
    pub anchors: Vec<RedirectField>,
    pub roots: Vec<RedirectField>,
    pub origins: Vec<RedirectField>,
    pub homes: Vec<RedirectField>,
    pub root: RedirectField,

    #[serde(flatten)]
    #[redirect(skip)]
    pub config: Config,
}

impl<'a> PrefabData<'a> for QuadrupedPrefab {
    type SystemData = WriteStorage<'a, Quadruped>;
    type Result = ();

    fn add_to_entity(
        &self,
        entity: Entity,
        data: &mut Self::SystemData,
        entities: &[Entity],
        _children: &[Entity],
    ) -> Result<Self::Result, Error> {
        let signals = [0.0, FRAC_PI_4, FRAC_PI_2, 3.0 * FRAC_PI_4]
            .iter()
            .map(|angle| {
                let ref radius = 1.0;
                Complex::from_polar(radius, angle)
            })
            .collect_vec();
        let limbs = multizip((&self.feet, &self.anchors, &self.roots, &self.origins, &self.homes, signals))
            .map(|fields| {
                let (
                    foot,
                    anchor,
                    root,
                    origin,
                    home,
                    signal,
                ) = fields;

                Limb {
                    foot: foot.clone().into_entity(entities),
                    anchor: anchor.clone().into_entity(entities),
                    root: root.clone().into_entity(entities),
                    origin: origin.clone().into_entity(entities),
                    home: home.clone().into_entity(entities),

                    state: State::Stance,
                    radius: 0.0,
                    angular_velocity: 0.0,
                    threshold: 0.0,
                    duty_factor: 0.0,

                    config: self.config.clone(),

                    signal,
                    transition: false,
                }
            })
            .collect_vec()
            .as_slice()
            .try_into()
            .unwrap();

        let component = Quadruped {
            limbs,
            root: self.root.clone().into_entity(entities),
        };
        data.insert(entity, component).map(|_| ()).map_err(Into::into)
    }
}

#[inline]
fn limb_velocity<D>(
    transforms: &Storage<'_, Transform, D>,
    entity: Entity,
    limb: &Limb,
    player: &Player,
) -> Option<Vector3<f32>>
    where D: Deref<Target=MaskedStorage<Transform>> {
    let ref home = transforms.get(limb.home)?.global_position();
    let root = transforms.get(entity)?.global_position();

    let ref radial = home - root;
    let ref angular = player.rotation().scaled_axis();
    let ref linear = player.velocity();

    let transform = transforms.get(entity)?.global_matrix();
    let angular = transform.transform_vector(angular);
    let linear = transform.transform_vector(linear);
    Some(linear + angular.cross(radial))
}