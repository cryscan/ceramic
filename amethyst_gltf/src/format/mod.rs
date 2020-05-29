//! GLTF format

use std::{cmp::Ordering, collections::HashMap, sync::Arc};

use gltf::{self, Gltf, khr_lights_punctual::Kind};
use log::debug;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use amethyst_animation::AnimationHierarchyPrefab;
use amethyst_assets::{Format, FormatValue, Prefab, PrefabData, Source};
use amethyst_core::{
    math::{convert, Quaternion, Unit, Vector3, Vector4},
    transform::Transform,
};
use amethyst_error::{Error, format_err, ResultExt};
use amethyst_rendy::{
    camera::CameraPrefab,
    light::{DirectionalLight, PointLight, SpotLight},
    palette::Srgb,
};
use redirect::Redirect;

use crate::{error, GltfMaterialSet, GltfNodeExtent, GltfPrefab, GltfSceneOptions, Named};

use self::{
    animation::load_animations,
    importer::{Buffers, get_image_data, ImageFormat, import},
    material::load_material,
    mesh::load_mesh,
    skin::load_skin,
};

mod animation;
mod importer;
mod material;
mod mesh;
mod skin;

pub trait Extra<'a> = Default + Redirect<String, usize> + Serialize + DeserializeOwned + PrefabData<'a>;

/// Gltf scene format, will load a single scene from a Gltf file.
///
/// Using the `GltfSceneLoaderSystem` a `Handle<GltfSceneAsset>` from this format can be attached
/// to an entity in ECS, and the system will then load the full scene using the given entity
/// as the root node of the scene hierarchy.
///
/// See `GltfSceneOptions` for more information about the load options.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GltfSceneFormat(pub GltfSceneOptions);

impl<'a, T> Format<Prefab<GltfPrefab<T>>> for GltfSceneFormat
    where T: Extra<'a> + 'static {
    fn name(&self) -> &'static str {
        "GLTFScene"
    }

    fn import(
        &self,
        name: String,
        source: Arc<dyn Source>,
        _create_reload: Option<Box<dyn Format<Prefab<GltfPrefab<T>>>>>,
    ) -> Result<FormatValue<Prefab<GltfPrefab<T>>>, Error> {
        Ok(FormatValue::data(
            load_gltf(source, &name, &self.0)
                .with_context(|_| format_err!("Failed to import gltf scene '{:?}'", name))?,
        ))
    }
}

fn load_gltf<'a, T>(
    source: Arc<dyn Source>,
    name: &str,
    options: &GltfSceneOptions,
) -> Result<Prefab<GltfPrefab<T>>, Error>
    where T: Extra<'a> {
    debug!("Loading GLTF scene '{}'", name);
    import(source.clone(), name)
        .with_context(|_| error::Error::GltfImporterError)
        .and_then(|(gltf, buffers)| {
            load_data(&gltf, &buffers, options, source, name).map_err(Into::into)
        })
}

fn load_data<'a, T>(
    gltf: &Gltf,
    buffers: &Buffers,
    options: &GltfSceneOptions,
    source: Arc<dyn Source>,
    name: &str,
) -> Result<Prefab<GltfPrefab<T>>, Error>
    where T: Extra<'a> {
    let scene_index = get_scene_index(gltf, options)?;
    let mut prefab = Prefab::<GltfPrefab<T>>::new();
    load_scene(
        gltf,
        scene_index,
        buffers,
        options,
        source,
        name,
        &mut prefab,
    )?;
    Ok(prefab)
}

fn get_scene_index(gltf: &Gltf, options: &GltfSceneOptions) -> Result<usize, Error> {
    let num_scenes = gltf.scenes().len();
    match (options.scene_index, gltf.default_scene()) {
        (Some(index), _) if index >= num_scenes => {
            Err(error::Error::InvalidSceneGltf(num_scenes).into())
        }
        (Some(index), _) => Ok(index),
        (None, Some(scene)) => Ok(scene.index()),
        (None, _) if num_scenes > 1 => Err(error::Error::InvalidSceneGltf(num_scenes).into()),
        (None, _) => Ok(0),
    }
}

