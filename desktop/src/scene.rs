//! Original, procedural kitchen visuals for the local Rust preview.
//! The receipt ledger and inventory decisions remain in hungerstrike-core.

mod food;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;

use crate::{AppAction, AppState};

const FRIDGE_X: f32 = -1.55;
const PANTRY_X: f32 = 3.05;

#[derive(Component)]
struct ZonePick(&'static str);

#[derive(Component)]
pub(crate) struct ZoneDoor {
    name: &'static str,
    movement: DoorMovement,
    target: f32,
    progress: f32,
    closed: Vec3,
}

#[derive(Clone, Copy)]
enum DoorMovement {
    Swing(f32),
    Slide(Vec3),
}

#[derive(Component)]
pub(crate) struct LotPick(String);

#[derive(Component)]
pub(crate) struct InventoryVisual;

#[derive(Component)]
pub(crate) struct ShellPanel;

#[derive(Resource, Clone)]
pub(crate) struct Palette {
    floor: Handle<StandardMaterial>,
    wall: Handle<StandardMaterial>,
    tile: Handle<StandardMaterial>,
    enamel: Handle<StandardMaterial>,
    liner: Handle<StandardMaterial>,
    enamel_shade: Handle<StandardMaterial>,
    gasket: Handle<StandardMaterial>,
    metal: Handle<StandardMaterial>,
    dark_metal: Handle<StandardMaterial>,
    glass: Handle<StandardMaterial>,
    ghost_shell: Handle<StandardMaterial>,
    plastic: Handle<StandardMaterial>,
    shadow: Handle<StandardMaterial>,
    shadow_soft: Handle<StandardMaterial>,
    wood: Handle<StandardMaterial>,
    wood_edge: Handle<StandardMaterial>,
    counter: Handle<StandardMaterial>,
    focus: Handle<StandardMaterial>,
    banana: Handle<StandardMaterial>,
    leaf: Handle<StandardMaterial>,
    milk: Handle<StandardMaterial>,
    blue: Handle<StandardMaterial>,
    egg: Handle<StandardMaterial>,
    carton: Handle<StandardMaterial>,
    pasta: Handle<StandardMaterial>,
    apple: Handle<StandardMaterial>,
    yogurt: Handle<StandardMaterial>,
    soap: Handle<StandardMaterial>,
}

fn material(
    materials: &mut Assets<StandardMaterial>,
    rgb: (f32, f32, f32),
    metallic: f32,
    roughness: f32,
    alpha: f32,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: Color::srgba(rgb.0, rgb.1, rgb.2, alpha),
        metallic,
        perceptual_roughness: roughness,
        alpha_mode: if alpha < 1.0 {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        },
        ..default()
    })
}

fn palette(materials: &mut Assets<StandardMaterial>) -> Palette {
    Palette {
        floor: material(materials, (0.80, 0.79, 0.75), 0.0, 0.88, 1.0),
        wall: material(materials, (0.87, 0.89, 0.88), 0.0, 0.95, 1.0),
        tile: material(materials, (0.74, 0.81, 0.81), 0.0, 0.62, 1.0),
        enamel: material(materials, (0.86, 0.91, 0.90), 0.10, 0.38, 1.0),
        liner: material(materials, (0.88, 0.93, 0.91), 0.0, 0.55, 1.0),
        enamel_shade: material(materials, (0.63, 0.74, 0.74), 0.08, 0.43, 1.0),
        gasket: material(materials, (0.19, 0.30, 0.31), 0.0, 0.80, 1.0),
        metal: material(materials, (0.62, 0.70, 0.72), 0.78, 0.27, 1.0),
        dark_metal: material(materials, (0.25, 0.34, 0.35), 0.65, 0.42, 1.0),
        glass: material(materials, (0.82, 0.91, 0.89), 0.0, 0.19, 0.22),
        ghost_shell: material(materials, (0.82, 0.91, 0.90), 0.12, 0.36, 0.33),
        plastic: material(materials, (0.88, 0.96, 0.94), 0.0, 0.27, 0.58),
        shadow: material(materials, (0.20, 0.26, 0.27), 0.0, 1.0, 0.16),
        shadow_soft: material(materials, (0.22, 0.28, 0.27), 0.0, 1.0, 0.045),
        wood: material(materials, (0.60, 0.43, 0.29), 0.0, 0.68, 1.0),
        wood_edge: material(materials, (0.39, 0.27, 0.18), 0.0, 0.72, 1.0),
        counter: material(materials, (0.90, 0.87, 0.79), 0.0, 0.31, 1.0),
        focus: material(materials, (0.18, 0.69, 0.64), 0.18, 0.34, 1.0),
        banana: material(materials, (0.96, 0.73, 0.25), 0.0, 0.73, 1.0),
        leaf: material(materials, (0.21, 0.52, 0.30), 0.0, 0.84, 1.0),
        milk: material(materials, (0.94, 0.94, 0.85), 0.0, 0.54, 1.0),
        blue: material(materials, (0.29, 0.55, 0.70), 0.0, 0.45, 1.0),
        egg: material(materials, (0.94, 0.84, 0.67), 0.0, 0.68, 1.0),
        carton: material(materials, (0.66, 0.61, 0.50), 0.0, 0.84, 1.0),
        pasta: material(materials, (0.83, 0.43, 0.23), 0.0, 0.66, 1.0),
        apple: material(materials, (0.74, 0.19, 0.17), 0.0, 0.50, 1.0),
        yogurt: material(materials, (0.86, 0.87, 0.91), 0.0, 0.55, 1.0),
        soap: material(materials, (0.34, 0.72, 0.72), 0.0, 0.31, 1.0),
    }
}

fn shape<S: Into<Mesh>>(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: &Handle<StandardMaterial>,
    primitive: S,
    transform: Transform,
    pickable: bool,
) -> Entity {
    let mut entity = commands.spawn((
        Mesh3d(meshes.add(primitive)),
        MeshMaterial3d(material.clone()),
        transform,
    ));
    if !pickable {
        entity.insert(Pickable::IGNORE);
    }
    entity.id()
}

fn block(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: &Handle<StandardMaterial>,
    size: Vec3,
    at: Vec3,
) -> Entity {
    shape(
        commands,
        meshes,
        material,
        Cuboid::new(size.x, size.y, size.z),
        Transform::from_translation(at),
        false,
    )
}

