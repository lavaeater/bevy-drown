use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(
            // This sets image filtering to nearest
            // This is done to prevent textures with low resolution (e.g. pixel art) from being blurred
            // by linear filtering.
            ImagePlugin::default_nearest(),
        ))
        .add_system(hello_world)
        .run();
}

pub fn hello_world() {
    println!("Hello World!")
}

#[derive(Component)]
pub struct Person {
    pub name: String 
}
