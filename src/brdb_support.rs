use brickadia as brs;
use std::path::PathBuf;
// Use root imports
use brdb::{Brick, BrickSize, BrickType, Collision, Color, Direction, Position, Rotation, World};
use brickadia::save::{Direction as BrsDirection, Rotation as BrsRotation};

pub fn write_brz(
    path: PathBuf,
    data: &brs::save::SaveData,
    use_procedural: bool,
    preview_image: Option<Vec<u8>>,
) {
    let mut world = World::new();

    // Set Metadata
    if let Some(img) = preview_image {
        world.meta.screenshot = Some(img);
    }

    // Set Bundle Info from SaveData
    // We can infer name from filename but path is PathBuf.
    if let Some(stem) = path.file_stem() {
        world.meta.bundle.name = stem.to_string_lossy().to_string();
    }
    world.meta.bundle.authors = vec![data.header1.author.name.clone()];
    world.meta.bundle.description = "Converted with obj2brs".to_string();

    let mut brdb_bricks = Vec::new();

    for brick in &data.bricks {
        let dir_enum = match &brick.direction {
            BrsDirection::XPositive => Direction::XPositive,
            BrsDirection::XNegative => Direction::XNegative,
            BrsDirection::YPositive => Direction::YPositive,
            BrsDirection::YNegative => Direction::YNegative,
            BrsDirection::ZPositive => Direction::ZPositive,
            BrsDirection::ZNegative => Direction::ZNegative,
        };

        let rot_enum = match &brick.rotation {
            BrsRotation::Deg0 => Rotation::Deg0,
            BrsRotation::Deg90 => Rotation::Deg90,
            BrsRotation::Deg180 => Rotation::Deg180,
            BrsRotation::Deg270 => Rotation::Deg270,
        };

        let asset_name = if (brick.asset_name_index as usize) < data.header2.brick_assets.len() {
            data.header2.brick_assets[brick.asset_name_index as usize].clone()
        } else {
            "PB_DefaultBrick".to_string()
        };

        let brick_type = if use_procedural
            || asset_name == "PB_DefaultTile"
            || asset_name == "PB_DefaultMicroBrick"
        {
            let size = match &brick.size {
                brs::save::Size::Procedural(x, y, z) => {
                    BrickSize::new(*x as u16, *y as u16, *z as u16)
                }
                _ => BrickSize::new(5, 5, 2),
            };
            BrickType::from((asset_name, size))
        } else {
            BrickType::from(asset_name)
        };

        let color = match &brick.color {
            brs::save::BrickColor::Unique(c) => Color::new(c.r, c.g, c.b),
            brs::save::BrickColor::Index(idx) => {
                if (*idx as usize) < data.header2.colors.len() {
                    let c = &data.header2.colors[*idx as usize];
                    Color::new(c.r, c.g, c.b)
                } else {
                    Color::new(255, 255, 255)
                }
            }
        };

        let new_brick = Brick {
            id: None,
            asset: brick_type,
            owner_index: None,
            position: Position {
                x: brick.position.0,
                y: brick.position.1,
                z: brick.position.2,
            },
            rotation: rot_enum,
            direction: dir_enum,
            collision: Collision {
                player: brick.collision.player,
                weapon: brick.collision.weapon,
                interact: brick.collision.interaction,
                tool: brick.collision.tool,
                physics: true,
                player1: Some(brick.collision.player),
                player2: Some(brick.collision.player),
                player3: Some(brick.collision.player),
            },
            visible: brick.visibility,
            color,
            material: "BMC_Plastic".into(),
            material_intensity: 5,
            components: Vec::new(),
        };
        brdb_bricks.push(new_brick);
    }

    world.bricks = brdb_bricks;

    match brdb::Brz::save(&path, &world) {
        Ok(_) => println!("Successfully wrote BRZ to {:?}", path),
        Err(e) => println!("Error writing BRZ: {:?}", e),
    }
}