fn flat_triangle(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    a: Vec3,
    mut b: Vec3,
    mut c: Vec3,
) {
    let mut normal = (b - a).cross(c - a).normalize();
    if normal.dot((a + b + c) / 3.0) < 0.0 {
        std::mem::swap(&mut b, &mut c);
        normal = -normal;
    }
    for vertex in [a, b, c] {
        positions.push(vertex.to_array());
        normals.push(normal.to_array());
    }
}

fn flat_quad(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    d: Vec3,
) {
    flat_triangle(positions, normals, a, b, c);
    flat_triangle(positions, normals, a, c, d);
}

// A closed, flat-chamfered cuboid: six faces, twelve edge strips, eight corners.
// The edge highlight comes from the geometry and its normals, not a texture.
fn chamfered_cuboid(size: Vec3, bevel: f32) -> Mesh {
    let h = size * 0.5;
    let r = bevel.min(h.min_element() * 0.45);
    let e = h - Vec3::splat(r);
    let mut positions = Vec::with_capacity(132);
    let mut normals = Vec::with_capacity(132);
    for s in [-1.0_f32, 1.0] {
        flat_quad(
            &mut positions,
            &mut normals,
            Vec3::new(s * h.x, -e.y, -e.z),
            Vec3::new(s * h.x, e.y, -e.z),
            Vec3::new(s * h.x, e.y, e.z),
            Vec3::new(s * h.x, -e.y, e.z),
        );
        flat_quad(
            &mut positions,
            &mut normals,
            Vec3::new(-e.x, s * h.y, -e.z),
            Vec3::new(e.x, s * h.y, -e.z),
            Vec3::new(e.x, s * h.y, e.z),
            Vec3::new(-e.x, s * h.y, e.z),
        );
        flat_quad(
            &mut positions,
            &mut normals,
            Vec3::new(-e.x, -e.y, s * h.z),
            Vec3::new(e.x, -e.y, s * h.z),
            Vec3::new(e.x, e.y, s * h.z),
            Vec3::new(-e.x, e.y, s * h.z),
        );
    }
    for sx in [-1.0_f32, 1.0] {
        for sy in [-1.0_f32, 1.0] {
            flat_quad(
                &mut positions,
                &mut normals,
                Vec3::new(sx * h.x, sy * e.y, -e.z),
                Vec3::new(sx * e.x, sy * h.y, -e.z),
                Vec3::new(sx * e.x, sy * h.y, e.z),
                Vec3::new(sx * h.x, sy * e.y, e.z),
            );
        }
        for sz in [-1.0_f32, 1.0] {
            flat_quad(
                &mut positions,
                &mut normals,
                Vec3::new(sx * h.x, -e.y, sz * e.z),
                Vec3::new(sx * e.x, -e.y, sz * h.z),
                Vec3::new(sx * e.x, e.y, sz * h.z),
                Vec3::new(sx * h.x, e.y, sz * e.z),
            );
        }
    }
    for sy in [-1.0_f32, 1.0] {
        for sz in [-1.0_f32, 1.0] {
            flat_quad(
                &mut positions,
                &mut normals,
                Vec3::new(-e.x, sy * h.y, sz * e.z),
                Vec3::new(-e.x, sy * e.y, sz * h.z),
                Vec3::new(e.x, sy * e.y, sz * h.z),
                Vec3::new(e.x, sy * h.y, sz * e.z),
            );
        }
    }
    for sx in [-1.0_f32, 1.0] {
        for sy in [-1.0_f32, 1.0] {
            for sz in [-1.0_f32, 1.0] {
                flat_triangle(
                    &mut positions,
                    &mut normals,
                    Vec3::new(sx * h.x, sy * e.y, sz * e.z),
                    Vec3::new(sx * e.x, sy * h.y, sz * e.z),
                    Vec3::new(sx * e.x, sy * e.y, sz * h.z),
                );
            }
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh
}

fn rail(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: &Handle<StandardMaterial>,
    start: Vec3,
    end: Vec3,
    radius: f32,
) -> Entity {
    let length = start.distance(end);
    shape(
        commands,
        meshes,
        material,
        Cylinder::new(radius, length),
        Transform::from_translation((start + end) / 2.0)
            .with_rotation(Quat::from_rotation_arc(Vec3::Y, (end - start).normalize())),
        false,
    )
}

fn zone_clicked(click: On<Pointer<Click>>, zones: Query<&ZonePick>, mut state: ResMut<AppState>) {
    if let Ok(zone) = zones.get(click.entity) {
        if state.dispatch(AppAction::SelectZone(zone.0.into())).is_ok() {
            state.message = format!("{} opened. Select a stocked item to inspect it.", zone.0);
        }
    }
}

pub(crate) fn lot_clicked(
    mut click: On<Pointer<Click>>,
    lots: Query<&LotPick>,
    parents: Query<&ChildOf>,
    mut state: ResMut<AppState>,
) {
    let mut target = click.entity;
    loop {
        if let Ok(lot) = lots.get(target) {
            if state.dispatch(AppAction::SelectLot(lot.0.clone())).is_ok() {
                state.message = "Stock lot selected. Use the detail panel to manage it.".into();
            }
            click.propagate(false);
            return;
        }
        let Ok(parent) = parents.get(target) else {
            return;
        };
        target = parent.parent();
    }
}

fn pick_zone(commands: &mut Commands, entity: Entity, name: &'static str) {
    commands
        .entity(entity)
        .insert(ZonePick(name))
        .observe(zone_clicked);
}

fn child(commands: &mut Commands, entity: Entity, parent: Entity) {
    commands.entity(entity).insert(ChildOf(parent));
}

fn door_root(
    commands: &mut Commands,
    name: &'static str,
    closed: Vec3,
    movement: DoorMovement,
) -> Entity {
    commands
        .spawn((
            Transform::from_translation(closed),
            ZoneDoor {
                name,
                movement,
                target: 0.0,
                progress: 0.0,
                closed,
            },
        ))
        .id()
}

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let p = palette(&mut materials);
    commands.insert_resource(ClearColor(Color::srgb(0.91, 0.93, 0.92)));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(4.2, 4.1, 9.2).looking_at(Vec3::new(0.0, 1.5, -0.6), Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 4_200.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(-4.0, 8.0, 7.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        PointLight {
            intensity: 395_000.0,
            range: 18.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(1.5, 5.5, 4.5),
    ));
    commands.spawn((
        PointLight {
            intensity: 95_000.0,
            range: 4.0,
            color: Color::srgb(1.0, 0.96, 0.90),
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(FRIDGE_X, 2.5, -0.6),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.92, 0.96, 1.0),
        brightness: 82.0,
        ..default()
    });

    block(
        &mut commands,
        &mut meshes,
        &p.floor,
        Vec3::new(11.0, 0.12, 8.0),
        Vec3::new(0.0, -0.08, 0.0),
    );
    for (x, radius) in [(FRIDGE_X, 1.35), (PANTRY_X, 1.08), (0.63, 1.10)] {
        shape(
            &mut commands,
            &mut meshes,
            &p.shadow_soft,
            Cylinder::new(radius * 1.25, 0.006),
            Transform::from_xyz(x, -0.012, -0.75).with_scale(Vec3::new(1.0, 1.0, 0.70)),
            false,
        );
        shape(
            &mut commands,
            &mut meshes,
            &p.shadow,
            Cylinder::new(radius, 0.006),
            Transform::from_xyz(x, -0.009, -0.75).with_scale(Vec3::new(1.0, 1.0, 0.62)),
            false,
        );
    }
    block(
        &mut commands,
        &mut meshes,
        &p.wall,
        Vec3::new(10.0, 4.1, 0.12),
        Vec3::new(0.0, 2.0, -2.34),
    );
    block(
        &mut commands,
        &mut meshes,
        &p.tile,
        Vec3::new(4.2, 1.0, 0.045),
        Vec3::new(0.55, 1.68, -2.26),
    );
    for x in -3..6 {
        block(
            &mut commands,
            &mut meshes,
            &p.liner,
            Vec3::new(0.012, 1.0, 0.05),
            Vec3::new(x as f32 * 0.52 - 1.0, 1.68, -2.22),
        );
    }
    for y in [1.18, 1.68, 2.18] {
        block(
            &mut commands,
            &mut meshes,
            &p.liner,
            Vec3::new(4.2, 0.012, 0.05),
            Vec3::new(0.55, y, -2.22),
        );
    }
    build_fridge(&mut commands, &mut meshes, &p);
    build_pantry(&mut commands, &mut meshes, &p);
    build_counter(&mut commands, &mut meshes, &p);
    commands.insert_resource(p);
}

// 3.14 below is a shelf height in metres, not an approximation of PI.
#[allow(clippy::approx_constant)]
fn build_fridge(commands: &mut Commands, meshes: &mut Assets<Mesh>, p: &Palette) {
    let x = FRIDGE_X;
    // The cabinet is a hollow, thick shell; shelf hardware and food remain visible.
    for (size, at, material) in [
        (
            Vec3::new(0.11, 3.18, 1.68),
            Vec3::new(x - 1.04, 1.60, -0.72),
            &p.enamel,
        ),
        (
            Vec3::new(2.20, 0.14, 1.68),
            Vec3::new(x, 3.13, -0.72),
            &p.enamel,
        ),
        (
            Vec3::new(2.20, 0.14, 1.68),
            Vec3::new(x, 0.07, -0.72),
            &p.enamel,
        ),
        (
            Vec3::new(2.02, 3.00, 0.08),
            Vec3::new(x, 1.60, -1.54),
            &p.liner,
        ),
        (
            Vec3::new(2.05, 0.10, 1.57),
            Vec3::new(x, 2.40, -0.73),
            &p.enamel_shade,
        ),
        (
            Vec3::new(2.10, 0.11, 1.72),
            Vec3::new(x, 0.06, -0.72),
            &p.dark_metal,
        ),
    ] {
        block(commands, meshes, material, size, at);
    }
    shape(
        commands,
        meshes,
        &p.enamel,
        chamfered_cuboid(Vec3::new(2.22, 0.16, 1.72), 0.055),
        Transform::from_xyz(x, 3.16, -0.72),
        false,
    );
    let side = block(
        commands,
        meshes,
        &p.enamel,
        Vec3::new(0.11, 3.18, 1.68),
        Vec3::new(x + 1.04, 1.60, -0.72),
    );
    commands.entity(side).insert(ShellPanel);
    // Narrow round edge rolls catch light without hiding the hollow cabinet.
    for side in [-1.0, 1.0] {
        for z in [-1.53, 0.09] {
            rail(
                commands,
                meshes,
                &p.enamel_shade,
                Vec3::new(x + side * 1.045, 0.10, z),
                Vec3::new(x + side * 1.045, 3.10, z),
                0.047,
            );
        }
        block(
            commands,
            meshes,
            &p.enamel_shade,
            Vec3::new(0.032, 2.91, 0.026),
            Vec3::new(x + side * 0.96, 1.56, -1.48),
        );
    }
    for z in [-1.53, 0.09] {
        rail(
            commands,
            meshes,
            &p.enamel_shade,
            Vec3::new(x - 1.04, 3.14, z),
            Vec3::new(x + 1.04, 3.14, z),
            0.055,
        );
    }
    // Dark door opening perimeter and magnetic gasket seat.
    for side in [-1.0, 1.0] {
        block(
            commands,
            meshes,
            &p.gasket,
            Vec3::new(0.035, 2.24, 0.03),
            Vec3::new(x + side * 0.98, 1.27, 0.125),
        );
        block(
            commands,
            meshes,
            &p.metal,
            Vec3::new(0.035, 2.18, 1.34),
            Vec3::new(x + side * 0.88, 1.28, -0.75),
        );
    }
    for y in [0.15, 2.40] {
        block(
            commands,
            meshes,
            &p.gasket,
            Vec3::new(1.98, 0.035, 0.03),
            Vec3::new(x, y, 0.125),
        );
    }
    for y in [0.91, 1.53, 2.13] {
        block(
            commands,
            meshes,
            &p.glass,
            Vec3::new(1.73, 0.032, 1.34),
            Vec3::new(x, y, -0.72),
        );
        // The darker rear lip separates each glass shelf from the back liner.
        block(
            commands,
            meshes,
            &p.enamel_shade,
            Vec3::new(1.73, 0.045, 0.035),
            Vec3::new(x, y + 0.01, -1.39),
        );
        // Fine supports under the clearer shelf give it depth without
        // presenting decorative geometry as additional stock.
        for offset in [-0.55, 0.0, 0.55] {
            rail(
                commands,
                meshes,
                &p.metal,
                Vec3::new(x + offset, y - 0.038, -1.32),
                Vec3::new(x + offset, y - 0.038, -0.16),
                0.009,
            );
        }
        for side in [-1.0, 1.0] {
            rail(
                commands,
                meshes,
                &p.metal,
                Vec3::new(x + side * 0.85, y, -1.36),
                Vec3::new(x + side * 0.85, y, -0.08),
                0.018,
            );
            for z in [-1.18, -0.64, -0.15] {
                block(
                    commands,
                    meshes,
                    &p.dark_metal,
                    Vec3::new(0.10, 0.022, 0.06),
                    Vec3::new(x + side * 0.89, y - 0.042, z),
                );
            }
        }
        rail(
            commands,
            meshes,
            &p.metal,
            Vec3::new(x - 0.85, y, -0.07),
            Vec3::new(x + 0.85, y, -0.07),
            0.022,
        );
    }
    // Two inset translucent crisper drawers with distinct lips and handles.
    for dx in [-0.45, 0.45] {
        block(
            commands,
            meshes,
            &p.plastic,
            Vec3::new(0.82, 0.45, 1.18),
            Vec3::new(x + dx, 0.43, -0.77),
        );
        block(
            commands,
            meshes,
            &p.enamel_shade,
            Vec3::new(0.82, 0.07, 0.04),
            Vec3::new(x + dx, 0.62, -0.15),
        );
        rail(
            commands,
            meshes,
            &p.metal,
            Vec3::new(x + dx - 0.25, 0.57, -0.11),
            Vec3::new(x + dx + 0.25, 0.57, -0.11),
            0.014,
        );
    }
    // Air vents and freezer shelf repeat the fine mechanical rhythm in the reference.
    for i in 0..12 {
        block(
            commands,
            meshes,
            &p.enamel_shade,
            Vec3::new(0.075, 0.025, 0.02),
            Vec3::new(x - 0.65 + i as f32 * 0.12, 2.34, -1.48),
        );
    }
    block(
        commands,
        meshes,
        &p.glass,
        Vec3::new(1.75, 0.035, 1.30),
        Vec3::new(x, 2.76, -0.74),
    );
    for y in [2.58, 2.98] {
        rail(
            commands,
            meshes,
            &p.metal,
            Vec3::new(x - 0.85, y, -0.12),
            Vec3::new(x + 0.85, y, -0.12),
            0.018,
        );
    }

    let hinge = Vec3::new(x - 1.03, 1.26, 0.20);
    let door = door_root(commands, "fridge", hinge, DoorMovement::Swing(-1.46));
    let panel = shape(
        commands,
        meshes,
        &p.enamel,
        chamfered_cuboid(Vec3::new(2.03, 2.27, 0.20), 0.055),
        Transform::from_xyz(1.015, 0.0, 0.0),
        true,
    );
    child(commands, panel, door);
    pick_zone(commands, panel, "fridge");
    let inner = block(
        commands,
        meshes,
        &p.liner,
        Vec3::new(1.78, 2.02, 0.025),
        Vec3::new(1.015, 0.0, -0.118),
    );
    child(commands, inner, door);
    for side in [0.04, 1.99] {
        let edge = rail(
            commands,
            meshes,
            &p.enamel_shade,
            Vec3::new(side, -1.09, 0.10),
            Vec3::new(side, 1.09, 0.10),
            0.046,
        );
        child(commands, edge, door);
    }
    for y in [-1.13, 1.13] {
        let edge = rail(
            commands,
            meshes,
            &p.enamel_shade,
            Vec3::new(0.06, y, 0.10),
            Vec3::new(1.97, y, 0.10),
            0.046,
        );
        child(commands, edge, door);
    }
    for side in [0.07, 1.96] {
        let trim = block(
            commands,
            meshes,
            &p.gasket,
            Vec3::new(0.035, 2.12, 0.025),
            Vec3::new(side, 0.0, -0.072),
        );
        child(commands, trim, door);
    }
    for y in [-1.02, 1.02] {
        let trim = block(
            commands,
            meshes,
            &p.gasket,
            Vec3::new(1.86, 0.035, 0.025),
            Vec3::new(1.015, y, -0.072),
        );
        child(commands, trim, door);
        let hinge_pin = shape(
            commands,
            meshes,
            &p.dark_metal,
            Cylinder::new(0.055, 0.12),
            Transform::from_xyz(0.0, y, 0.02),
            false,
        );
        child(commands, hinge_pin, door);
    }
    let handle = block(
        commands,
        meshes,
        &p.metal,
        Vec3::new(0.07, 0.78, 0.065),
        Vec3::new(1.78, 0.12, 0.14),
    );
    child(commands, handle, door);
    for y in [-0.66, 0.0, 0.66] {
        let bin = block(
            commands,
            meshes,
            &p.plastic,
            Vec3::new(1.42, 0.04, 0.26),
            Vec3::new(1.0, y - 0.08, -0.22),
        );
        child(commands, bin, door);
        let front = block(
            commands,
            meshes,
            &p.plastic,
            Vec3::new(1.42, 0.19, 0.018),
            Vec3::new(1.0, y + 0.03, -0.35),
        );
        child(commands, front, door);
        for z in [-0.34, -0.10] {
            let bar = rail(
                commands,
                meshes,
                &p.metal,
                Vec3::new(0.30, y + 0.12, z),
                Vec3::new(1.70, y + 0.12, z),
                0.014,
            );
            child(commands, bar, door);
        }
        for x in [0.31, 1.69] {
            let upright = rail(
                commands,
                meshes,
                &p.metal,
                Vec3::new(x, y - 0.07, -0.34),
                Vec3::new(x, y + 0.12, -0.34),
                0.013,
            );
            child(commands, upright, door);
        }
    }

    let freezer = door_root(
        commands,
        "freezer",
        Vec3::new(x - 1.02, 2.77, 0.18),
        DoorMovement::Slide(Vec3::new(0.0, 0.0, 0.68)),
    );
    let front = shape(
        commands,
        meshes,
        &p.enamel_shade,
        chamfered_cuboid(Vec3::new(2.03, 0.71, 0.19), 0.05),
        Transform::from_xyz(1.02, 0.0, 0.0),
        true,
    );
    child(commands, front, freezer);
    pick_zone(commands, front, "freezer");
    for y in [-0.34, 0.34] {
        let edge = rail(
            commands,
            meshes,
            &p.enamel,
            Vec3::new(0.05, y, 0.10),
            Vec3::new(1.99, y, 0.10),
            0.038,
        );
        child(commands, edge, freezer);
    }
    for y in [-0.27, 0.27] {
        let seam = block(
            commands,
            meshes,
            &p.dark_metal,
            Vec3::new(1.87, 0.018, 0.015),
            Vec3::new(1.02, y, 0.079),
        );
        child(commands, seam, freezer);
    }
    let pull = block(
        commands,
        meshes,
        &p.metal,
        Vec3::new(1.32, 0.055, 0.065),
        Vec3::new(1.02, -0.18, 0.13),
    );
    child(commands, pull, freezer);
    let tray = block(
        commands,
        meshes,
        &p.plastic,
        Vec3::new(1.75, 0.12, 1.27),
        Vec3::new(1.02, -0.21, -0.67),
    );
    child(commands, tray, freezer);
    for side in [0.16, 1.88] {
        let runner = block(
            commands,
            meshes,
            &p.metal,
            Vec3::new(0.04, 0.05, 1.24),
            Vec3::new(side, -0.16, -0.66),
        );
        child(commands, runner, freezer);
    }
}

fn build_pantry(commands: &mut Commands, meshes: &mut Assets<Mesh>, p: &Palette) {
    let x = PANTRY_X;
    for (size, at, material) in [
        (
            Vec3::new(0.10, 2.76, 1.42),
            Vec3::new(x - 0.91, 1.43, -0.75),
            &p.wood,
        ),
        (
            Vec3::new(0.10, 2.76, 1.42),
            Vec3::new(x + 0.91, 1.43, -0.75),
            &p.wood,
        ),
        (
            Vec3::new(1.92, 0.11, 1.42),
            Vec3::new(x, 2.78, -0.75),
            &p.wood,
        ),
        (
            Vec3::new(1.92, 0.11, 1.42),
            Vec3::new(x, 0.09, -0.75),
            &p.wood,
        ),
        (
            Vec3::new(1.78, 2.62, 0.06),
            Vec3::new(x, 1.42, -1.45),
            &p.wood_edge,
        ),
        (
            Vec3::new(1.93, 0.12, 1.45),
            Vec3::new(x, 0.08, -0.74),
            &p.wood_edge,
        ),
    ] {
        block(commands, meshes, material, size, at);
    }
    for y in [0.72, 1.28, 1.84, 2.40] {
        block(
            commands,
            meshes,
            &p.wood,
            Vec3::new(1.72, 0.065, 1.31),
            Vec3::new(x, y, -0.76),
        );
        block(
            commands,
            meshes,
            &p.wood_edge,
            Vec3::new(1.72, 0.035, 0.035),
            Vec3::new(x, y + 0.02, -0.09),
        );
        for side in [-0.82, 0.82] {
            block(
                commands,
                meshes,
                &p.metal,
                Vec3::new(0.025, 0.035, 1.06),
                Vec3::new(x + side, y - 0.04, -0.72),
            );
        }
    }
    let hinge = Vec3::new(x - 0.90, 1.43, 0.02);
    let door = door_root(commands, "pantry", hinge, DoorMovement::Swing(-1.35));
    let panel = shape(
        commands,
        meshes,
        &p.wood,
        chamfered_cuboid(Vec3::new(1.78, 2.66, 0.085), 0.025),
        Transform::from_xyz(0.89, 0.0, 0.0),
        true,
    );
    child(commands, panel, door);
    pick_zone(commands, panel, "pantry");
    for y in [-1.12, 1.12] {
        let hinge_pin = shape(
            commands,
            meshes,
            &p.dark_metal,
            Cylinder::new(0.045, 0.12),
            Transform::from_xyz(0.0, y, 0.0),
            false,
        );
        child(commands, hinge_pin, door);
    }
    for y in [-0.88, 0.88] {
        let stile = block(
            commands,
            meshes,
            &p.wood_edge,
            Vec3::new(1.55, 0.035, 0.018),
            Vec3::new(0.89, y, 0.054),
        );
        child(commands, stile, door);
    }
    let handle = block(
        commands,
        meshes,
        &p.metal,
        Vec3::new(0.055, 0.54, 0.075),
        Vec3::new(1.51, 0.0, 0.14),
    );
    child(commands, handle, door);
}

fn build_counter(commands: &mut Commands, meshes: &mut Assets<Mesh>, p: &Palette) {
    for (size, at, material) in [
        (
            Vec3::new(1.62, 0.88, 1.45),
            Vec3::new(0.63, 0.48, -1.02),
            &p.wood,
        ),
        (
            Vec3::new(1.82, 0.13, 1.64),
            Vec3::new(0.63, 0.99, -1.02),
            &p.counter,
        ),
        (
            Vec3::new(1.82, 0.035, 1.64),
            Vec3::new(0.63, 0.91, -1.02),
            &p.wood_edge,
        ),
        (
            Vec3::new(1.62, 0.11, 0.12),
            Vec3::new(0.63, 0.10, -0.25),
            &p.wood_edge,
        ),
        (
            Vec3::new(0.035, 0.70, 0.04),
            Vec3::new(0.63, 0.51, -0.26),
            &p.wood_edge,
        ),
    ] {
        block(commands, meshes, material, size, at);
    }
    for dx in [-0.38, 0.38] {
        block(
            commands,
            meshes,
            &p.wood,
            Vec3::new(0.70, 0.66, 0.055),
            Vec3::new(0.63 + dx, 0.51, -0.23),
        );
        rail(
            commands,
            meshes,
            &p.metal,
            Vec3::new(0.45 + dx, 0.57, -0.18),
            Vec3::new(0.80 + dx, 0.57, -0.18),
            0.013,
        );
    }
    // A simple inset sink provides context without competing with the fridge.
    block(
        commands,
        meshes,
        &p.dark_metal,
        Vec3::new(0.65, 0.01, 0.44),
        Vec3::new(0.63, 1.06, -1.11),
    );
    block(
        commands,
        meshes,
        &p.metal,
        Vec3::new(0.55, 0.02, 0.35),
        Vec3::new(0.63, 1.067, -1.11),
    );
    rail(
        commands,
        meshes,
        &p.metal,
        Vec3::new(0.63, 1.07, -1.37),
        Vec3::new(0.63, 1.28, -1.37),
        0.017,
    );
    rail(
        commands,
        meshes,
        &p.metal,
        Vec3::new(0.63, 1.28, -1.37),
        Vec3::new(0.63, 1.28, -1.15),
        0.017,
    );
}

pub fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut state: ResMut<AppState>,
    mut camera: Single<&mut Transform, With<Camera3d>>,
) {
    let amount = time.delta_secs() * 1.5;
    if keys.pressed(KeyCode::ArrowLeft) {
        state.camera_yaw -= amount;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        state.camera_yaw += amount;
    }
    if keys.pressed(KeyCode::ArrowUp) {
        state.camera_distance = (state.camera_distance - amount * 3.0).max(5.8);
    }
    if keys.pressed(KeyCode::ArrowDown) {
        state.camera_distance = (state.camera_distance + amount * 3.0).min(15.0);
    }
    let focus = match state.zone.as_str() {
        // Leave room for the open door while keeping the far pantry outside
        // the fridge composition at both supported capture widths.
        "fridge" => Vec3::new(FRIDGE_X - 0.18, 1.57, -0.66),
        "freezer" => Vec3::new(FRIDGE_X, 2.64, -0.66),
        "pantry" => Vec3::new(PANTRY_X, 1.48, -0.65),
        _ => Vec3::new(0.0, 1.48, -0.65),
    };
    let distance = state.camera_distance
        * match state.zone.as_str() {
            "fridge" => 0.87,
            "freezer" => 0.92,
            "pantry" => 0.94,
            _ => 0.94,
        };
    let yaw = state.camera_yaw;
    let desired = focus + Vec3::new(yaw.sin() * distance, distance * 0.39, yaw.cos() * distance);
    let easing = 1.0 - (-time.delta_secs() * 3.5).exp();
    camera.translation = camera.translation.lerp(desired, easing);
    let wanted = (focus - camera.translation).normalize_or_zero();
    if wanted.length_squared() > 0.0 {
        let target_rotation = Transform::from_translation(camera.translation)
            .looking_at(focus, Vec3::Y)
            .rotation;
        camera.rotation = camera.rotation.slerp(target_rotation, easing.max(0.02));
    }
}

#[allow(clippy::too_many_arguments)] // Bevy system parameters
pub fn sync_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Option<Res<AssetServer>>,
    p: Res<Palette>,
    existing: Query<Entity, With<InventoryVisual>>,
    mut doors: Query<&mut ZoneDoor>,
    mut shell: Query<&mut MeshMaterial3d<StandardMaterial>, With<ShellPanel>>,
    mut state: ResMut<AppState>,
) {
    if !state.scene_dirty {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    for mut door in &mut doors {
        door.target = if state.zone == door.name || (state.zone == "all" && door.name == "fridge") {
            1.0
        } else {
            0.0
        };
    }
    for mut material in &mut shell {
        material.0 = if state.zone == "fridge" || state.zone == "all" {
            p.ghost_shell.clone()
        } else {
            p.enamel.clone()
        };
    }
    for zone in ["fridge", "freezer", "pantry"] {
        if state.zone != "all" && state.zone != zone {
            continue;
        }
        let lots: Vec<_> = state
            .snapshot
            .lots
            .iter()
            .filter(|lot| lot.zone == zone && lot.quantity != "0")
            .take(18)
            .collect();
        for (index, lot) in lots.iter().enumerate() {
            let at = match zone {
                "fridge" => Vec3::new(
                    FRIDGE_X + (index % 3) as f32 * 0.62 - 0.62,
                    1.13 + (index / 3) as f32 * 0.61,
                    -0.46,
                ),
                "freezer" => Vec3::new(FRIDGE_X - 0.55 + (index % 3) as f32 * 0.55, 2.79, -0.72),
                _ => Vec3::new(
                    PANTRY_X - 0.40 + (index % 2) as f32 * 0.80,
                    0.92 + (index / 2) as f32 * 0.56,
                    -0.75,
                ),
            };
            food::spawn_lot(
                &mut commands,
                &mut meshes,
                asset_server.as_deref(),
                &p,
                &lot.id,
                &lot.name,
                at,
                state.lot_id == lot.id,
            );
        }
    }
    state.scene_dirty = false;
}

pub fn animate_visuals(time: Res<Time>, mut doors: Query<(&mut ZoneDoor, &mut Transform)>) {
    let easing = 1.0 - (-time.delta_secs() * 5.5).exp();
    for (mut door, mut transform) in &mut doors {
        door.progress += (door.target - door.progress) * easing;
        match door.movement {
            DoorMovement::Swing(angle) => {
                transform.translation = door.closed;
                transform.rotation = Quat::from_rotation_y(angle * door.progress);
            }
            DoorMovement::Slide(distance) => {
                transform.translation = door.closed + distance * door.progress;
            }
        }
    }
}

/// Light (original) or dark room settings for the virtual kitchen.
struct RoomLook {
    clear: (f32, f32, f32),
    floor: (f32, f32, f32),
    wall: (f32, f32, f32),
    tile: (f32, f32, f32),
    liner: (f32, f32, f32),
    counter: (f32, f32, f32),
    ambient: (f32, f32, f32),
    ambient_brightness: f32,
    key_illuminance: f32,
}

const LIGHT_ROOM: RoomLook = RoomLook {
    clear: (0.91, 0.93, 0.92),
    floor: (0.80, 0.79, 0.75),
    wall: (0.87, 0.89, 0.88),
    tile: (0.74, 0.81, 0.81),
    liner: (0.88, 0.93, 0.91),
    counter: (0.90, 0.87, 0.79),
    ambient: (0.92, 0.96, 1.0),
    ambient_brightness: 82.0,
    key_illuminance: 4_200.0,
};

/// An evening kitchen: darker room surfaces so the light appliances and the
/// warm fridge interior stand out against the dark app panels.
const DARK_ROOM: RoomLook = RoomLook {
    clear: (0.075, 0.085, 0.092),
    floor: (0.30, 0.29, 0.27),
    wall: (0.20, 0.235, 0.245),
    tile: (0.19, 0.29, 0.30),
    liner: (0.28, 0.35, 0.35),
    counter: (0.62, 0.58, 0.50),
    ambient: (0.70, 0.80, 0.95),
    ambient_brightness: 55.0,
    key_illuminance: 3_400.0,
};

fn room_look(dark: bool) -> &'static RoomLook {
    if dark {
        &DARK_ROOM
    } else {
        &LIGHT_ROOM
    }
}

