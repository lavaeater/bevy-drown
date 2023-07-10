use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(
            // This sets image filtering to nearest
            // This is done to prevent textures with low resolution (e.g. pixel art) from being blurred
            // by linear filtering.
            ImagePlugin::default_nearest(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, print_names)
        .run();
}

pub fn print_names(person_query: Query<&Person>) {
    for person in person_query.iter() {
        println!("Name: {}", person.name);
    }
}

pub fn setup(mut commands: Commands) {
    commands
        .spawn(Person {
            name: "Elaina Proctor".to_string()
        });
}

#[derive(Component)]
pub struct Person {
    pub name: String 
}

#[derive(Component)]
pub struct Employed {
    pub job: Job
}

#[derive(Debug)]
pub enum Job {
    Doctor,
    FireFighter,
    PenisJouster
}
