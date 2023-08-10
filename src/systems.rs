use bevy::prelude::{Added, Camera2dBundle, Commands, default, Entity, EventReader, OrthographicProjection, Query, Res, ResMut, SpriteBundle, Transform, With, Without};
use bevy_xpbd_2d::components::{Collider, CollisionLayers, ExternalForce, Position, RigidBody, Sensor};
use bevy::math::{Rect, Vec2, Vec3};
use bevy_ecs_ldtk::{GridCoords, LdtkLevel, LdtkWorldBundle, LevelSelection};
use bevy::hierarchy::{BuildChildren, Parent};
use bevy::asset::{Assets, AssetServer, Handle};
use std::collections::{HashMap, HashSet};
use bevy_ecs_ldtk::ldtk::LayerInstance;
use bevy::render::camera::ScalingMode;
use bevy::input::keyboard::KeyboardInput;
use bevy_xpbd_2d::collision::{CollisionEnded, CollisionStarted};
use bevy_xpbd_2d::math::Vector;
use bevy_xpbd_2d::prelude::*;
use crate::{GameCam, HEAD_SIZE, Layer, METERS_PER_PIXEL, PIXELS_PER_METER, Plate, WallRect};
use crate::components::{CameraFollow, InWater, Player, PlayerStart, Wall, Water};

pub fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    start_query: Query<(&GridCoords, &Parent), Added<PlayerStart>>,
) {
    if let Ok((gc, _)) = start_query.get_single() {

        let head = commands.spawn(
            (
                CameraFollow {},
                InWater {},
                SpriteBundle {
                    transform: Transform::from_xyz(
                        gc.x as f32 * PIXELS_PER_METER,
                        gc.y as f32 * PIXELS_PER_METER,
                        1.0,
                    ).with_scale(
                        Vec3::new(
                            METERS_PER_PIXEL,
                            METERS_PER_PIXEL,
                            1.0)),
                    texture: asset_server.load("sprites/head.png"),
                    ..default()
                },
                Player {},
                RigidBody::Dynamic,
                Position::from(Vec2 {
                    x: gc.x as f32 * PIXELS_PER_METER,
                    y: gc.y as f32 * PIXELS_PER_METER,
                }),
                ExternalForce::default().with_persistence(false),
                Collider::ball(HEAD_SIZE * METERS_PER_PIXEL / 2.0),
                CollisionLayers::new([Layer::Player], [Layer::Walls, Layer::Water])
            )
        )
            .id();

        let body = commands.spawn(
            (
                SpriteBundle {
                    transform: Transform::from_xyz(
                        gc.x as f32 * PIXELS_PER_METER,
                        gc.y as f32 * PIXELS_PER_METER,
                        1.0,
                    ).with_scale(
                        Vec3::new(
                            METERS_PER_PIXEL,
                            METERS_PER_PIXEL,
                            1.0)),
                    texture: asset_server.load("sprites/body.png"),
                    ..default()
                },
                InWater {},
                RigidBody::Dynamic,
                Position::from(Vec2 {
                    x: gc.x as f32 * PIXELS_PER_METER,
                    y: gc.y as f32 * PIXELS_PER_METER,
                }),
                ExternalForce::default().with_persistence(false),
                Collider::cuboid(HEAD_SIZE * METERS_PER_PIXEL / 2.0,HEAD_SIZE * METERS_PER_PIXEL * 8.0),
                CollisionLayers::new([Layer::Player], [Layer::Walls, Layer::Water])
            )
        )
            .id();

        commands.spawn(
            RevoluteJoint::new(head, body)
                .with_local_anchor_2(Vector::Y * 1.0)
                .with_angle_limits(-2.0, 2.0),
        );
    }
}

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2dBundle {
            projection: OrthographicProjection {
                scale: METERS_PER_PIXEL * 2.0,
                near: 0.0,
                far: 1000.0,
                viewport_origin: Vec2::new(0.5, 0.5),
                scaling_mode: ScalingMode::WindowSize(PIXELS_PER_METER),
                area: Rect::new(-1.0, -1.0, 1.0, 1.0),
            },
            ..default()
        },
        GameCam {},
    )
    );
}

pub fn load_map(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(LdtkWorldBundle {
        ldtk_handle: asset_server.load("maps/shafts.ldtk"),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..Default::default()
    });
}

pub fn camera_follow(to_follow: Query<&Transform, (With<CameraFollow>, Without<GameCam>)>,
                     mut camera: Query<&mut Transform, (With<GameCam>, Without<CameraFollow>)>,
) {
    let Ok(player_position) = to_follow.get_single() else { return; };
    let Ok(mut camera_transform) = camera.get_single_mut() else { return; };
    let target = Vec3 {
        x: player_position.translation.x,
        y: player_position.translation.y,
        z: camera_transform.translation.z,
    };


    camera_transform.translation = camera_transform.translation.lerp(target, 0.1);
}