/// Re-colour the room, background and lighting when the theme changes.
pub fn apply_scene_theme(
    state: Res<AppState>,
    palette: Option<Res<Palette>>,
    mut applied: Local<Option<bool>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut clear: ResMut<ClearColor>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut key_lights: Query<&mut DirectionalLight>,
) {
    let Some(palette) = palette else {
        return;
    };
    if *applied == Some(state.dark) {
        return;
    }
    let look = room_look(state.dark);
    clear.0 = Color::srgb(look.clear.0, look.clear.1, look.clear.2);
    ambient.color = Color::srgb(look.ambient.0, look.ambient.1, look.ambient.2);
    ambient.brightness = look.ambient_brightness;
    for mut light in &mut key_lights {
        light.illuminance = look.key_illuminance;
    }
    for (handle, rgb) in [
        (&palette.floor, look.floor),
        (&palette.wall, look.wall),
        (&palette.tile, look.tile),
        (&palette.liner, look.liner),
        (&palette.counter, look.counter),
    ] {
        if let Some(material) = materials.get_mut(handle) {
            material.base_color = Color::srgb(rgb.0, rgb.1, rgb.2);
        }
    }
    *applied = Some(state.dark);
}

#[cfg(test)]
mod theme_tests {
    use super::*;

    #[test]
    fn light_room_matches_the_original_scene_values() {
        let look = room_look(false);
        assert_eq!(look.clear, (0.91, 0.93, 0.92));
        assert_eq!(look.floor, (0.80, 0.79, 0.75));
        assert_eq!(look.ambient_brightness, 82.0);
        assert_eq!(look.key_illuminance, 4_200.0);
    }