fn load_scene<'a, T>(
    gltf: &Gltf,
    scene_index: usize,
    buffers: &Buffers,
    options: &GltfSceneOptions,
    source: Arc<dyn Source>,
    name: &str,
    prefab: &mut Prefab<GltfPrefab<T>>,
) -> Result<(), Error>
    where T: Extra<'a> {
    let scene = gltf
        .scenes()
        .nth(scene_index)
        .expect("Tried to load a scene which does not exist");
    let mut node_map = HashMap::new();
    let mut name_map = HashMap::new();
    let mut skin_map = HashMap::new();
    let mut bounding_box = GltfNodeExtent::default();
    let mut material_set = GltfMaterialSet::default();
    for node in scene.nodes() {
        let index = prefab.add(Some(0), None);
        load_node(
            gltf,
            &node,
            index,
            buffers,
            options,
            source.clone(),
            name,
            prefab,
            &mut node_map,
            &mut name_map,
            &mut skin_map,
            &mut bounding_box,
            &mut material_set,
        )?;
    }
    if bounding_box.valid() {
        prefab.data_or_default(0).extent = Some(bounding_box);
    }
    prefab.data_or_default(0).materials = Some(material_set);

    // load skins
    for (node_index, skin_info) in skin_map {
        load_skin(
            &gltf.skins().nth(skin_info.skin_index).expect(
                "Unreachable: `skin_map` is initialized with indexes from the `Gltf` object",
            ),
            buffers,
            *node_map
                .get(&node_index)
                .expect("Unreachable: `node_map` should contain all nodes present in `skin_map`"),
            &node_map,
            skin_info.mesh_indices,
            prefab,
        )?;
    }

    // load animations, if applicable
    if options.load_animations {
        let mut hierarchy_prefab = AnimationHierarchyPrefab::default();
        hierarchy_prefab.nodes = node_map
            .iter()
            .map(|(node, entity)| (*node, *entity))
            .collect();
        prefab
            .data_or_default(0)
            .animatable
            .get_or_insert_with(Default::default)
            .hierarchy = Some(hierarchy_prefab);

        prefab
            .data_or_default(0)
            .animatable
            .get_or_insert_with(Default::default)
            .animation_set = Some(load_animations(gltf, buffers, &node_map)?);
    }

    // redirect extras after loading all nodes
    redirect_extras(gltf, prefab, &node_map, &name_map)?;

    Ok(())
}

fn redirect_extras<'a, T>(
    gltf: &Gltf,
    prefab: &mut Prefab<GltfPrefab<T>>,
    node_map: &HashMap<usize, usize>,
    name_map: &HashMap<String, usize>,
) -> Result<(), Error>
    where T: Extra<'a> {
    for (node_index, ref _node) in gltf.nodes().enumerate() {
        let entity_index = node_map
            .get(&node_index)
            .expect("Unreachable: `node_map` should contain all nodes present in the scene");
        let ref name_map = |name: String| *name_map
            .get(name.as_str())
            .expect(
                format!(
                    "No such node with name {}",
                    name
                ).as_str()
            );
        if let Some(extras) = prefab.data_or_default(*entity_index).extras.take() {
            let extras = extras.redirect(name_map);
            prefab.data_or_default(*entity_index).extras.replace(extras);
        }
    }
    Ok(())
}

#[derive(Debug)]
struct SkinInfo {
    skin_index: usize,
    mesh_indices: Vec<usize>,
}

