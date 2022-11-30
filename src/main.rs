mod barycentric;
mod color;
mod gui;
mod icon;
mod intersect;
mod octree;
mod palette;
mod rampify;
mod simplify;
mod voxelize;

use brickadia as brs;
use brs::save::Preview;
use cgmath::Vector4;
use eframe::{run_native, NativeOptions, epi::App, egui, egui::*};
use gui::bool_color;
use simplify::*;
use uuid::Uuid;
use rfd::FileDialog;
use std::{
    env,
    fs::File,
    path::Path, path::PathBuf, ops::RangeInclusive,
    thread,
    sync::mpsc, sync::mpsc::Receiver};
use voxelize::voxelize;

const WINDOW_WIDTH: f32 = 600.;
const WINDOW_HEIGHT: f32 = 480.;

const OBJ_ICON: &[u8; 10987] = include_bytes!("../res/obj_icon.png");

#[derive(Debug)]
pub struct Obj2Brs {
    pub bricktype: BrickType,
    input_file_path_receiver: Option<Receiver<Option<PathBuf>>>,
    input_file_path: String,
    pub match_brickadia_colorset: bool,
    material: Material,
    material_intensity: u32,
    output_directory_receiver: Option<Receiver<Option<PathBuf>>>,
    output_directory: String,
    save_owner_id: String,
    save_owner_name: String,
    raise: bool,
    rampify: bool,
    save_name: String,
    scale: f32,
    simplify: bool,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BrickType {
    Microbricks,
    Default,
    Tiles
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Material {
    Plastic,
    Glass,
    Glow,
    Metallic,
    Hologram,
    Ghost,
}

impl Default for Obj2Brs {
    fn default() -> Self {
        Self {
            bricktype: BrickType::Microbricks,
            input_file_path_receiver: None,
            input_file_path: "test.obj".into(),
            match_brickadia_colorset: false,
            material: Material::Plastic,
            material_intensity: 5,
            output_directory_receiver: None,
            output_directory: "builds".into(),
            save_owner_id: "d66c4ad5-59fc-4a9b-80b8-08dedc25bff9".into(),
            save_owner_name: "obj2brs".into(),
            raise: true,
            rampify: false,
            save_name: "test".into(),
            scale: 1.0,
            simplify: false,
        }
    }
}

impl App for Obj2Brs {
    fn update(&mut self, ctx: &egui::Context, _frame: &eframe::epi::Frame) {
        self.receive_file_dialog_messages();

        let input_file_valid = Path::new(&self.input_file_path).exists();
        let output_dir_valid = Path::new(&self.output_directory).is_dir();
        let uuid_valid = Uuid::parse_str(&self.save_owner_id).is_ok();
        let can_convert = input_file_valid && output_dir_valid && uuid_valid;

        CentralPanel::default().show(ctx, |ui: &mut Ui| {
            gui::add_grid(ui, |ui| {
                self.paths(ui, input_file_valid, output_dir_valid)
            });
            gui::add_horizontal_line(ui);
            gui::add_grid(ui, |ui| {
                self.options(ui, uuid_valid)
            });
            gui::info_text(ui);

            ui.add_space(10.);
            ui.vertical_centered(|ui| {
                if gui::button(ui, "Voxelize", can_convert) {
                    self.do_conversion()
                }
            });

            gui::footer(ctx);
        });
    }

    fn name(&self) -> &str {
        "obj2brs"
    }
}

impl Obj2Brs {
    fn receive_file_dialog_messages(&mut self) {
        if let Some(rx) = &self.input_file_path_receiver {
            if let Ok(data) = rx.try_recv() {
                self.input_file_path_receiver = None;
                if let Some(path) = data {
                    self.input_file_path = path.into_os_string().into_string().unwrap();
                }
            }
        }

        if let Some(rx) = &self.output_directory_receiver {
            if let Ok(data) = rx.try_recv() {
                self.output_directory_receiver = None;
                if let Some(path) = data {
                    self.output_directory = path.into_os_string().into_string().unwrap();
                }
            }
        }
    }

    fn paths(&mut self, ui: &mut Ui, input_file_valid: bool, output_dir_valid: bool) {
        let file_color = gui::bool_color(input_file_valid);

        ui.label("OBJ File").on_hover_text("Model to convert");
        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(&mut self.input_file_path).desired_width(400.0).text_color(file_color));
            if gui::file_button(ui) && self.input_file_path_receiver.is_none() {
                let (tx, rx) = mpsc::channel();
                self.input_file_path_receiver = Some(rx);
                thread::spawn(move || {
                    let obj_path = FileDialog::new().add_filter("OBJ", &["obj"]).pick_file();
                    tx.send(obj_path).unwrap();
                });
            }
        });
        ui.end_row();

        let dir_color = gui::bool_color(output_dir_valid);

