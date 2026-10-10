use bevy::prelude::*;

use crate::AppState;

#[derive(Component)]
struct Zone(&'static str);

#[derive(Component)]
pub(crate) struct InventoryVisual;

fn zone_clicked(click: On<Pointer<Click>>, zones: Query<&Zone>, mut state: ResMut<AppState>) {
    if let Ok(zone) = zones.get(click.entity) {
        state.zone = zone.0.into();
        state.tab = crate::Tab::Kitchen;
        state.message = format!(
            "Showing {} stock. Choose a lot in the list for actions.",
            zone.0
        );
    }
}

fn box_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    size: Vec3,
    at: Vec3,
    color: Color,
    pickable: bool,
) -> Entity {
    let mut entity = commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
        MeshMaterial3d(materials.add(color)),
        Transform::from_translation(at),
    ));
    if !pickable {
        entity.insert(Pickable::IGNORE);
    }
    entity.id()
}

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(6.0, 5.2, 8.0).looking_at(Vec3::new(0.0, 1.2, 0.0), Vec3::Y),
    ));
    commands.spawn((
        PointLight {
            intensity: 5_000_000.0,
            range: 30.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(1.0, 7.0, 6.0),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.82, 0.88, 1.0),
        brightness: 400.0,
        ..default()
    });

    box_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(14.0, 0.15, 9.0),
        Vec3::new(0.0, -0.1, 0.0),
        Color::srgb(0.28, 0.25, 0.22),
        false,
    );
    box_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(11.0, 4.4, 0.18),
        Vec3::new(0.0, 2.1, -2.6),
        Color::srgb(0.18, 0.22, 0.25),
        false,
    );

    // Each appliance is made of perspective 3D meshes. The front volumes are directly clickable.
    box_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(2.25, 3.6, 1.9),
        Vec3::new(-3.25, 1.8, -0.7),
        Color::srgb(0.55, 0.66, 0.70),
        false,
    );
    let fridge = box_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(2.0, 2.2, 0.16),
        Vec3::new(-3.25, 1.35, 0.32),
        Color::srgb(0.34, 0.58, 0.66),
        true,
    );
    commands
        .entity(fridge)
        .insert(Zone("fridge"))
        .observe(zone_clicked);
    let freezer = box_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(2.0, 0.92, 0.16),
        Vec3::new(-3.25, 2.92, 0.32),
        Color::srgb(0.39, 0.52, 0.65),
        true,
    );
    commands
        .entity(freezer)
        .insert(Zone("freezer"))
        .observe(zone_clicked);
    box_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(0.06, 0.52, 0.09),
        Vec3::new(-2.38, 1.5, 0.47),
        Color::srgb(0.9, 0.9, 0.86),
        false,
    );

    box_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(2.5, 3.5, 1.6),
        Vec3::new(3.0, 1.75, -1.0),
        Color::srgb(0.49, 0.33, 0.21),
        false,
    );
    let pantry = box_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(2.24, 3.2, 0.12),
        Vec3::new(3.0, 1.75, -0.13),
        Color::srgb(0.66, 0.47, 0.28),
        true,
    );
    commands
        .entity(pantry)
        .insert(Zone("pantry"))
        .observe(zone_clicked);
    for y in [0.8, 1.6, 2.4, 3.2] {
        box_mesh(
            &mut commands,
            &mut meshes,
            &mut materials,
            Vec3::new(2.3, 0.08, 1.65),
            Vec3::new(3.0, y, -1.0),
            Color::srgb(0.32, 0.21, 0.14),
            false,
        );
    }
    box_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(2.0, 0.85, 1.3),
        Vec3::new(0.0, 0.42, -1.4),
        Color::srgb(0.42, 0.37, 0.30),
        false,
    );
    box_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(2.4, 0.11, 1.55),
        Vec3::new(0.0, 0.9, -1.4),
        Color::srgb(0.70, 0.67, 0.59),
        false,
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
        state.camera_distance = (state.camera_distance - amount * 3.0).max(5.5);
    }
    if keys.pressed(KeyCode::ArrowDown) {
        state.camera_distance = (state.camera_distance + amount * 3.0).min(18.0);
    }
    let yaw = state.camera_yaw;
    let distance = state.camera_distance;
    camera.translation = Vec3::new(yaw.sin() * distance, distance * 0.48, yaw.cos() * distance);
    camera.look_at(Vec3::new(-1.7, 1.35, -0.6), Vec3::Y);
}

pub fn sync_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing: Query<Entity, With<InventoryVisual>>,
    mut state: ResMut<AppState>,
) {
    if !state.scene_dirty {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    for (zone, base, color) in [
        (
            "fridge",
            Vec3::new(-3.25, 0.48, 0.43),
            Color::srgb(0.38, 0.82, 0.74),
        ),
        (
            "freezer",
            Vec3::new(-3.25, 2.63, 0.43),
            Color::srgb(0.50, 0.75, 0.95),
        ),
        (
            "pantry",
            Vec3::new(3.0, 0.49, 0.0),
            Color::srgb(0.95, 0.73, 0.42),
        ),
    ] {
        let count = state
            .snapshot
            .lots
            .iter()
            .filter(|lot| lot.zone == zone && lot.quantity != "0")
            .count()
            .min(18);
        for index in 0..count {
            let column = index % 4;
            let row = index / 4;
            let at = base + Vec3::new((column as f32 - 1.5) * 0.42, row as f32 * 0.37, 0.0);
            let entity = box_mesh(
                &mut commands,
                &mut meshes,
                &mut materials,
                Vec3::new(0.28, 0.26, 0.24),
                at,
                color,
                false,
            );
            commands.entity(entity).insert(InventoryVisual);
        }
    }
    state.scene_dirty = false;
}