    #[test]
    fn dark_room_is_darker_than_the_appliances() {
        let look = room_look(true);
        let brightness = |(r, g, b): (f32, f32, f32)| (r + g + b) / 3.0;
        // Appliance enamel is ~0.89; the room must stay well below it.
        assert!(brightness(look.clear) < 0.15);
        assert!(brightness(look.wall) < 0.89 / 2.0);
        assert!(brightness(look.floor) < 0.89 / 2.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::TaskPoolPlugin;
    use bevy::asset::{AssetApp, AssetPlugin};
    use bevy::camera::primitives::MeshAabb;
    use bevy::camera::NormalizedRenderTarget;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::gltf::{Gltf, GltfMesh, GltfPlugin};
    use bevy::mesh::MeshPlugin;
    use bevy::picking::backend::HitData;
    use bevy::picking::mesh_picking::ray_cast::{
        MeshRayCast, MeshRayCastSettings, RayCastBackfaces, RayCastVisibility,
    };
    use bevy::picking::pointer::{Location, PointerButton, PointerId};
    use bevy::scene::ScenePlugin;
    use hungerstrike_core::{confirm_receipt, parse_text, review, Store};
    use std::path::PathBuf;
    use std::time::Duration;

    fn synthetic_click(world: &mut World, target: Entity) {
        world.trigger(Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::None {
                    width: 1,
                    height: 1,
                },
                position: Vec2::ZERO,
            },
            Click {
                button: PointerButton::Primary,
                hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                duration: Duration::ZERO,
            },
            target,
        ));
    }