/// Spawns heron collisions for the walls of a level
///
/// You could just insert a ColliderBundle in to the WallBundle,
/// but this spawns a different collider for EVERY wall tile.
/// This approach leads to bad performance.
///
/// Instead, by flagging the wall tiles and spawning the collisions later,
/// we can minimize the amount of colliding entities.
///
/// The algorithm used here is a nice compromise between simplicity, speed,
/// and a small number of rectangle colliders.
/// In basic terms, it will:
/// 1. consider where the walls are
/// 2. combine wall tiles into flat "plates" in each individual row
/// 3. combine the plates into rectangles across multiple rows wherever possible
/// 4. spawn colliders for each rectangle
pub fn spawn_wall_collision(
    mut commands: Commands,
    wall_query: Query<(&GridCoords, &Parent), Added<Wall>>,
    parent_query: Query<&Parent, Without<Wall>>,
    level_query: Query<(Entity, &Handle<LdtkLevel>)>,
    levels: Res<Assets<LdtkLevel>>,
) {

    // Consider where the walls are
    // storing them as GridCoords in a HashSet for quick, easy lookup
    //
    // The key of this map will be the entity of the level the wall belongs to.
    // This has two consequences in the resulting collision entities:
    // 1. it forces the walls to be split along level boundaries
    // 2. it lets us easily add the collision entities as children of the appropriate level entity
    let mut level_to_wall_locations: HashMap<Entity, HashSet<GridCoords>> = HashMap::new();

    wall_query.for_each(|(&grid_coords, parent)| {
        // An intgrid tile's direct parent will be a layer entity, not the level entity
        // To get the level entity, you need the tile's grandparent.
        // This is where parent_query comes in.
        if let Ok(grandparent) = parent_query.get(parent.get()) {
            level_to_wall_locations
                .entry(grandparent.get())
                .or_default()
                .insert(grid_coords);
        }
    });

    if !wall_query.is_empty() {
        level_query.for_each(|(level_entity, level_handle)| {
            if let Some(level_walls) = level_to_wall_locations.get(&level_entity) {
                let level = levels
                    .get(level_handle)
                    .expect("Level should be loaded by this point");

                let LayerInstance {
                    c_wid: width,
                    c_hei: height,
                    grid_size,
                    ..
                } = level
                    .level
                    .layer_instances
                    .clone()
                    .expect("Level asset should have layers")[0];

                // combine wall tiles into flat "plates" in each individual row
                let mut plate_stack: Vec<Vec<Plate>> = Vec::new();

                for y in 0..height {
                    let mut row_plates: Vec<Plate> = Vec::new();
                    let mut plate_start = None;

                    // + 1 to the width so the algorithm "terminates" plates that touch the right edge
                    for x in 0..width + 1 {
                        match (plate_start, level_walls.contains(&GridCoords { x, y })) {
                            (Some(s), false) => {
                                row_plates.push(Plate {
                                    left: s,
                                    right: x - 1,
                                });
                                plate_start = None;
                            }
                            (None, true) => plate_start = Some(x),
                            _ => (),
                        }
                    }

                    plate_stack.push(row_plates);
                }

                // combine "plates" into rectangles across multiple rows
                let mut rect_builder: HashMap<Plate, WallRect> = HashMap::new();
                let mut prev_row: Vec<Plate> = Vec::new();
                let mut wall_rects: Vec<WallRect> = Vec::new();

                // an extra empty row so the algorithm "finishes" the rects that touch the top edge
                plate_stack.push(Vec::new());

                for (y, current_row) in plate_stack.into_iter().enumerate() {
                    for prev_plate in &prev_row {
                        if !current_row.contains(prev_plate) {
                            // remove the finished rect so that the same plate in the future starts a new rect
                            if let Some(rect) = rect_builder.remove(prev_plate) {
                                wall_rects.push(rect);
                            }
                        }
                    }
                    for plate in &current_row {
                        rect_builder
                            .entry(plate.clone())
                            .and_modify(|e| e.top += 1)
                            .or_insert(WallRect {
                                bottom: y as i32,
                                top: y as i32,
                                left: plate.left,
                                right: plate.right,
                            });
                    }
                    prev_row = current_row;
                }

                commands.entity(level_entity).with_children(|level| {
                    // Spawn colliders for every rectangle..
                    // Making the collider a child of the level serves two purposes:
                    // 1. Adjusts the transforms to be relative to the level for free
                    // 2. the colliders will be despawned automatically when levels unload
                    for wall_rect in wall_rects {
                        level
                            .spawn_empty()
                            .insert(
                                (
                                    RigidBody::Static,
                                    Collider::cuboid((wall_rect.right as f32 - wall_rect.left as f32 + 1.)
                                                         * grid_size as f32
                                                     ,// /2., we're not using half extents because we're not using rapier
                                                     (wall_rect.top as f32 - wall_rect.bottom as f32 + 1.)
                                                         * grid_size as f32
                                                     , // / 2., full extents
                                    ),
                                    Position::from(Vec2 {
                                        x: (wall_rect.left + wall_rect.right + 1) as f32 * grid_size as f32
                                            / 2.,
                                        y: (wall_rect.bottom + wall_rect.top + 1) as f32 * grid_size as f32
                                            / 2.,
                                    }),
                                    CollisionLayers::new([Layer::Walls], [Layer::Player])
                                ));
                    }
                });
            }
        });
    }
}