        ui.label("Output Directory").on_hover_text("Where generated save will be written to");
        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(&mut self.output_directory).desired_width(400.0).text_color(dir_color));
            if gui::file_button(ui) && self.output_directory_receiver.is_none() {
                let (tx, rx) = mpsc::channel();
                self.output_directory_receiver = Some(rx);
                let default_dir = self.output_directory.clone();
                thread::spawn(move || {
                    let mut dialog = FileDialog::new();
                    if output_dir_valid {
                        dialog = dialog.set_directory(Path::new(default_dir.as_str()));
                    }
                    let output_dir = dialog.pick_folder();
                    tx.send(output_dir).unwrap();
                });
            }
        });
        ui.end_row();

        ui.label("Save Name").on_hover_text("Name for the brickadia savefile");
        ui.add(TextEdit::singleline(&mut self.save_name));
        ui.end_row();
    }

    fn options(&mut self, ui: &mut Ui, uuid_valid: bool) {

        ui.label("Lossy Conversion")
            .on_hover_text("Whether or not to merge similar bricks to create a less detailed model");
        ui.add_enabled(!self.rampify, Checkbox::new(&mut self.simplify, "Simplify (reduces brickcount)"));
        ui.end_row();

        ui.label("Raise Underground")
            .on_hover_text("Prevents bricks under the ground plate in Brickadia");
        ui.add(Checkbox::new(&mut self.raise, ""));
        ui.end_row();

        ui.label("Match to Colorset")
            .on_hover_text("Modify the color of the model to match the default color palette in Brickadia");
        ui.add_enabled(!self.rampify, Checkbox::new(&mut self.match_brickadia_colorset, "Use Default Palette"));
        ui.end_row();

        ui.label("Rampify")
            .on_hover_text("Creates a Lego-World like rampification of the model, uses default colorset");
        ui.add(Checkbox::new(&mut self.rampify, "Run the result through Wrapperup's plate-rampifier"));
        ui.end_row();

        ui.label("Scale")
            .on_hover_text("Adjusts the overall size of the generated save");
        ui.add(DragValue::new(&mut self.scale).min_decimals(2).prefix("x").speed(0.1));
        ui.end_row();

        ui.label("Bricktype")
            .on_hover_text("Which type of bricks will make up the generated save, use default to get a stud texture");
        ui.add_enabled_ui(!self.rampify, |ui| {
            ComboBox::from_label("")
                .selected_text(format!("{:?}", &mut self.bricktype))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.bricktype, BrickType::Microbricks, "Microbricks");
                    ui.selectable_value(&mut self.bricktype, BrickType::Default, "Default");
                    ui.selectable_value(&mut self.bricktype, BrickType::Tiles, "Tiles");
                });
        });
        ui.end_row();

        ui.label("Material");
        ComboBox::from_label("\n")
            .selected_text(format!("{:?}", &mut self.material))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.material, Material::Plastic, "Plastic");
                ui.selectable_value(&mut self.material, Material::Glass, "Glass");
                ui.selectable_value(&mut self.material, Material::Glow, "Glow");
                ui.selectable_value(&mut self.material, Material::Metallic, "Metallic");
                ui.selectable_value(&mut self.material, Material::Hologram, "Hologram");
                ui.selectable_value(&mut self.material, Material::Ghost, "Ghost");
            });
        ui.end_row();

        ui.label("Material Intensity");
        ui.add(Slider::new(&mut self.material_intensity, RangeInclusive::new(0, 10)));
        ui.end_row();

        let id_color = bool_color(uuid_valid);

        ui.label("Brick Owner").on_hover_text("Who will have ownership of the generated bricks");
        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(&mut self.save_owner_name).desired_width(100.0));
            ui.add(TextEdit::singleline(&mut self.save_owner_id).desired_width(300.0).text_color(id_color));
        });
        ui.end_row();
    }

    fn do_conversion(&mut self) {
        if self.rampify {
            self.simplify = false;
            self.match_brickadia_colorset = true;
            self.bricktype = BrickType::Default;
        }

        println!("{:?}", self);
        let mut octree = match generate_octree(self) {
            Ok(tree) => tree,
            Err(e) => {
                println!("{}", e);
                println!("Check that your .mtl file exists and doesn't contain any spaces in the filename!");
                println!("If your .mtl has spaces, rename the file and edit the .obj file to point to the new .mtl file");
                return;
            }
        };

        write_brs_data(
            &mut octree,
            self,
        );
    }
}