    fn spawn_picking_fixture(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        p: Res<Palette>,
        state: Res<crate::AppState>,
    ) {
        for (index, name) in ["banana", "apple", "yogurt"].into_iter().enumerate() {
            food::spawn_lot(
                &mut commands,
                &mut meshes,
                None,
                &p,
                &state.snapshot.lots[index].id,
                name,
                Vec3::new(index as f32, 1.0, 0.0),
                false,
            );
        }
    }

    #[test]
    fn every_visible_food_piece_and_nested_mesh_selects_its_stock_lot() {
        let mut store = Store::open_in_memory().unwrap();
        let raw = include_str!("../../core/tests/fixtures/receipts/R001-purchase.txt");
        confirm_receipt(&mut store, review(parse_text(raw)).unwrap()).unwrap();
        let mut app = App::new();
        app.insert_resource(crate::AppState::new(store, "2026-10-04".into()).unwrap());
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        let palette = palette(&mut app.world_mut().resource_mut::<Assets<StandardMaterial>>());
        app.insert_resource(palette);
        app.add_observer(lot_clicked);
        app.add_systems(Startup, spawn_picking_fixture);
        app.update();

        let world = app.world_mut();
        let mut roots = world.query::<(Entity, &LotPick, &Children)>();
        let groups: Vec<_> = roots
            .iter(world)
            .map(|(root, lot, children)| (root, lot.0.clone(), children.iter().collect::<Vec<_>>()))
            .collect();
        assert_eq!(groups.len(), 3);
        for (index, (root, id, children)) in groups.into_iter().enumerate() {
            synthetic_click(world, root);
            assert_eq!(world.resource::<crate::AppState>().lot_id, id, "root");
            world.resource_mut::<crate::AppState>().lot_id.clear();
            let visible: Vec<_> = children
                .into_iter()
                .filter(|&child| {
                    world.get::<Mesh3d>(child).is_some()
                        && world.get::<Pickable>(child) != Some(&Pickable::IGNORE)
                })
                .collect();
            assert_eq!(visible.len(), [22, 6, 4][index]);
            for target in visible.iter().copied() {
                synthetic_click(world, target);
                assert_eq!(world.resource::<crate::AppState>().lot_id, id);
                world.resource_mut::<crate::AppState>().lot_id.clear();
            }

            // Imported glTFs can have grouping nodes between the lot and a mesh.
            let group = world.spawn((Transform::IDENTITY, ChildOf(root))).id();
            let nested_mesh = world
                .spawn((
                    Mesh3d(Handle::<Mesh>::default()),
                    Transform::IDENTITY,
                    ChildOf(group),
                ))
                .id();
            synthetic_click(world, nested_mesh);
            assert_eq!(world.resource::<crate::AppState>().lot_id, id);
            world.resource_mut::<crate::AppState>().lot_id.clear();
        }
    }

