use bevy::prelude::*;
use bevy_xpbd_2d::prelude::*;

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
                transform: Transform::from_xyz(0.0, 10.0, 0.0),
                texture: asset_server.load("sprites/head.png"),
                ..default()
            },
            Player {},
            RigidBody::Dynamic,
            Collider::ball(0.35)
        )
    );
}

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 1.0),
        ..default()
    });
}
