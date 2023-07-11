use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy_xpbd_2d::prelude::*;

const PIXELS_PER_METER: f32 = 32.0;
const METERS_PER_PIXEL: f32 = 1.0 / PIXELS_PER_METER;
const HEAD_SIZE: f32 = 8.0;


fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(
                ImagePlugin::default_nearest(),
            ),
        )
        .add_plugins(
            PhysicsPlugins::default()
        )
        .add_systems(Startup, spawn_camera)
        .add_systems(Startup,  spawn_world)
        .add_systems(Startup, spawn_player)
        .run();
}


#[derive(Component)]
pub struct Player {}

pub fn spawn_world(
    mut commands: Commands,
) {
    commands.spawn(
        (
            RigidBody::Static,
            Collider::cuboid(10.0, 1.0)
            )
    );
}

pub fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(
        (
            SpriteBundle {
                transform: Transform::from_xyz(0.0, 10.0, 0.0).with_scale(Vec3::new(METERS_PER_PIXEL,METERS_PER_PIXEL,METERS_PER_PIXEL)),
                texture: asset_server.load("sprites/head.png"),
                ..default()
            },
            Player {},
            RigidBody::Dynamic,
            Collider::ball(HEAD_SIZE * METERS_PER_PIXEL)
        )
    );
}

pub fn spawn_camera(mut commands: Commands) {
    let proj: OrthographicProjection = OrthographicProjection {
        scale: 1.0,
        near: 0.0,
        far: 100.0,
        viewport_origin: Vec2::new(0.5, 0.5),
        scaling_mode: ScalingMode::WindowSize(PIXELS_PER_METER),
        area: Rect::new(-1.0, -1.0, 1.0, 1.0),
    };

    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 1.0),
        projection: proj,
        ..default()
    });
}
