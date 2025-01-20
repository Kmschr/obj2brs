use crate::color::*;
use crate::octree::{TreeBody, VoxelTree};
use crate::BrickType;
use crate::Obj2Brs;

use brickadia::save as brs;
use cgmath::{Vector3, Vector4};

pub fn simplify_lossy(
    octree: &mut VoxelTree<Vector4<u8>>,
    save_data: &mut brs::SaveData,
    opts: &Obj2Brs,
    max_merge: isize,
) {
    let colorset = convert_colorset_to_hsv(&save_data.header2.colors);
    let scales: (isize, isize, isize) = if opts.bricktype == BrickType::Microbricks {
        (opts.brick_scale, opts.brick_scale, opts.brick_scale)
    } else {
        (5, 5, 2)
    };

    let max_scale = isize::max(isize::max(scales.0, scales.1), scales.2);
    let max_merge = max_merge / max_scale;

    loop {
        let mut colors = Vec::<Vector4<u8>>::new();
        let (x, y, z);
        {
            let (location, voxel) = octree.get_any_mut_or_create();

            x = location[0];
            y = location[1];
            z = location[2];

            match voxel {
                TreeBody::Leaf(leaf_color) => {
                    colors.push(*leaf_color);
                }
                _ => break,
            }
        }

        let mut xp = x + 1;
        let mut yp = y + 1;
        let mut zp = z + 1;

        // Expand z direction first due to octree ordering followed by y and x
        // Ensures blocks are simplified in the pattern of Morton coding
        // Saves us having to check in the negative directions
        while zp - z < max_merge {
            let voxel = octree.get_mut_or_create(Vector3::new(x, y, zp));
            match voxel {
                TreeBody::Leaf(leaf_color) => {
                    colors.push(*leaf_color);
                    zp += 1
                }
                _ => break,
            }
        }

        while yp - y < max_merge {
            let mut pass = true;
            for sz in z..zp {
                let voxel = octree.get_mut_or_create(Vector3::new(x, yp, sz));
                match voxel {
                    TreeBody::Leaf(leaf_color) => colors.push(*leaf_color),
                    _ => {
                        pass = false;
                        break;
                    }
                }
            }
            if !pass {
                break;
            }
            yp += 1;
        }

        while xp - x < max_merge {
            let mut pass = true;
            for sy in y..yp {
                for sz in z..zp {
                    let voxel = octree.get_mut_or_create(Vector3::new(xp, sy, sz));
                    match voxel {
                        TreeBody::Leaf(leaf_color) => colors.push(*leaf_color),
                        _ => {
                            pass = false;
                            break;
                        }
                    }
                }
                if !pass {
                    break;
                }
            }
            if !pass {
                break;
            }
            xp += 1;
        }

        // Clear nodes
        // This cant be done during the loops above unless you keep track
        // of which nodes you have already deleted
        for sx in x..xp {
            for sy in y..yp {
                for sz in z..zp {
                    let voxel = octree.get_mut_or_create(Vector3::new(sx, sy, sz));
                    *voxel = TreeBody::Empty;
                }
            }
        }

        let avg_color = hsv_average(&colors);
        let color = if opts.match_brickadia_colorset {
            brs::BrickColor::Index(match_hsv_to_colorset(&colorset, &avg_color) as u32)
        } else {
            let rgba = gamma_correct(hsv2rgb(avg_color));
            brs::BrickColor::Unique(brs::Color {
                r: rgba[0],
                g: rgba[1],
                b: rgba[2],
                a: rgba[3],
            })
        };

        let width = xp - x;
        let height = yp - y;
        let depth = zp - z;

        save_data.bricks.push(brs::Brick {
            asset_name_index: if opts.bricktype == BrickType::Microbricks {
                0
            } else {
                1
            },
            // Coordinates are rotated
            size: scaled_size(scales, (width, depth, height)),
            position: scaled_pos(scales, (width, depth, height), (x, z, y)),
            color,
            material_intensity: opts.material_intensity,
            ..Default::default()
        });
    }
}

