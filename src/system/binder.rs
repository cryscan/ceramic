use std::{
    borrow::Cow,
    marker::PhantomData,
};

use amethyst::{
    assets::PrefabData,
    core::{bundle::SystemBundle, Named},
    derive::{PrefabData, SystemDesc},
    ecs::prelude::*,
    Error,
};
use serde::{Deserialize, Serialize};

use crate::system::{
    animal::Tracker,
    kinematics::{Chain, Direction, Hinge, Pole},
};

#[derive(Debug, Clone, Serialize, Deserialize, PrefabData)]
#[prefab(Component)]
pub struct Binder {
    pub name: Cow<'static, str>,
}

impl Component for Binder {
    type Storage = DenseVecStorage<Self>;
}

#[derive(SystemDesc)]
pub struct BinderSystem<T: Component + Clone> {
    _marker: PhantomData<T>,
}

impl<T: Component + Clone> Default for BinderSystem<T> {
    fn default() -> Self {
        BinderSystem { _marker: PhantomData }
    }
}

impl<'a, T: Component + Clone> System<'a> for BinderSystem<T> {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Binder>,
        ReadStorage<'a, Named>,
        WriteStorage<'a, T>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            binders,
            names,
            mut storage,
        ) = data;

        for (entity, binder) in (&*entities, &binders).join() {
            let component = storage.get(entity).cloned();
            for (entity, name) in (&*entities, &names).join() {
                if binder.name == name.name {
                    if let Some(component) = component {
                        storage.insert(entity, component).unwrap();
                    }
                    break;
                }
            }
            entities.delete(entity).unwrap();
        }
    }
}

#[derive(Default)]
pub struct BinderBundle;

impl BinderBundle {
    pub fn new() -> Self { BinderBundle }
}

macro_rules! impl_bundle {
    [$( $t: ty ),*] => {
        impl<'a, 'b> SystemBundle<'a, 'b> for BinderBundle {
            fn build(self, _world: &mut World, builder: &mut DispatcherBuilder<'a, 'b>) -> Result<(), Error> {
                $( builder.add(BinderSystem::<$t>::default(), concat!(stringify!("_", $t, "_binder")), &[]); )*
                Ok(())
            }
        }
    }
}

impl_bundle![Chain, Direction, Hinge, Pole, Tracker];