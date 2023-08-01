use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy_xpbd_2d::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use std::collections::{HashMap, HashSet};
use bevy_inspector_egui::quick::WorldInspectorPlugin;


const PIXELS_PER_METER: f32 = 8.0;
const METERS_PER_PIXEL: f32 = 1.0 / PIXELS_PER_METER;
const HEAD_SIZE: f32 = 8.0;


fn main() {
    App::new()
        .insert_resource(Msaa::Sample4)
        .add_plugins(
            DefaultPlugins.set(
                ImagePlugin::default_nearest(),
            ),
        )
        .add_plugins(
            PhysicsPlugins::default()
        )
        .add_plugins(LdtkPlugin)
        .add_plugins(WorldInspectorPlugin::new())
        .add_systems(Startup,spawn_camera)
        .add_systems(Startup,load_map)
        .add_systems(Update,spawn_wall_collision)
        .add_systems(Update,spawn_player)
        .insert_resource(LevelSelection::Index(0))
        .insert_resource(LdtkSettings {
            level_background: LevelBackground::Nonexistent,
            int_grid_rendering: IntGridRendering::Invisible,
            level_spawn_behavior: LevelSpawnBehavior::UseZeroTranslation,
            ..default()
        })
        .register_ldtk_int_cell::<WallBundle>(1)
        .register_ldtk_int_cell::<WaterBundle>(2)
        .register_ldtk_int_cell::<PlayerStartBundle>(3)
        .add_systems(Update,update_level_selection)
        .add_systems(Update,camera_follow)
        .run();
}


#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Component)]
pub struct Wall;

#[derive(Clone, Debug, Default, Bundle, LdtkIntCell)]
pub struct WallBundle {
    wall: Wall,
}


#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Component)]
pub struct Water;

#[derive(Clone, Debug, Default, Bundle, LdtkIntCell)]
pub struct WaterBundle {
    water: Water,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Component)]
pub struct PlayerStart;

#[derive(Clone, Debug, Default, Bundle, LdtkIntCell)]
pub struct PlayerStartBundle {
    player_start: PlayerStart,
}


#[derive(Component)]
pub struct Player {}

#[derive(Component)]
pub struct CameraFollow {}

#[derive(Component)]
pub struct GameCam {}

#[derive(Bundle, LdtkEntity)]
pub struct MapEntity {
    #[sprite_sheet_bundle]
    #[bundle()]
    sprite_bundle: SpriteSheetBundle,
}

#[derive(Bundle, LdtkIntCell)]
pub struct IntCell {
    #[bundle()]
    sprite_bundle: SpriteSheetBundle,
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

pub fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    start_query: Query<(&GridCoords, &Parent), Added<PlayerStart>>,
) {
    if let Ok((gc, _)) = start_query.get_single() {
        commands.spawn(
            (
                CameraFollow {},
                SpriteBundle {
                    transform: Transform::from_xyz(-gc.x as f32 * PIXELS_PER_METER, -gc.y as f32 * PIXELS_PER_METER,1.0).with_scale(Vec3::new(METERS_PER_PIXEL, METERS_PER_PIXEL, 1.0)),
                    texture: asset_server.load("sprites/head.png"),
                    ..default()
                },
                Player {},
                RigidBody::Dynamic,
                Position::from(Vec2{x: gc.x as f32 * PIXELS_PER_METER, y: gc.y as f32 * PIXELS_PER_METER}),
                Collider::ball(HEAD_SIZE * METERS_PER_PIXEL / 2.0)
            )
        );
    }
}

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2dBundle {
            projection: OrthographicProjection {
                scale: METERS_PER_PIXEL,
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

pub fn camera_follow(to_follow: Query<&Transform, (With<CameraFollow>, Without<GameCam>)>,
                     mut camera: Query<&mut Transform, (With<GameCam>, Without<CameraFollow>)>,
) {
    let Ok(player_position) = to_follow.get_single() else { return; };
    let Ok(mut camera_transform) = camera.get_single_mut() else { return; };
    let target = Vec3 { x: player_position.translation.x, y: player_position.translation.y, z: camera_transform.translation.z };


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
    /// Represents a wide wall that is 1 tile tall
    /// Used to spawn wall collisions
    #[derive(Clone, Eq, PartialEq, Debug, Default, Hash)]
    struct Plate {
        left: i32,
        right: i32,
    }

    /// A simple rectangle type representing a wall of any size
    struct Rect {
        left: i32,
        right: i32,
        top: i32,
        bottom: i32,
    }

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
                let mut rect_builder: HashMap<Plate, Rect> = HashMap::new();
                let mut prev_row: Vec<Plate> = Vec::new();
                let mut wall_rects: Vec<Rect> = Vec::new();

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
                            .or_insert(Rect {
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
                                (RigidBody::Static,
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
                            ));
                    }
                });
            }
        });
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