pub fn simplify_lossless(
    octree: &mut VoxelTree<Vector4<u8>>,
    save_data: &mut brs::SaveData,
    opts: &Obj2Brs,
    max_merge: isize,
) {
    let d: isize = 1 << octree.size;
    let len = d + 1;

    let colorset = convert_colorset_to_hsv(&save_data.header2.colors);

    let scales: (isize, isize, isize) = if opts.bricktype == BrickType::Microbricks {
        (opts.brick_scale, opts.brick_scale, opts.brick_scale)
    } else {
        (5, 5, 2)
    };

    let max_scale = isize::max(isize::max(scales.0, scales.1), scales.2);
    let max_merge = max_merge / max_scale;

    loop {
        let matched_color;
        let unmatched_color;
        let x;
        let y;
        let z;
        {
            let (location, voxel) = octree.get_any_mut_or_create();

            x = location[0];
            y = location[1];
            z = location[2];

            match voxel {
                TreeBody::Leaf(leaf_color) => {
                    let final_color = gamma_correct(*leaf_color);
                    matched_color = match_hsv_to_colorset(&colorset, &rgb2hsv(final_color));
                    unmatched_color = brs::BrickColor::Unique(brs::Color {
                        r: final_color[0],
                        g: final_color[1],
                        b: final_color[2],
                        a: final_color[3],
                    });
                }
                _ => break,
            }
        }

        let mut xp = x + 1;
        let mut yp = y + 1;
        let mut zp = z + 1;

        // Expand z direction first due to octree ordering followed by y
        // Ensures blocks are simplified in the pattern of Morton coding
        while zp < len && (zp - z) < max_merge {
            let voxel = octree.get_mut_or_create(Vector3::new(x, y, zp));
            match voxel {
                TreeBody::Leaf(leaf_color) => {
                    let final_color = gamma_correct(*leaf_color);
                    let color_temp = match_hsv_to_colorset(&colorset, &rgb2hsv(final_color));
                    if color_temp != matched_color {
                        break;
                    }
                    zp += 1;
                }
                _ => break,
            }
        }

        while yp < len && (yp - y) < max_merge {
            let mut pass = true;
            for sz in z..zp {
                let voxel = octree.get_mut_or_create(Vector3::new(x, yp, sz));
                match voxel {
                    TreeBody::Leaf(leaf_color) => {
                        let final_color = gamma_correct(*leaf_color);
                        let color_temp = match_hsv_to_colorset(&colorset, &rgb2hsv(final_color));
                        if color_temp != matched_color {
                            pass = false;
                            break;
                        }
                    }
                    _ => {
                        pass = false;
                        break;
                    }
                }
            }
            if !pass {
                break;
            }
            yp += 1;
        }

        while xp < len && (xp - x) < max_merge {
            let mut pass = true;
            for sy in y..yp {
                for sz in z..zp {
                    let voxel = octree.get_mut_or_create(Vector3::new(xp, sy, sz));
                    match voxel {
                        TreeBody::Leaf(leaf_color) => {
                            let final_color = gamma_correct(*leaf_color);
                            let color_temp =
                                match_hsv_to_colorset(&colorset, &rgb2hsv(final_color));
                            if color_temp != matched_color {
                                pass = false;
                                break;
                            }
                        }
                        _ => {
                            pass = false;
                            break;
                        }
                    }
                }
                if !pass {
                    break;
                }
            }
            if !pass {
                break;
            }
            xp += 1;
        }

        // Clear nodes
        // This cant be done during the loops above unless you keep track
        // of which nodes you have already deleted
        for sx in x..xp {
            for sy in y..yp {
                for sz in z..zp {
                    let voxel = octree.get_mut_or_create(Vector3::new(sx, sy, sz));
                    *voxel = TreeBody::Empty;
                }
            }
        }

        let width = xp - x;
        let height = yp - y;
        let depth = zp - z;

        let color = if opts.match_brickadia_colorset {
            brs::BrickColor::Index(matched_color as u32)
        } else {
            unmatched_color
        };

        save_data.bricks.push(brs::Brick {
            asset_name_index: if opts.bricktype == BrickType::Microbricks {
                0
            } else {
                1
            },
            // Coordinates are rotated
            size: scaled_size(scales, (width, depth, height)),
            position: scaled_pos(scales, (width, depth, height), (x, z, y)),
            color,
            owner_index: 1,
            material_intensity: opts.material_intensity,
            ..Default::default()
        });
    }
}

fn scaled_size(scale: (isize, isize, isize), size: (isize, isize, isize)) -> brs::Size {
    brs::Size::Procedural(
        (scale.0 * size.0) as u32,
        (scale.1 * size.1) as u32,
        (scale.2 * size.2) as u32,
    )
}

fn scaled_pos(
    scale: (isize, isize, isize),
    size: (isize, isize, isize),
    pos: (isize, isize, isize),
) -> (i32, i32, i32) {
    (
        (scale.0 * size.0 + 2 * scale.0 * pos.0) as i32,
        (scale.1 * size.1 + 2 * scale.1 * pos.1) as i32,
        (scale.2 * size.2 + 2 * scale.2 * pos.2) as i32,
    )
}
