//! Small original mesh proxies. All visible pieces belong to one stock-lot root.

use bevy::prelude::*;

use super::{shape, InventoryVisual, LotPick, Palette};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FoodVisual {
    Apple,
    Avocado,
    Milk,
    CannedBeans,
    CannedTomatoes,
    Sardines,
    Banana,
    Eggs,
    Pasta,
    Spinach,
    DishSoap,
    Yogurt,
    Unknown,
}

fn visual_for(name: &str) -> FoodVisual {
    // Receipt names are user editable. Only known product classes get a
    // recognizable shape; a shared word is not evidence of form or packaging.
    let name = name.trim().to_ascii_lowercase();
    match name.as_str() {
        "apple"
        | "apples"
        | "gala apple"
        | "gala apples"
        | "honeycrisp apple"
        | "honeycrisp apples"
        | "granny smith apple"
        | "granny smith apples" => FoodVisual::Apple,
        "avocado" | "avocados" | "fresh avocado" | "fresh avocados" => FoodVisual::Avocado,
        "milk" | "milk 2%" | "whole milk" | "skim milk" | "2% milk" => FoodVisual::Milk,
        "canned beans" | "beans can" | "black beans can" | "canned black beans" => {
            FoodVisual::CannedBeans
        }
        "canned tomatoes" | "tomatoes can" | "diced tomatoes can" | "crushed tomatoes can" => {
            FoodVisual::CannedTomatoes
        }
        "sardines" | "canned sardines" => FoodVisual::Sardines,
        "banana" | "bananas" => FoodVisual::Banana,
        "eggs" | "eggs lg" | "large eggs" => FoodVisual::Eggs,
        "pasta" | "dry pasta" => FoodVisual::Pasta,
        "spinach" | "fresh spinach" | "bagged spinach" => FoodVisual::Spinach,
        "dish soap" => FoodVisual::DishSoap,
        "yogurt" | "plain yogurt" | "greek yogurt" => FoodVisual::Yogurt,
        _ => FoodVisual::Unknown,
    }
}

fn piece<S: Into<Mesh>>(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    root: Entity,
    material: &Handle<StandardMaterial>,
    primitive: S,
    mut transform: Transform,
    pickable: bool,
) -> Entity {
    transform.scale *= 1.22;
    let entity = shape(commands, meshes, material, primitive, transform, pickable);
    commands.entity(entity).insert(ChildOf(root));
    entity
}

fn box_piece(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    root: Entity,
    material: &Handle<StandardMaterial>,
    at: Vec3,
    size: Vec3,
    pickable: bool,
) {
    piece(
        commands,
        meshes,
        root,
        material,
        Cuboid::new(size.x, size.y, size.z),
        Transform::from_translation(at),
        pickable,
    );
}

#[allow(clippy::too_many_arguments)]
fn cylinder_piece(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    root: Entity,
    material: &Handle<StandardMaterial>,
    at: Vec3,
    radius: f32,
    height: f32,
    pickable: bool,
) {
    piece(
        commands,
        meshes,
        root,
        material,
        Cylinder::new(radius, height),
        Transform::from_translation(at),
        pickable,
    );
}

