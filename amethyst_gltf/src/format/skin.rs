use std::collections::HashMap;

use amethyst_animation::{JointPrefab, SkinnablePrefab, SkinPrefab};
use amethyst_assets::Prefab;
use amethyst_core::math::{convert, Matrix4};
use amethyst_error::Error;
use amethyst_rendy::skinning::JointTransformsPrefab;

use crate::GltfPrefab;

use super::Buffers;

pub fn load_skin<'a, T: Default>(
    skin: &gltf::Skin<'_>,
    buffers: &Buffers,
    skin_entity: usize,
    node_map: &HashMap<usize, usize>,
    meshes: Vec<usize>,
    prefab: &mut Prefab<GltfPrefab<T>>,
) -> Result<(), Error> {
    let joints = skin
        .joints()
        .map(|j| {
            node_map.get(&j.index()).cloned().expect(
                "Unreachable: `node_map` is initialized with the indexes from the `Gltf` object",
            )
        })
        .collect::<Vec<_>>();

    let reader = skin.reader(|buffer| buffers.buffer(&buffer));

    let inverse_bind_matrices = reader
        .read_inverse_bind_matrices()
        .map(|matrices| {
            matrices
                .map(Matrix4::from)
                .map(convert::<_, Matrix4<f32>>)
                .collect()
        })
        .unwrap_or_else(|| vec![Matrix4::identity(); joints.len()]);

    for (_bind_index, joint_index) in joints.iter().enumerate() {
        prefab
            .data_or_default(*joint_index)
            .skinnable
            .get_or_insert_with(SkinnablePrefab::default)
            .joint
            .get_or_insert_with(JointPrefab::default)
            .skins
            .push(skin_entity);
    }
    let joint_transforms = JointTransformsPrefab::new(skin_entity, joints.len());
    for mesh_index in &meshes {
        prefab
            .data_or_default(*mesh_index)
            .skinnable
            .get_or_insert_with(SkinnablePrefab::default)
            .joint_transforms = Some(joint_transforms.clone());
    }

    let skin_prefab = SkinPrefab {
        joints,
        meshes,
        bind_shape_matrix: Matrix4::identity(),
        inverse_bind_matrices,
    };
    prefab
        .data_or_default(skin_entity)
        .skinnable
        .get_or_insert_with(SkinnablePrefab::default)
        .skin = Some(skin_prefab);

    Ok(())
}