pub fn spawn_water_sensors(
    mut commands: Commands,
    water_query: Query<(&GridCoords, &Parent), Added<Water>>,
    parent_query: Query<&Parent, Without<Water>>,
    level_query: Query<(Entity, &Handle<LdtkLevel>)>,
    levels: Res<Assets<LdtkLevel>>,
) {


    // Consider where the walls are
    // storing them as GridCoords in a HashSet for quick, easy lookup
    //
    // The key of this map will be the entity of the level the wall belongs to.
    // This has two consequences in the resulting collision entities:
    // 1. it forces the walls to be split along level boundaries
    // 2. it lets us easily add the collision entities as children of the appropriate level entity
    let mut level_to_water_locations: HashMap<Entity, HashSet<GridCoords>> = HashMap::new();

    water_query.for_each(|(&grid_coords, parent)| {
        // An intgrid tile's direct parent will be a layer entity, not the level entity
        // To get the level entity, you need the tile's grandparent.
        // This is where parent_query comes in.
        if let Ok(grandparent) = parent_query.get(parent.get()) {
            level_to_water_locations
                .entry(grandparent.get())
                .or_default()
                .insert(grid_coords);
        }
    });

    if !water_query.is_empty() {
        level_query.for_each(|(level_entity, level_handle)| {
            if let Some(level_water) = level_to_water_locations.get(&level_entity) {
                let level = levels
                    .get(level_handle)
                    .expect("Level should be loaded by this point");

                let LayerInstance {
                    c_wid: width,
                    c_hei: height,
                    grid_size,
                    ..
                } = level
                    .level
                    .layer_instances
                    .clone()
                    .expect("Level asset should have layers")[0];

                // combine wall tiles into flat "plates" in each individual row
                let mut plate_stack: Vec<Vec<Plate>> = Vec::new();

                for y in 0..height {
                    let mut row_plates: Vec<Plate> = Vec::new();
                    let mut plate_start = None;

                    // + 1 to the width so the algorithm "terminates" plates that touch the right edge
                    for x in 0..width + 1 {
                        match (plate_start, level_water.contains(&GridCoords { x, y })) {
                            (Some(s), false) => {
                                row_plates.push(Plate {
                                    left: s,
                                    right: x - 1,
                                });
                                plate_start = None;
                            }
                            (None, true) => plate_start = Some(x),
                            _ => (),
                        }
                    }

                    plate_stack.push(row_plates);
                }

                // combine "plates" into rectangles across multiple rows
                let mut rect_builder: HashMap<Plate, WallRect> = HashMap::new();
                let mut prev_row: Vec<Plate> = Vec::new();
                let mut water_rects: Vec<WallRect> = Vec::new();

                // an extra empty row so the algorithm "finishes" the rects that touch the top edge
                plate_stack.push(Vec::new());

                for (y, current_row) in plate_stack.into_iter().enumerate() {
                    for prev_plate in &prev_row {
                        if !current_row.contains(prev_plate) {
                            // remove the finished rect so that the same plate in the future starts a new rect
                            if let Some(rect) = rect_builder.remove(prev_plate) {
                                water_rects.push(rect);
                            }
                        }
                    }
                    for plate in &current_row {
                        rect_builder
                            .entry(plate.clone())
                            .and_modify(|e| e.top += 1)
                            .or_insert(WallRect {
                                bottom: y as i32,
                                top: y as i32,
                                left: plate.left,
                                right: plate.right,
                            });
                    }
                    prev_row = current_row;
                }

                commands
                    .entity(level_entity)
                    .with_children(|level| {
                        // Spawn colliders for every rectangle..
                        // Making the collider a child of the level serves two purposes:
                        // 1. Adjusts the transforms to be relative to the level for free
                        // 2. the colliders will be despawned automatically when levels unload
                        for water_rect in water_rects {
                            level
                                .spawn_empty()
                                .insert(
                                    (
                                        RigidBody::Static,
                                        Collider::cuboid(
                                            (water_rect.right as f32 - water_rect.left as f32 + 1.)
                                                * grid_size as f32
                                            ,// /2., we're not using half extents because we're not using rapier
                                            (water_rect.top as f32 - water_rect.bottom as f32 + 1.)
                                                * grid_size as f32
                                            , // / 2., full extents
                                        ),
                                        Position::from(
                                            Vec2 {
                                                x: (water_rect.left + water_rect.right + 1) as f32 * grid_size as f32
                                                    / 2.,
                                                y: (water_rect.bottom + water_rect.top + 1) as f32 * grid_size as f32
                                                    / 2.,
                                            }),
                                        Sensor,
                                        CollisionLayers::new([Layer::Water], [Layer::Player])
                                    ));
                        }
                    });
            }
        });
    }
}

