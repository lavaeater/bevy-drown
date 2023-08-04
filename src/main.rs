use bevy::prelude::*;
use bevy_xpbd_2d::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use components::{PlayerStartBundle, WallBundle, WaterBundle};
use crate::systems::systems::*;

mod systems;
mod components;


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
        .add_systems(Startup, spawn_camera)
        .add_systems(Startup, load_map)
        .add_systems(Update, spawn_wall_collision)
        .add_systems(Update, spawn_water_sensors)
        .add_systems(Update, spawn_player)
        .insert_resource(GizmoConfig { depth_bias: -1.0, ..default() })
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
        .add_systems(Update, update_level_selection)
        .add_systems(Update, camera_follow)
        .add_systems(Update, water_started)
        .add_systems(Update, water_ended)
        .add_systems(Update, buoyancy)
        .add_systems(Update, keyboard_input)
        .run();
}

#[derive(PhysicsLayer)]
enum Layer {
    Player,
    Enemy,
    Walls,
    Water
}

#[derive(Component)]
pub struct GameCam {}

/// Represents a wide wall that is 1 tile tall
/// Used to spawn wall collisions
#[derive(Clone, Eq, PartialEq, Debug, Default, Hash)]
struct Plate {
    left: i32,
    right: i32,
}

/// A simple rectangle type representing a wall of any size
struct WallRect {
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
}