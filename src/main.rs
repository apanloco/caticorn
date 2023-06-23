use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

const PLAYER_SPEED: f32 = 600.0;
const CANDY_SPEED: f32 = 250.0;

const PLAYER_SIZE: f32 = 60.0;
const CANDY_SIZE: f32 = 32.0;

const NUMBER_OF_INITIAL_CANDIES: usize = 10;

#[derive(Component)]
pub struct Player {}

#[derive(Component)]
pub struct Candy {
    pub direction: Vec2,
}

#[derive(Resource)]
pub struct CandyChangeDirectionSound {
    sounds: Vec<Handle<AudioSource>>,
}

impl CandyChangeDirectionSound {
    pub fn select_random(&self) -> Handle<AudioSource> {
        self.sounds[rand::random::<usize>() % self.sounds.len()].clone()
    }
}

#[derive(Resource, Deref)]
pub struct PlayerCandyCollisionSound(Handle<AudioSource>);

#[derive(Resource, Deref)]
pub struct PlayerImage(Handle<Image>);

#[derive(Resource, Deref)]
pub struct CandyImage(Handle<Image>);

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest())) // prevents blurry sprites
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update,
                     (
                         player_movement,
                         candy_movement,
                         update_candy_direction.after(candy_movement),
                         player_candy_collision.after(player_movement).after(candy_movement),
                         confine_entity_movement.after(player_candy_collision),
                     ))
        .run();
}

pub fn setup(
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
) {
    let window = window_query.get_single().unwrap();

    commands.spawn(Camera2dBundle::default());

    commands.insert_resource(CandyChangeDirectionSound {
        sounds: vec![asset_server.load("audio/candy_wall_collision_1.ogg"), asset_server.load("audio/candy_wall_collision_2.ogg")]
    });

    commands.insert_resource(PlayerCandyCollisionSound(asset_server.load("audio/player_candy_collision.ogg")));

    let player_image = PlayerImage(
        asset_server.load("sprites/caticorn.png")
    );

    let candy_image = CandyImage(
        asset_server.load("sprites/rainbow-sphere.png"),
    );

    commands.spawn((
        SpriteBundle {
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            texture: player_image.clone(),
            ..default()
        },
        Player {},
    ));

    for _ in 0..NUMBER_OF_INITIAL_CANDIES {
        let random_pos_x = rand::random::<f32>() * window.width() - window.width() / 2.0;
        let random_pos_y = rand::random::<f32>() * window.height() - window.height() / 2.0;
        let random_dir_x = rand::random::<f32>();
        let random_dir_y = rand::random::<f32>();

        commands.spawn((
            SpriteBundle {
                transform: Transform::from_xyz(random_pos_x, random_pos_y, 0.0),
                texture: candy_image.clone(),
                ..default()
            },
            Candy {
                direction: Vec2::new(random_dir_x, random_dir_y).normalize(),
            },
        ));
    }

    commands.insert_resource(player_image);
    commands.insert_resource(candy_image);
}

pub fn player_movement(
    keyboard_input: Res<Input<KeyCode>>,
    mut player_query: Query<&mut Transform, With<Player>>,
    time: Res<Time>,
) {
    //println!("1 {:?}", std::thread::current().id());
    if let Ok(mut transform) = player_query.get_single_mut() {
        let mut direction = Vec3::ZERO;

        if keyboard_input.pressed(KeyCode::Left) || keyboard_input.pressed(KeyCode::A) {
            direction += Vec3::new(-1.0, 0.0, 0.0);
        }
        if keyboard_input.pressed(KeyCode::Right) || keyboard_input.pressed(KeyCode::D) {
            direction += Vec3::new(1.0, 0.0, 0.0);
        }
        if keyboard_input.pressed(KeyCode::Up) || keyboard_input.pressed(KeyCode::W) {
            direction += Vec3::new(0.0, 1.0, 0.0);
        }
        if keyboard_input.pressed(KeyCode::Down) || keyboard_input.pressed(KeyCode::S) {
            direction += Vec3::new(0.0, -1.0, 0.0);
        }

        if direction.length() > 0.0 {
            direction = direction.normalize();
        }

        transform.translation += direction * PLAYER_SPEED * time.delta_seconds()
    }
}

pub fn candy_movement(mut q: Query<(&mut Transform, &Candy)>, time: Res<Time>) {
    //println!("2 {:?}", std::thread::current().id());
    for (mut transform, candy) in q.iter_mut() {
        let direction = Vec3::new(candy.direction.x, candy.direction.y, 0.0);
        transform.translation += direction * CANDY_SPEED * time.delta_seconds();
    }
}

pub fn update_candy_direction(
    mut q: Query<(&Transform, &mut Candy)>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    audio: Res<Audio>,
    sound: Res<CandyChangeDirectionSound>,
) {
//    println!("3 {:?}", std::thread::current().id());
    for (transform, mut candy) in q.iter_mut() {
        let window = window_query.get_single().unwrap();
        let half_size = CANDY_SIZE / 2.0;
        let min_x = -(window.width() / 2.0) + half_size;
        let max_x = (window.width() / 2.0) - half_size;
        let min_y = -(window.height() / 2.0) + half_size;
        let max_y = (window.height() / 2.0) - half_size;
        let mut changed_direction = false;
        let pos = transform.translation;
        if pos.x < min_x || pos.x > max_x {
            candy.direction.x *= -1.0;
            changed_direction = true;
        }
        if pos.y < min_y || pos.y > max_y {
            candy.direction.y *= -1.0;
            changed_direction = true;
        }
        if changed_direction {
            audio.play(sound.select_random());
        }
    }
}

pub fn confine_entity_movement(
    mut query: Query<(&mut Transform, &Handle<Image>, Option<&Player>)>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
//    println!("4 {:?}", std::thread::current().id());
    for (mut transform, image_handle, player) in query.iter_mut() {
        let window = window_query.get_single().unwrap();
        let half_size = 32.0 / 2.0;
        let min_x = -(window.width() / 2.0) + half_size;
        let max_x = (window.width() / 2.0) - half_size;
        let min_y = -(window.height() / 2.0) + half_size;
        let max_y = (window.height() / 2.0) - half_size;

        transform.translation.x = transform.translation.x.clamp(min_x, max_x);
        transform.translation.y = transform.translation.y.clamp(min_y, max_y);
    }
}

pub fn player_candy_collision(
    mut commands: Commands,
    mut player_query: Query<(Entity, &Transform), With<Player>>,
    candy_query: Query<(Entity, &Transform), With<Candy>>,
    audio: Res<Audio>,
    sound: Res<PlayerCandyCollisionSound>,

) {
    // println!("5 {:?}", std::thread::current().id());
    if let Ok((player_entity, player_transform)) = player_query.get_single_mut() {
        for (candy_entity, candy_transform) in candy_query.iter() {
            let distance = player_transform.translation.distance(candy_transform.translation);
            if distance < 50.0 {
                audio.play(sound.clone());
                commands.entity(candy_entity).despawn();
            }
        }
    }
}
