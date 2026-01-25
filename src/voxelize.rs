use crate::barycentric::interpolate_uv;
use crate::color::*;
use crate::BrickType;
use crate::intersect::intersect;
use crate::octree::{Branches, TreeBody, VoxelTree};


use cgmath::{Vector2, Vector3, Vector4};
use image::RgbaImage;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct Triangle {
    material_id: Option<usize>,
    vertices: [Vector3<f32>; 3],
    uvs: Option<[Vector2<f32>; 3]>,
}

pub fn voxelize(
    models: &[tobj::Model],
    materials: &[RgbaImage],
    _scale: f32,
    _bricktype: BrickType,
    material_filter: Option<usize>,
) -> VoxelTree<Vector4<u8>> {
    let mut octree = VoxelTree::<Vector4<u8>>::new();

    // Determine model AABB to expand triangle octree to final size
    // Models are already scaled by scale_models()
    let u = &models[0].mesh.positions; // Guess initial
    let mut min = Vector3::new(u[0], u[1], u[2]);
    let mut max = min;

    for m in models.iter() {
        let p = &m.mesh.positions;
        for v in (0..p.len()).step_by(3) {
            for m in 0..3 {
                min[m] = min[m].min(p[v + m]);
                max[m] = max[m].max(p[v + m]);
            }
        }
    }

    let floor_min = Vector3::<isize>::new(
        min[0].floor() as isize - 1,
        min[1].floor() as isize - 1,
        min[2].floor() as isize - 1,
    );
    let ceil_max = Vector3::<isize>::new(
        max[0].ceil() as isize + 1,
        max[1].ceil() as isize + 1,
        max[2].ceil() as isize + 1,
    );

    while !octree.contains_bounds(floor_min) || !octree.contains_bounds(ceil_max) {
        octree.size += 1;
    }

    let mask = 1 << octree.size;

    // Voxelize
    let mut triangles = Vec::<Triangle>::new();
    for m in models.iter() {
        let mesh = &m.mesh;
        let material = mesh.material_id;

        // Skip if material doesn't match filter
        if let Some(filter_id) = material_filter {
            if material != Some(filter_id) {
                continue;
            }
        }

        for n in (0..mesh.indices.len()).step_by(3) {
            let mut m = (3 * mesh.indices[n]) as usize;
            let v0 = Vector3::new(
                mesh.positions[m],
                mesh.positions[m + 1],
                mesh.positions[m + 2],
            );
            m = (3 * mesh.indices[n + 1]) as usize;
            let v1 = Vector3::new(
                mesh.positions[m],
                mesh.positions[m + 1],
                mesh.positions[m + 2],
            );
            m = (3 * mesh.indices[n + 2]) as usize;
            let v2 = Vector3::new(
                mesh.positions[m],
                mesh.positions[m + 1],
                mesh.positions[m + 2],
            );

            let uvs = if !mesh.texcoords.is_empty() {
                m = (2 * mesh.indices[n]) as usize;
                let uv0 = Vector2::new(mesh.texcoords[m], mesh.texcoords[m + 1]);
                m = (2 * mesh.indices[n + 1]) as usize;
                let uv1 = Vector2::new(mesh.texcoords[m], mesh.texcoords[m + 1]);
                m = (2 * mesh.indices[n + 2]) as usize;
                let uv2 = Vector2::new(mesh.texcoords[m], mesh.texcoords[m + 1]);

                Some([uv0, uv1, uv2])
            } else {
                None
            };

            let triangle = Triangle {
                material_id: material,
                vertices: [v0, v1, v2],
                uvs,
            };

            triangles.push(triangle);
        }
    }

    recursive_voxelize(&mut octree.contents, mask, triangles, materials);

    octree
}

fn recursive_voxelize(
    branches: &mut Branches<Vector4<u8>>,
    mask: isize,
    vector: Vec<Triangle>,
    materials: &[RgbaImage],
) {
    let m = mask >> 1;
    let half_box = (2 * m + ((m == 0) as isize)) as f32 / 2.;

    for (i, branch) in branches.iter_mut().enumerate() {
        if let TreeBody::Empty = branch {
            let center = Vector3::<f32>::new(
                half_box * (2 * ((i & 4) > 0) as isize - 1) as f32,
                half_box * (2 * ((i & 2) > 0) as isize - 1) as f32,
                half_box * (2 * ((i & 1) > 0) as isize - 1) as f32,
            );

            let mut triangles = Vec::<Triangle>::new();
            let mut colors = Vec::<Vector4<u8>>::new();

            for triangle in &vector {
                match intersect(
                    half_box,
                    center,
                    triangle.vertices[0],
                    triangle.vertices[1],
                    triangle.vertices[2],
                ) {
                    Some(intersection) => {
                        // Only calculate colors if in root level
                        if m == 0 {
                            if let Some(id) = triangle.material_id {
                                let uv =
                                    interpolate_uv(&triangle.vertices, &triangle.uvs, intersection);
                                let m = &materials[id];

                                let u = ((uv[0] - uv[0].floor()) * (m.width() - 1) as f32) as u32;
                                let v =
                                    ((1. - uv[1] + uv[1].floor()) * (m.height() - 1) as f32) as u32;

                                let c = *m.get_pixel(u, v);
                                if c[3] == 0 {
                                    continue;
                                } // If alpha is zero, skeedaddle
                                colors.push(Vector4::<u8>::new(c[0], c[1], c[2], c[3]));
                            }
                        }
                    }
                    None => continue,
                }

                let mut cloned_triangle = *triangle;
                cloned_triangle.vertices[0] -= center;
                cloned_triangle.vertices[1] -= center;
                cloned_triangle.vertices[2] -= center;

                triangles.push(cloned_triangle);
            }

            if triangles.is_empty() {
                continue;
            }
            if m != 0 {
                // Not yet at root level, keep on recursing...
                *branch = TreeBody::Branch(Box::new(TreeBody::empty()));
                if let TreeBody::Branch(b) = branch {
                    recursive_voxelize(b, m, triangles, materials);
                }
            } else {
                *branch = TreeBody::Leaf(hsv2rgb(hsv_average(&colors)));
            }
        }
    }
}
