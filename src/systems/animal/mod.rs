use std::{
    convert::TryInto,
    f32::consts::{FRAC_PI_4, PI, TAU},
};

use amethyst::{
    assets::PrefabData,
    core::math::{Complex, Point3, UnitQuaternion},
    ecs::prelude::*,
    error::Error,
};
use itertools::{Itertools, multizip};
use serde::{Deserialize, Serialize};

pub use bounce::BounceSystem;
pub use locomotion::{LocomotionSystem, OscillatorSystem};
use redirect::RedirectItem as GenericRedirectItem;
pub use track::TrackSystem;

pub mod bounce;
pub mod locomotion;
pub mod track;

type RedirectItem = GenericRedirectItem<String, usize>;

#[derive(Debug, Copy, Clone)]
pub struct Tracker {
    target: Entity,
    limit: Option<f32>,
    speed: f32,
    rotation: Option<UnitQuaternion<f32>>,
}

impl Component for Tracker {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerPrefab {
    pub target: RedirectItem,
    pub limit: Option<f32>,
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
        let target = self.target.clone().unwrap();
        let component = Tracker {
            target: entities[target],
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
    state: State,
    /// The home position of foot related to root.
    home: Option<Point3<f32>>,
    /// The original position of the anchor related to root.
    origin: Option<Point3<f32>>,

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

#[derive(Debug, Copy, Clone)]
pub struct Quadruped {
    limbs: [Limb; 4],
    root: Entity,
}

impl Component for Quadruped {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuadrupedPrefab {
    pub feet: Vec<RedirectItem>,
    pub anchors: Vec<RedirectItem>,
    pub roots: Vec<RedirectItem>,
    pub root: RedirectItem,

    #[serde(flatten)]
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
        let signals = (0..self.feet.len())
            .map(|i| {
                let ref radius = 1.0;
                let ref angle = (i as f32) * FRAC_PI_4;
                Complex::from_polar(radius, angle)
            })
            .collect_vec();
        let limbs = multizip((&self.feet, &self.anchors, &self.roots, signals))
            .map(|(foot, anchor, root, signal)| {
                let (foot, anchor, root) = multizip((foot.iter(), anchor.iter(), root.iter()))
                    .collect_vec()
                    .first()
                    .unwrap()
                    .clone();
                (foot, anchor, root, signal)
            })
            .map(|(foot, anchor, root, signal)| Limb {
                foot: entities[foot],
                anchor: entities[anchor],
                root: entities[root],
                state: State::Stance,
                home: None,
                origin: None,

                radius: 0.0,
                angular_velocity: 0.0,
                threshold: 0.0,
                duty_factor: 0.0,

                config: self.config.clone(),

                signal,
                transition: false,
            })
            .collect_vec()
            .as_slice()
            .try_into()
            .unwrap();
        let root = self.root.clone().unwrap();
        let component = Quadruped { limbs, root: entities[root] };

        data.insert(entity, component).map(|_| ()).map_err(Into::into)
    }
}