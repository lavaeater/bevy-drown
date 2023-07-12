use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy_xpbd_2d::prelude::*;
use bevy_prototype_lyon::prelude::*;

const PIXELS_PER_METER: f32 = 32.0;
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
        .add_plugins(ShapePlugin)
        .add_systems(Startup, spawn_camera)
        .add_systems(Startup, spawn_world)
        .add_systems(Startup, spawn_player)
        .add_systems(Update, camera_follow)
        .run();
}


#[derive(Component)]
pub struct Player {}

#[derive(Component)]
pub struct CameraFollow {}

#[derive(Component)]
pub struct GameCam {}

pub fn spawn_world(
    mut commands: Commands,
) {
    /*
    We need four bodies
    They can be built from two different shapes
     */
    //Add a stupid shape for now?
    let long_shape = shapes::Rectangle {
        extents: Vec2::new(10.0, 0.1),
        origin: shapes::RectangleOrigin::Center,
    };

    let short_shape = shapes::Rectangle {
        extents: Vec2::new(0.1, 2.2),
        origin: shapes::RectangleOrigin::Center,
    };




    commands.spawn(
        (
            RigidBody::Static,
            Collider::cuboid(10.0, 0.1),
            Position::from(Vec2 { x: 0.0, y: 0.0 }),
            Rotation::from_degrees(45.0),
            ShapeBundle {
                path: GeometryBuilder::build_as(&long_shape),
                ..default()
            },
            Fill::color(Color::CYAN),
            Stroke::new(Color::BLACK, 0.01),
        )
    );

    let rot = Vec2::from_angle(45.0);

    let pos = rot.rotate(Vec2 { x: 5.0, y: 0.0 });// - rot.rotate(Vec2 { x: 5.0, y: 1.0 });

    commands.spawn(
        (
            RigidBody::Static,
            Collider::cuboid(0.1, 2.2),
            Position::from(pos),
            Rotation::from_degrees(45.0),
            ShapeBundle {
                path: GeometryBuilder::build_as(&short_shape),
                ..default()
            },
            Fill::color(Color::CYAN),
            Stroke::new(Color::BLACK, 0.01),
        )
    );
}

pub fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(
        (
            CameraFollow {},
            SpriteBundle {
                transform: Transform::from_xyz(0.0, 10.0, 0.0).with_scale(Vec3::new(METERS_PER_PIXEL, METERS_PER_PIXEL, METERS_PER_PIXEL)),
                texture: asset_server.load("sprites/head.png"),
                ..default()
            },
            Player {},
            RigidBody::Dynamic,
            Collider::ball(HEAD_SIZE * METERS_PER_PIXEL / 2.0)
        )
    );
}

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            projection: OrthographicProjection {
                scale: METERS_PER_PIXEL * 20.0,
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
