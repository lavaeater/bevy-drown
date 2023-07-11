use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy::render::camera::ScalingMode;

const PIXELS_PER_METER: f32 = 32.0;
const METERS_PER_PIXEL: f32 = 1.0 / PIXELS_PER_METER;
const HEAD_SIZE: f32 = 8.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(
            // This sets image filtering to nearest
            // This is done to prevent textures with low resolution (e.g. pixel art) from being blurred
            // by linear filtering.
            ImagePlugin::default_nearest(),
        ))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(32.0))
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_systems(Startup, spawn_camera)
        .add_systems(Startup, spawn_player)
        .add_systems(Startup, setup_physics)
        .run();
}

fn setup_physics(mut commands: Commands) {
    commands
        .spawn(Collider::cuboid(500.0, 50.0))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, -100.0, 0.0)));

    /* Create the bouncing ball. */
    commands
        .spawn(RigidBody::Dynamic)
        .insert(Collider::ball(50.0))
        .insert(Restitution::coefficient(0.95))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, 400.0, 0.0)));
}

#[derive(Component)]
pub struct Player {}

pub fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(
        (
            SpriteBundle {
                transform: Transform::from_xyz(0.0, 10.0, 0.0)
                    .with_scale(Vec3::new(
                        METERS_PER_PIXEL,
                        METERS_PER_PIXEL,
                        METERS_PER_PIXEL)
                    ),
                texture: asset_server.load("sprites/head.png"),
                ..default()
            },
            Player {},
        )
    );
}

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 1.0),
        projection: OrthographicProjection {
            scale: 1.0,
            near: 0.0,
            far: 100.0,
            viewport_origin: Vec2::new(0.5, 0.5),
            scaling_mode: ScalingMode::WindowSize(PIXELS_PER_METER),
            area: Rect::new(-1.0, -1.0, 1.0, 1.0),
        },
        ..default()
    });
}