fn generate_octree(opt: &Obj2Brs) -> Result<octree::VoxelTree<Vector4<u8>>, String> {
    let p: &Path = opt.input_file_path.as_ref();
    println!("Loading {:?}", p);
    match File::open(p) {
        Ok(_f) => println!("success"),
        Err(e) => println!("{}", e.to_string())
    }

    println!("Importing model...");
    let (mut models, materials) = match tobj::load_obj(&opt.input_file_path, true) {
        Err(e) => return Err(format!("Error encountered when loading obj file: {}", e.to_string())),
        Ok(f) => f,
    };

    println!("Loading materials...");
    let mut material_images = Vec::<image::RgbaImage>::new();
    for material in materials {
        if material.diffuse_texture == "" {
            println!(
                "\tMaterial {} does not have an associated diffuse texture",
                material.name
            );

            // Create mock texture from diffuse color
            let mut image = image::RgbaImage::new(1, 1);

            image.put_pixel(0,0,
                image::Rgba([
                    color::ftoi(material.diffuse[0]),
                    color::ftoi(material.diffuse[1]),
                    color::ftoi(material.diffuse[2]),
                    color::ftoi(material.dissolve),
                ]),
            );

            material_images.push(image);
        } else {
            let image_path = Path::new(&opt.input_file_path).parent().unwrap().join(&material.diffuse_texture);
            println!(
                "\tLoading diffuse texture for {} from: {:?}",
                material.name, image_path
            );

            let image = match image::open(&image_path) {
                Err(e) => return Err(format!(
                    "Error encountered when loading {} texture file from {:?}: {}",
                    &material.diffuse_texture,
                    &image_path,
                    e.to_string()
                )),
                Ok(f) => f.into_rgba8(),
            };
            material_images.push(image);
        }
    }

    println!("Voxelizing...");
    Ok(voxelize(
        &mut models,
        &material_images,
        opt.scale,
        opt.bricktype,
    ))
}

fn write_brs_data(
    octree: &mut octree::VoxelTree<Vector4<u8>>,
    opts: &mut Obj2Brs,
) {
    let mut max_merge = 200;
    if opts.rampify {
        max_merge = 1;
    }

    let owner = brs::save::User {
        name: opts.save_owner_name.clone(),
        id: opts.save_owner_id.parse().unwrap(),
    };

    let mut write_data = brs::save::SaveData {
        header1: brs::save::Header1 {
            author: owner.clone(),
            host: Some(owner.clone()),
            ..Default::default()
        },
        header2: brs::save::Header2 {
            brick_assets:
                vec![
                    "PB_DefaultMicroBrick".into(),
                    "PB_DefaultBrick".into(),
                    "PB_DefaultRamp".into(),
                    "PB_DefaultWedge".into(),
                    "PB_DefaultTile".into(),
                ],
            materials: match opts.material {
                Material::Plastic => vec!["BMC_Plastic".into()],
                Material::Glass => vec!["BMC_Glass".into()],
                Material::Glow => vec!["BMC_Glow".into()],
                Material::Metallic => vec!["BMC_Metallic".into()],
                Material::Hologram => vec!["BMC_Hologram".into()],
                Material::Ghost => vec!["BMC_Ghost".into()],
            },
            brick_owners: vec![brs::save::BrickOwner::from_user_bricks(owner.clone(), 1)],
            colors: palette::DEFAULT_PALETTE.to_vec(),
            ..Default::default()
        },
        ..Default::default()
    };

    if opts.bricktype == BrickType::Tiles {
        write_data.header2.brick_assets[1] = "PB_DefaultTile".into();
    }

    println!("Simplifying...");
    if opts.simplify {
        simplify_lossy(octree, &mut write_data, opts, max_merge);
    } else {
        simplify_lossless(octree, &mut write_data, opts, max_merge);
    }

    if opts.raise {
        println!("Raising...");
        let mut min_z = 0;
        for brick in &write_data.bricks {
            let height = match brick.size {
                brs::save::Size::Procedural(_x, _y, z) => z,
                _ => 0
            };
            let z = brick.position.2 - height as i32;
            if z < min_z {
                min_z = z;
            }
        }

        for brick in &mut write_data.bricks {
            brick.position.2 -= min_z;
        }
    }

    if opts.rampify {
        rampify::rampify(&mut write_data);
    }

    // Write file
    println!("Writing {} bricks...", write_data.bricks.len());

    let preview = image::load_from_memory_with_format(OBJ_ICON, image::ImageFormat::Png).unwrap();

    let mut preview_bytes = Vec::new();
    preview.write_to(&mut preview_bytes, image::ImageOutputFormat::Png).unwrap();

    write_data.preview = Preview::PNG(preview_bytes);

    let output_file_path = opts.output_directory.clone() + "/" + &opts.save_name + ".brs";
    brs::write::SaveWriter::new(File::create(output_file_path).unwrap(), write_data)
        .write()
        .unwrap();

    println!("Save Written!");
}

fn main() {
    let build_dir = match env::consts::OS {
        "windows" => dirs::data_local_dir().unwrap().to_str().unwrap().to_string() + "\\Brickadia\\Saved\\Builds",
        "linux" => dirs::config_dir().unwrap().to_str().unwrap().to_string() + "/Epic/Brickadia/Saved/Builds",
        _ => String::new(),
    };

    let app = Obj2Brs {
        output_directory: build_dir,
        ..Default::default()
    };
    let win_option = NativeOptions {
        initial_window_size: Some([WINDOW_WIDTH, WINDOW_HEIGHT].into()),
        resizable: false,
        icon_data: Some(eframe::epi::IconData {
            rgba: icon::ICON.to_vec(),
            width: 32,
            height: 32,
        }),
        ..Default::default()
    };
    run_native(Box::new(app), win_option);
}