#[allow(clippy::too_many_arguments)]
fn sphere_piece(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    root: Entity,
    material: &Handle<StandardMaterial>,
    at: Vec3,
    radius: f32,
    scale: Vec3,
    pickable: bool,
) {
    piece(
        commands,
        meshes,
        root,
        material,
        Sphere::new(radius),
        Transform::from_translation(at).with_scale(scale),
        pickable,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_lot(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    asset_server: Option<&AssetServer>,
    p: &Palette,
    id: &str,
    name: &str,
    at: Vec3,
    selected: bool,
) {
    // The pointer observer resolves this root through ChildOf, including mesh
    // descendants created asynchronously by an imported glTF scene.
    let root = commands
        .spawn((
            Transform::IDENTITY,
            Visibility::Visible,
            InventoryVisual,
            LotPick(id.to_string()),
        ))
        .id();
    piece(
        commands,
        meshes,
        root,
        &p.shadow,
        Cylinder::new(0.22, 0.004),
        Transform::from_translation(at + Vec3::new(0.0, -0.225, 0.0))
            .with_scale(Vec3::new(1.0, 1.0, 0.62)),
        false,
    );
    if selected {
        cylinder_piece(
            commands,
            meshes,
            root,
            &p.focus,
            at + Vec3::new(0.0, -0.23, 0.0),
            0.34,
            0.018,
            false,
        );
    }
    let visual = visual_for(name);
    let model = match visual {
        FoodVisual::Apple => Some((
            "models/polyhaven/food_apple_01/food_apple_01_1k.gltf#Scene0",
            3.5,
        )),
        FoodVisual::Avocado => Some((
            "models/polyhaven/food_avocado_01/food_avocado_01_1k.gltf#Scene0",
            3.3,
        )),
        FoodVisual::Milk => Some((
            "models/polyhaven/long_life_food/generic_milk.gltf#Scene0",
            2.1,
        )),
        FoodVisual::CannedBeans => Some((
            "models/polyhaven/long_life_food/generic_beans.gltf#Scene0",
            2.6,
        )),
        FoodVisual::CannedTomatoes => Some((
            "models/polyhaven/long_life_food/generic_tomatoes.gltf#Scene0",
            2.7,
        )),
        FoodVisual::Sardines => Some((
            "models/polyhaven/long_life_food/generic_sardines.gltf#Scene0",
            2.5,
        )),
        _ => None,
    };
    if let (Some(asset_server), Some((path, scale))) = (asset_server, model) {
        commands.spawn((
            SceneRoot(asset_server.load(path)),
            Transform::from_translation(at + Vec3::new(0.0, -0.225, 0.0))
                .with_scale(Vec3::splat(scale)),
            ChildOf(root),
        ));
        // The source packaging artwork is omitted from the bundle. These
        // simple local color blocks are original, generic front labels.
        if visual == FoodVisual::Milk {
            box_piece(
                commands,
                meshes,
                root,
                &p.blue,
                at + Vec3::new(0.0, -0.02, 0.080),
                Vec3::new(0.18, 0.20, 0.008),
                true,
            );
            box_piece(
                commands,
                meshes,
                root,
                &p.milk,
                at + Vec3::new(0.0, -0.02, 0.089),
                Vec3::new(0.10, 0.065, 0.009),
                true,
            );
        } else if matches!(visual, FoodVisual::CannedBeans | FoodVisual::CannedTomatoes) {
            box_piece(
                commands,
                meshes,
                root,
                &p.milk,
                at + Vec3::new(0.0, -0.03, 0.122),
                Vec3::new(0.14, 0.15, 0.008),
                true,
            );
        }
        return;
    }
    if visual == FoodVisual::Banana {
        // Tapered overlapping ellipsoids form three curved fruit instead of rods.
        for bunch in 0..3 {
            let z = (bunch as f32 - 1.0) * 0.105;
            for part in 0..5 {
                let offset = part as f32 - 2.0;
                let radius = [0.032, 0.052, 0.061, 0.052, 0.032][part];
                let pos = at + Vec3::new(offset * 0.105, -0.13 + offset * offset * 0.031, z);
                piece(
                    commands,
                    meshes,
                    root,
                    &p.banana,
                    Sphere::new(1.0),
                    Transform::from_translation(pos)
                        .with_rotation(Quat::from_rotation_z(offset * 0.18))
                        .with_scale(Vec3::new(0.126, radius, radius * 1.13)),
                    true,
                );
            }
            for side in [-1.0, 1.0] {
                sphere_piece(
                    commands,
                    meshes,
                    root,
                    &p.wood_edge,
                    at + Vec3::new(side * 0.272, 0.035, z),
                    0.028,
                    Vec3::new(0.65, 0.72, 0.60),
                    true,
                );
            }
        }
        cylinder_piece(
            commands,
            meshes,
            root,
            &p.leaf,
            at + Vec3::new(0.27, -0.06, 0.0),
            0.028,
            0.12,
            true,
        );
    } else if visual == FoodVisual::Milk {
        box_piece(
            commands,
            meshes,
            root,
            &p.milk,
            at,
            Vec3::new(0.33, 0.35, 0.30),
            true,
        );
        for side in [-1.0, 1.0] {
            piece(
                commands,
                meshes,
                root,
                &p.milk,
                Cuboid::new(0.19, 0.12, 0.30),
                Transform::from_translation(at + Vec3::new(side * 0.085, 0.224, 0.0))
                    .with_rotation(Quat::from_rotation_z(-side * 0.47)),
                true,
            );
        }
        box_piece(
            commands,
            meshes,
            root,
            &p.blue,
            at + Vec3::new(0.0, 0.278, 0.0),
            Vec3::new(0.10, 0.045, 0.31),
            true,
        );
        box_piece(
            commands,
            meshes,
            root,
            &p.blue,
            at + Vec3::new(0.0, 0.035, 0.157),
            Vec3::new(0.31, 0.165, 0.012),
            true,
        );
        for side in [-1.0, 1.0] {
            box_piece(
                commands,
                meshes,
                root,
                &p.blue,
                at + Vec3::new(side * 0.169, 0.035, 0.0),
                Vec3::new(0.012, 0.165, 0.28),
                true,
            );
        }
        sphere_piece(
            commands,
            meshes,
            root,
            &p.milk,
            at + Vec3::new(0.0, 0.035, 0.170),
            0.066,
            Vec3::new(0.70, 0.93, 0.12),
            true,
        );
        box_piece(
            commands,
            meshes,
            root,
            &p.milk,
            at + Vec3::new(0.0, -0.091, 0.159),
            Vec3::new(0.24, 0.06, 0.012),
            true,
        );
        cylinder_piece(
            commands,
            meshes,
            root,
            &p.blue,
            at + Vec3::new(0.035, 0.33, 0.0),
            0.052,
            0.055,
            true,
        );
        for dx in [-0.085, 0.085] {
            box_piece(
                commands,
                meshes,
                root,
                &p.blue,
                at + Vec3::new(dx, -0.155, 0.159),
                Vec3::new(0.028, 0.018, 0.012),
                true,
            );
        }
    } else if visual == FoodVisual::Eggs {
        box_piece(
            commands,
            meshes,
            root,
            &p.carton,
            at + Vec3::new(0.0, -0.13, 0.0),
            Vec3::new(0.58, 0.09, 0.35),
            true,
        );
        for row in 0..2 {
            for col in 0..3 {
                sphere_piece(
                    commands,
                    meshes,
                    root,
                    &p.egg,
                    at + Vec3::new((col as f32 - 1.0) * 0.17, -0.01, (row as f32 - 0.5) * 0.16),
                    0.085,
                    Vec3::new(0.82, 1.17, 0.82),
                    true,
                );
            }
        }
        box_piece(
            commands,
            meshes,
            root,
            &p.carton,
            at + Vec3::new(0.0, -0.035, -0.20),
            Vec3::new(0.58, 0.11, 0.035),
            true,
        );
    } else if visual == FoodVisual::Pasta {
        box_piece(
            commands,
            meshes,
            root,
            &p.pasta,
            at,
            Vec3::new(0.31, 0.45, 0.16),
            true,
        );
        box_piece(
            commands,
            meshes,
            root,
            &p.milk,
            at + Vec3::new(0.0, 0.045, 0.086),
            Vec3::new(0.24, 0.19, 0.012),
            true,
        );
        for dx in [-0.06, 0.0, 0.06] {
            box_piece(
                commands,
                meshes,
                root,
                &p.banana,
                at + Vec3::new(dx, 0.03, 0.095),
                Vec3::new(0.025, 0.13, 0.006),
                true,
            );
        }
    } else if visual == FoodVisual::Spinach {
        box_piece(
            commands,
            meshes,
            root,
            &p.plastic,
            at,
            Vec3::new(0.38, 0.37, 0.22),
            true,
        );
        for i in 0..7 {
            let dx = (i % 3) as f32 * 0.105 - 0.11;
            let dy = (i / 3) as f32 * 0.09 - 0.075;
            sphere_piece(
                commands,
                meshes,
                root,
                &p.leaf,
                at + Vec3::new(dx, dy, 0.13),
                0.075,
                Vec3::new(1.0, 0.62, 0.28),
                true,
            );
        }
        box_piece(
            commands,
            meshes,
            root,
            &p.milk,
            at + Vec3::new(0.0, -0.10, 0.16),
            Vec3::new(0.30, 0.08, 0.008),
            true,
        );
    } else if visual == FoodVisual::DishSoap {
        cylinder_piece(commands, meshes, root, &p.soap, at, 0.15, 0.38, true);
        cylinder_piece(
            commands,
            meshes,
            root,
            &p.milk,
            at + Vec3::new(0.0, 0.20, 0.0),
            0.10,
            0.07,
            true,
        );
        cylinder_piece(
            commands,
            meshes,
            root,
            &p.dark_metal,
            at + Vec3::new(0.0, 0.28, 0.0),
            0.035,
            0.12,
            true,
        );
        box_piece(
            commands,
            meshes,
            root,
            &p.dark_metal,
            at + Vec3::new(0.07, 0.33, 0.0),
            Vec3::new(0.18, 0.03, 0.055),
            true,
        );
        box_piece(
            commands,
            meshes,
            root,
            &p.milk,
            at + Vec3::new(0.0, -0.035, 0.149),
            Vec3::new(0.20, 0.15, 0.008),
            true,
        );
    } else if visual == FoodVisual::Apple {
        for i in 0..2 {
            let dx = (i as f32 - 0.5) * 0.30;
            sphere_piece(
                commands,
                meshes,
                root,
                &p.apple,
                at + Vec3::new(dx, -0.05, 0.0),
                0.145,
                Vec3::new(1.0, 0.88, 1.0),
                true,
            );
            cylinder_piece(
                commands,
                meshes,
                root,
                &p.wood_edge,
                at + Vec3::new(dx, 0.09, 0.0),
                0.012,
                0.07,
                true,
            );
            sphere_piece(
                commands,
                meshes,
                root,
                &p.leaf,
                at + Vec3::new(dx + 0.035, 0.13, 0.0),
                0.055,
                Vec3::new(1.2, 0.22, 0.65),
                true,
            );
        }
    } else if visual == FoodVisual::Yogurt {
        for i in 0..2 {
            let dx = (i as f32 - 0.5) * 0.28;
            cylinder_piece(
                commands,
                meshes,
                root,
                &p.yogurt,
                at + Vec3::new(dx, -0.06, 0.0),
                0.12,
                0.25,
                true,
            );
            cylinder_piece(
                commands,
                meshes,
                root,
                &p.blue,
                at + Vec3::new(dx, 0.07, 0.0),
                0.13,
                0.018,
                true,
            );
        }
    } else {
        // An unclassified line does not inherit a fruit or canned-food shape.
        box_piece(
            commands,
            meshes,
            root,
            &p.carton,
            at,
            Vec3::new(0.31, 0.35, 0.23),
            true,
        );
        box_piece(
            commands,
            meshes,
            root,
            &p.milk,
            at + Vec3::new(0.0, 0.0, 0.16),
            Vec3::new(0.23, 0.18, 0.008),
            true,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_shape_requires_a_supported_product_and_packaging_class() {
        assert_eq!(visual_for("APPLES"), FoodVisual::Apple);
        assert_eq!(visual_for("MILK 2%"), FoodVisual::Milk);
        assert_eq!(visual_for("CANNED TOMATOES"), FoodVisual::CannedTomatoes);
        assert_eq!(visual_for("DISH SOAP"), FoodVisual::DishSoap);

        for name in [
            "apple juice",
            "apple sauce",
            "fresh tomatoes",
            "tomato soup",
            "dried beans",
            "almond milk",
            "green tea",
            "hand soap",
        ] {
            assert_eq!(visual_for(name), FoodVisual::Unknown, "{name}");
        }
    }
}