    #[test]
    fn imported_generic_milk_mesh_ray_hits_a_selectable_lot() {
        // This exercises Bevy's real mesh raycast against bundled glTF geometry,
        // then sends a synthetic click to the existing ancestry observer. It
        // does not exercise an OS pointer or a rendered camera.
        let asset_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets");
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin {
                file_path: asset_root.to_string_lossy().into_owned(),
                ..default()
            },
            ScenePlugin,
            MeshPlugin,
            GltfPlugin::default(),
        ));
        app.init_asset::<StandardMaterial>();
        app.finish();
        app.cleanup();
        let mut store = Store::open_in_memory().unwrap();
        let raw = include_str!("../../core/tests/fixtures/receipts/R001-purchase.txt");
        confirm_receipt(&mut store, review(parse_text(raw)).unwrap()).unwrap();
        let state = crate::AppState::new(store, "2026-10-04".into()).unwrap();
        let milk_id = state
            .snapshot
            .lots
            .iter()
            .find(|lot| lot.name == "milk 2%")
            .expect("confirmed milk lot")
            .id
            .clone();
        app.insert_resource(state);
        app.add_observer(lot_clicked);

        let handle: Handle<Gltf> = app
            .world()
            .resource::<AssetServer>()
            .load("models/polyhaven/long_life_food/generic_milk.gltf");
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while app
            .world()
            .resource::<Assets<Gltf>>()
            .get(&handle)
            .is_none()
            && std::time::Instant::now() < deadline
        {
            app.update();
            std::thread::sleep(Duration::from_millis(2));
        }
        let mesh_handle = {
            let world = app.world();
            let gltf_assets = world.resource::<Assets<Gltf>>();
            let gltf = gltf_assets.get(&handle).expect("generic milk glTF loads");
            let gltf_meshes = world.resource::<Assets<GltfMesh>>();
            let gltf_mesh = gltf_meshes.get(&gltf.meshes[0]).expect("gltf mesh loads");
            gltf_mesh.primitives[0].mesh.clone()
        };
        let aabb = app
            .world()
            .resource::<Assets<Mesh>>()
            .get(&mesh_handle)
            .expect("primitive mesh loads")
            .compute_aabb()
            .expect("imported mesh has bounds");
        let center = Vec3::from(aabb.center);
        let extent = Vec3::from(aabb.half_extents);
        let root = app
            .world_mut()
            .spawn((
                LotPick(milk_id.clone()),
                Transform::IDENTITY,
                Visibility::Visible,
            ))
            .id();
        let group = app
            .world_mut()
            .spawn((Transform::IDENTITY, ChildOf(root)))
            .id();
        let mesh_entity = app
            .world_mut()
            .spawn((
                Mesh3d(mesh_handle),
                aabb,
                GlobalTransform::IDENTITY,
                InheritedVisibility::VISIBLE,
                ViewVisibility::default(),
                RayCastBackfaces,
                ChildOf(group),
            ))
            .id();
        let ray = Ray3d::new(center + Vec3::Z * (extent.z + 1.0), Dir3::NEG_Z);
        let hits: Vec<Entity> = app
            .world_mut()
            .run_system_once(move |mut ray_cast: MeshRayCast| {
                ray_cast
                    .cast_ray(
                        ray,
                        &MeshRayCastSettings::default().with_visibility(RayCastVisibility::Any),
                    )
                    .iter()
                    .map(|(entity, _)| *entity)
                    .collect::<Vec<_>>()
            })
            .expect("raycast system runs");
        assert!(hits.contains(&mesh_entity), "ray hits imported geometry");
        synthetic_click(app.world_mut(), mesh_entity);
        assert_eq!(app.world().resource::<crate::AppState>().lot_id, milk_id);
    }

    #[test]
    fn selected_zone_targets_door_and_renders_only_confirmed_lots() {
        let mut store = Store::open_in_memory().unwrap();
        let raw = include_str!("../../core/tests/fixtures/receipts/R001-purchase.txt");
        confirm_receipt(&mut store, review(parse_text(raw)).unwrap()).unwrap();
        let mut app = App::new();
        app.insert_resource(crate::AppState::new(store, "2026-10-04".into()).unwrap());
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.add_systems(Startup, setup);
        app.add_systems(Update, sync_visuals);
        app.update();
        app.world_mut()
            .resource_mut::<crate::AppState>()
            .dispatch(AppAction::SelectZone("fridge".into()))
            .unwrap();
        app.update();

        let world = app.world_mut();
        let mut doors = world.query::<&ZoneDoor>();
        let targets: Vec<_> = doors
            .iter(world)
            .map(|door| (door.name, door.target))
            .collect();
        assert!(targets.contains(&("fridge", 1.0)));
        assert!(targets.contains(&("freezer", 0.0)));
        assert!(targets.contains(&("pantry", 0.0)));
        let mut lots = world.query::<&LotPick>();
        let shown: Vec<_> = lots.iter(world).map(|lot| lot.0.clone()).collect();
        let snapshot = &world.resource::<crate::AppState>().snapshot;
        let expected: Vec<_> = snapshot
            .lots
            .iter()
            .filter(|lot| lot.zone == "fridge" && lot.quantity != "0")
            .map(|lot| lot.id.clone())
            .collect();
        assert_eq!(shown.len(), expected.len());
        assert!(shown.iter().all(|id| expected.contains(id)));
    }
}
