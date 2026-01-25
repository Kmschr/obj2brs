use crate::{Obj2Brs, SaveData};
use crate::error::{ConversionError, ConversionResult};
use std::path::PathBuf;
use brdb::{Entity, World};

pub fn write_brz(
    path: PathBuf,
    data: &SaveData,
    opts: &Obj2Brs,
    _use_procedural: bool,
    preview_image: Option<Vec<u8>>,
) -> ConversionResult<()> {
    let mut world = World::new();

    // Set Metadata
    if let Some(img) = preview_image {
        world.meta.screenshot = Some(img);
    }

    // Set Bundle Info from SaveData
    if let Some(stem) = path.file_stem() {
        world.meta.bundle.name = stem.to_string_lossy().to_string();
    }
    world.meta.bundle.authors = vec![data.author_name.clone()];
    world.meta.bundle.description = "Converted with obj2brs".to_string();

    // Copy bricks directly - they're already in brdb format
    world.bricks = data.bricks.clone();

    // Update material intensity for all bricks
    for brick in &mut world.bricks {
        brick.material_intensity = opts.material_intensity as u8;
    }

    match world.write_brz(&path) {
        Ok(_) => {
            opts.logger.log(format!("Successfully wrote BRZ to {:?}", path));
            Ok(())
        }
        Err(e) => Err(ConversionError::SaveWriteError(format!("Failed to write BRZ file: {:?}", e))),
    }
}

pub fn write_brz_grids(
    path: PathBuf,
    grids: Vec<(Entity, Vec<brdb::Brick>)>,
    opts: &Obj2Brs,
    preview_image: Option<Vec<u8>>,
) -> ConversionResult<()> {
    let mut world = World::new();

    // Set Metadata
    if let Some(img) = preview_image {
        world.meta.screenshot = Some(img);
    }

    // Set Bundle Info
    if let Some(stem) = path.file_stem() {
        world.meta.bundle.name = stem.to_string_lossy().to_string();
    }
    world.meta.bundle.authors = vec![opts.save_owner_name.clone()];
    world.meta.bundle.description = "Converted with obj2brs (split by material)".to_string();

    // Add each material's bricks as a separate frozen grid
    let total_bricks: usize = grids.iter().map(|(_, bricks)| bricks.len()).sum();
    for (entity, bricks) in grids {
        world.add_brick_grid(entity, bricks);
    }

    // Update material intensity for all bricks on main grid (should be empty but just in case)
    for brick in &mut world.bricks {
        brick.material_intensity = opts.material_intensity as u8;
    }

    opts.logger.log(format!("Total bricks across all grids: {}", total_bricks));

    match world.write_brz(&path) {
        Ok(_) => {
            opts.logger.log(format!("Successfully wrote BRZ with multiple grids to {:?}", path));
            Ok(())
        }
        Err(e) => Err(ConversionError::SaveWriteError(format!("Failed to write BRZ file: {:?}", e))),
    }
}
