use bevy::prelude::*;

use crate::{AppAction, AppState};

#[derive(Component)]
pub(crate) struct Zone {
    name: &'static str,
    closed: Vec3,
    open_offset: Vec3,
}

#[derive(Component)]
struct LotPick(String);

#[derive(Component)]
pub(crate) struct InventoryVisual;

fn zone_clicked(click: On<Pointer<Click>>, zones: Query<&Zone>, mut state: ResMut<AppState>) {
    if let Ok(zone) = zones.get(click.entity) {
        if state
            .dispatch(AppAction::SelectZone(zone.name.into()))
            .is_ok()
        {
            state.message = format!(
                "{} opened. Its confirmed stock is in the list and on the shelf.",
                zone.name
            );
        }
    }
}

fn lot_clicked(click: On<Pointer<Click>>, lots: Query<&LotPick>, mut state: ResMut<AppState>) {
    if let Ok(lot) = lots.get(click.entity) {
        if state.dispatch(AppAction::SelectLot(lot.0.clone())).is_ok() {
            state.message =
                "Stock lot selected. Choose Consume, Discard or Move in the list.".into();
        }
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
        #[cfg(target_os = "android")]
        Msaa::Off,
    ));
    commands.spawn((
        PointLight {
            intensity: 5_000_000.0,
            range: 30.0,
            shadows_enabled: !cfg!(target_os = "android"),
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
    // A hollow appliance lets selected stock sit on visible shelves behind its doors.
    for (size, at, color) in [
        (
            Vec3::new(0.14, 3.6, 1.9),
            Vec3::new(-4.31, 1.8, -0.7),
            Color::srgb(0.55, 0.66, 0.70),
        ),
        (
            Vec3::new(0.14, 3.6, 1.9),
            Vec3::new(-2.19, 1.8, -0.7),
            Color::srgb(0.55, 0.66, 0.70),
        ),
        (
            Vec3::new(2.25, 3.6, 0.12),
            Vec3::new(-3.25, 1.8, -1.59),
            Color::srgb(0.27, 0.38, 0.42),
        ),
        (
            Vec3::new(2.25, 0.12, 1.9),
            Vec3::new(-3.25, 3.55, -0.7),
            Color::srgb(0.60, 0.72, 0.76),
        ),
        (
            Vec3::new(2.25, 0.12, 1.9),
            Vec3::new(-3.25, 0.08, -0.7),
            Color::srgb(0.60, 0.72, 0.76),
        ),
        (
            Vec3::new(2.05, 0.07, 1.55),
            Vec3::new(-3.25, 1.12, -0.72),
            Color::srgb(0.64, 0.80, 0.80),
        ),
        (
            Vec3::new(2.05, 0.07, 1.55),
            Vec3::new(-3.25, 1.88, -0.72),
            Color::srgb(0.64, 0.80, 0.80),
        ),
    ] {
        box_mesh(
            &mut commands,
            &mut meshes,
            &mut materials,
            size,
            at,
            color,
            false,
        );
    }
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
        .insert(Zone {
            name: "fridge",
            closed: Vec3::new(-3.25, 1.35, 0.32),
            open_offset: Vec3::new(-0.95, 0.0, 0.55),
        })
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
        .insert(Zone {
            name: "freezer",
            closed: Vec3::new(-3.25, 2.92, 0.32),
            open_offset: Vec3::new(0.0, 0.0, 0.65),
        })
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
        .insert(Zone {
            name: "pantry",
            closed: Vec3::new(3.0, 1.75, -0.13),
            open_offset: Vec3::new(1.08, 0.0, 0.55),
        })
        .observe(zone_clicked);
    box_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(0.06, 0.64, 0.10),
        Vec3::new(3.98, 1.7, 0.0),
        Color::srgb(0.94, 0.81, 0.56),
        false,
    );
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
    // A backsplash, counter trim and plinths give the fixed room clear physical zones.
    for (size, at, color) in [
        (
            Vec3::new(4.3, 0.9, 0.08),
            Vec3::new(0.0, 1.65, -2.47),
            Color::srgb(0.30, 0.38, 0.40),
        ),
        (
            Vec3::new(2.55, 0.16, 1.85),
            Vec3::new(3.0, 0.08, -1.0),
            Color::srgb(0.26, 0.18, 0.13),
        ),
        (
            Vec3::new(2.30, 0.14, 2.0),
            Vec3::new(-3.25, 0.08, -0.7),
            Color::srgb(0.31, 0.39, 0.44),
        ),
        (
            Vec3::new(2.48, 0.12, 1.7),
            Vec3::new(0.0, 0.98, -1.4),
            Color::srgb(0.78, 0.73, 0.65),
        ),
    ] {
        box_mesh(
            &mut commands,
            &mut meshes,
            &mut materials,
            size,
            at,
            color,
            false,
        );
    }
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
    mut zones: Query<(&Zone, &mut Transform, &MeshMaterial3d<StandardMaterial>)>,
    mut state: ResMut<AppState>,
) {
    if !state.scene_dirty {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    for (zone, mut transform, material) in &mut zones {
        let selected = state.zone == zone.name;
        transform.translation = zone.closed
            + if selected {
                zone.open_offset
            } else {
                Vec3::ZERO
            };
        if let Some(material) = materials.get_mut(&material.0) {
            material.base_color = match (zone.name, selected) {
                ("fridge", true) => Color::srgb(0.49, 0.86, 0.83),
                ("freezer", true) => Color::srgb(0.66, 0.82, 1.0),
                ("pantry", true) => Color::srgb(0.94, 0.70, 0.38),
                ("fridge", false) => Color::srgb(0.34, 0.58, 0.66),
                ("freezer", false) => Color::srgb(0.39, 0.52, 0.65),
                _ => Color::srgb(0.66, 0.47, 0.28),
            };
        }
    }
    for (zone, base, color) in [
        (
            "fridge",
            Vec3::new(-3.25, 0.58, 0.16),
            Color::srgb(0.38, 0.82, 0.74),
        ),
        (
            "freezer",
            Vec3::new(-3.25, 2.72, 0.16),
            Color::srgb(0.50, 0.75, 0.95),
        ),
        (
            "pantry",
            Vec3::new(3.0, 0.49, 0.0),
            Color::srgb(0.95, 0.73, 0.42),
        ),
    ] {
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
            let columns = if zone == "fridge" { 2 } else { 3 };
            let column = index % columns;
            let row = index / columns;
            let spacing = if zone == "fridge" { 0.74 } else { 0.55 };
            let at = base
                + Vec3::new(
                    (column as f32 - (columns as f32 - 1.0) / 2.0) * 0.57,
                    row as f32 * spacing,
                    0.0,
                );
            let entity = box_mesh(
                &mut commands,
                &mut meshes,
                &mut materials,
                Vec3::new(0.28, 0.26, 0.24),
                at,
                if state.lot_id == lot.id {
                    Color::srgb(1.0, 0.90, 0.38)
                } else {
                    color
                },
                true,
            );
            commands
                .entity(entity)
                .insert((InventoryVisual, LotPick(lot.id.clone())))
                .observe(lot_clicked);
        }
    }
    state.scene_dirty = false;
}

#[cfg(test)]
mod tests {
    use super::*;
    use hungerstrike_core::{confirm_receipt, parse_text, review, Store};

    #[test]
    fn selected_zone_opens_and_renders_only_its_confirmed_lots() {
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
        let mut zones = world.query::<(&Zone, &Transform)>();
        let mut opened = 0;
        for (zone, transform) in zones.iter(world) {
            if zone.name == "fridge" {
                assert_eq!(transform.translation, zone.closed + zone.open_offset);
                opened += 1;
            } else {
                assert_eq!(transform.translation, zone.closed);
            }
        }
        assert_eq!(opened, 1);
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