pub fn water_started(mut collision_event_reader: EventReader<CollisionStarted>, query: Query<&CollisionLayers>, mut commands: Commands) {
    for CollisionStarted(entity1, entity2) in collision_event_reader.iter() {

        if let Ok([layers1, layers2]) = query.get_many([*entity1, *entity2]) {
            if layers1.contains_group(Layer::Player) && layers2.contains_group(Layer::Water) {
                println!("Entity 1 is in the Water!");
                commands.entity(*entity1).insert(InWater {});
            } else if layers1.contains_group(Layer::Water) && layers2.contains_group(Layer::Player)
            {
                println!("Entity 2 is in the Water!");
                commands.entity(*entity2).insert(InWater {});
            }
        }
    }
}

pub fn water_ended(mut collision_event_reader: EventReader<CollisionEnded>, query: Query<&CollisionLayers>, mut commands: Commands) {
    for CollisionEnded(entity1, entity2) in collision_event_reader.iter() {
        if let Ok([layers1, layers2]) = query.get_many([*entity1, *entity2]) {
            if layers1.contains_group(Layer::Player) && layers2.contains_group(Layer::Water) {
                println!("Entity 1 is out of the Water!");
                commands.entity(*entity1).remove::<InWater>();
            } else if layers1.contains_group(Layer::Water) && layers2.contains_group(Layer::Player)
            {
                println!("Entity 2 is out of the Water!");
                commands.entity(*entity2).remove::<InWater>();
            }
        }
    }
}

pub fn buoyancy(mut query: Query<&mut ExternalForce, With<InWater>>) {
    for mut force in query.iter_mut() {
        force.apply_force(Vec2 {x: 0.0, y:12.0});
    }
}

pub fn update_level_selection(
    level_query: Query<(&Handle<LdtkLevel>, &Transform), Without<Player>>,
    player_query: Query<&Transform, With<Player>>,
    mut level_selection: ResMut<LevelSelection>,
    ldtk_levels: Res<Assets<LdtkLevel>>,
) {
    for (level_handle, level_transform) in &level_query {
        if let Some(ldtk_level) = ldtk_levels.get(level_handle) {
            let level_bounds = Rect {
                min: Vec2::new(level_transform.translation.x, level_transform.translation.y),
                max: Vec2::new(
                    level_transform.translation.x + ldtk_level.level.px_wid as f32,
                    level_transform.translation.y + ldtk_level.level.px_hei as f32,
                ),
            };

            for player_transform in &player_query {
                if player_transform.translation.x < level_bounds.max.x
                    && player_transform.translation.x > level_bounds.min.x
                    && player_transform.translation.y < level_bounds.max.y
                    && player_transform.translation.y > level_bounds.min.y
                    && !level_selection.is_match(&0, &ldtk_level.level)
                {
                    *level_selection = LevelSelection::Iid(ldtk_level.level.iid.clone());
                }
            }
        }
    }
}

pub fn keyboard_input(
    mut key_evr: EventReader<KeyboardInput>,
    mut query: Query<&mut ExternalForce, With<Player>>
) {
    use bevy::input::ButtonState;
    use bevy::prelude::KeyCode;
    use bevy::math::vec2;
    if let Ok(mut external_force) = query.get_single_mut() {
        for ev in key_evr.iter() {
            match ev.state {
                ButtonState::Pressed => {
                    match ev.key_code {
                        Some(KeyCode::A) => {
                            external_force.apply_force(vec2(-5.0,0.0));
                        },
                        Some(KeyCode::D) => {
                            external_force.apply_force(vec2(5.0,0.0));
                        },
                        Some(KeyCode::W) => {
                            external_force.apply_force(vec2(0.0,5.0));
                        },
                        Some(KeyCode::S) => {
                            external_force.apply_force(vec2(0.0,-5.0));
                        },
                        _ => {}
                    }
                }
                ButtonState::Released => {
                    println!("Key release: {:?} ({})", ev.key_code, ev.scan_code);
                }
            }
        }
    }


}