fn load_node<'a, T>(
    gltf: &Gltf,
    node: &gltf::Node<'_>,
    entity_index: usize,
    buffers: &Buffers,
    options: &GltfSceneOptions,
    source: Arc<dyn Source>,
    name: &str,
    prefab: &mut Prefab<GltfPrefab<T>>,
    node_map: &mut HashMap<usize, usize>,
    name_map: &mut HashMap<String, usize>,
    skin_map: &mut HashMap<usize, SkinInfo>,
    parent_bounding_box: &mut GltfNodeExtent,
    material_set: &mut GltfMaterialSet,
) -> Result<(), Error>
    where T: Extra<'a> {
    node_map.insert(node.index(), entity_index);

    // Load node name.
    if let Some(name) = node.name() {
        prefab.data_or_default(entity_index).name = Some(Named::new(name.to_string()));
        name_map.insert(name.to_string(), entity_index);
    }

    // Load transformation data, default will be identity
    let (translation, rotation, scale) = node.transform().decomposed();
    let mut local_transform = Transform::default();
    *local_transform.translation_mut() = convert::<_, Vector3<f32>>(Vector3::from(translation));
    *local_transform.rotation_mut() = Unit::new_normalize(convert::<_, Quaternion<f32>>(
        Quaternion::from(Vector4::from(rotation)),
    ));
    *local_transform.scale_mut() = convert::<_, Vector3<f32>>(Vector3::from(scale));
    prefab.data_or_default(entity_index).transform = Some(local_transform);

    // Load camera
    if let Some(camera) = node.camera() {
        prefab.data_or_default(entity_index).camera = Some(match camera.projection() {
            gltf::camera::Projection::Orthographic(proj) => CameraPrefab::Orthographic {
                left: -proj.xmag(),
                right: proj.xmag(),
                bottom: -proj.ymag(),
                top: proj.ymag(),
                znear: proj.znear(),
                zfar: proj.zfar(),
            },
            gltf::camera::Projection::Perspective(proj) => CameraPrefab::Perspective {
                aspect: proj.aspect_ratio().ok_or_else(|| {
                    format_err!(
                        "Camera {} is a perspective projection, but has no aspect ratio",
                        camera.index()
                    )
                }).unwrap_or(1.3),
                fovy: proj.yfov(),
                znear: proj.znear(),
                zfar: proj.zfar().ok_or_else(|| {
                    format_err!(
                        "Camera {} is perspective projection, but has no far plane",
                        camera.index()
                    )
                })?,
            },
        });

        if let Some(extras) = camera.extras() {
            prefab.data_or_default(entity_index).extras = Some(
                serde_json::from_str(&*extras.get())?
            );
        }
    }

    // load extras
    if let Some(extras) = node.extras() {
        prefab.data_or_default(entity_index).extras = Some(
            serde_json::from_str(&*extras.get())?
        );
    }

    // load lights
    if options.load_lights {
        if let Some(light) = node.light() {
            let color = light.color();
            let color = Srgb::new(color[0], color[1], color[2]);

            let intensity = light.intensity();
            let range = light.range().unwrap_or_default();
            prefab.data_or_default(entity_index).light = Some(match light.kind() {
                Kind::Directional => {
                    DirectionalLight {
                        color,
                        intensity,
                        direction: -Vector3::z(),
                    }.into()
                }
                Kind::Point => {
                    PointLight {
                        color,
                        intensity,
                        radius: range,
                        ..Default::default()
                    }.into()
                }
                Kind::Spot { inner_cone_angle: _, outer_cone_angle } => {
                    SpotLight {
                        angle: outer_cone_angle,
                        color,
                        direction: -Vector3::z(),
                        intensity,
                        range,
                        ..Default::default()
                    }.into()
                }
            }
            );
        }
    }

    // check for skinning
    let mut skin = node.skin().map(|skin| SkinInfo {
        skin_index: skin.index(),
        mesh_indices: Vec::default(),
    });

    let mut bounding_box = GltfNodeExtent::default();

    // load graphics
    if let Some(mesh) = node.mesh() {
        let mut graphics = load_mesh(&mesh, buffers, options)?;
        match graphics.len().cmp(&1) {
            Ordering::Equal => {
                // single primitive can be loaded directly onto the node
                let (mesh, material_index, bounds) = graphics.remove(0);
                bounding_box.extend_range(&bounds);
                let prefab_data = prefab.data_or_default(entity_index);
                prefab_data.mesh = Some(mesh);
                if let Some((material_id, material)) =
                material_index.and_then(|index| gltf.materials().nth(index).map(|m| (index, m)))
                {
                    material_set
                        .materials
                        .entry(material_id)
                        .or_insert(load_material(&material, buffers, source.clone(), name)?);
                    prefab_data.material_id = Some(material_id);
                }
                // if we have a skin we need to track the mesh entities
                if let Some(ref mut skin) = skin {
                    skin.mesh_indices.push(entity_index);
                }
            }
            Ordering::Greater => {
                // if we have multiple primitives,
                // we need to add each primitive as a child entity to the node
                for (mesh, material_index, bounds) in graphics {
                    let mesh_entity = prefab.add(Some(entity_index), None);
                    let prefab_data = prefab.data_or_default(mesh_entity);
                    prefab_data.transform = Some(Transform::default());
                    prefab_data.mesh = Some(mesh);
                    if let Some((material_id, material)) = material_index
                        .and_then(|index| gltf.materials().nth(index).map(|m| (index, m)))
                    {
                        material_set
                            .materials
                            .entry(material_id)
                            .or_insert(load_material(&material, buffers, source.clone(), name)?);
                        prefab_data.material_id = Some(material_id);
                    }

                    // if we have a skin we need to track the mesh entities
                    if let Some(ref mut skin) = skin {
                        skin.mesh_indices.push(mesh_entity);
                    }

                    // extent
                    bounding_box.extend_range(&bounds);
                    prefab_data.extent = Some(bounds.into());
                }
            }
            Ordering::Less => {}
        }
    }

    // load children
    for child in node.children() {
        let index = prefab.add(Some(entity_index), None);
        load_node(
            gltf,
            &child,
            index,
            buffers,
            options,
            source.clone(),
            name,
            prefab,
            node_map,
            name_map,
            skin_map,
            &mut bounding_box,
            material_set,
        )?;
    }
    if bounding_box.valid() {
        parent_bounding_box.extend(&bounding_box);
        prefab.data_or_default(entity_index).extent = Some(bounding_box);
    }

    // propagate skin information
    if let Some(skin) = skin {
        skin_map.insert(node.index(), skin);
    }

    Ok(())
}
