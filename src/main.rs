use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

const PLAYER_SPEED: f32 = 600.0;
const CANDY_SPEED: f32 = 250.0;

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

#[derive(Resource, Deref, DerefMut)]
pub struct CandyCounter(usize);

#[derive(Resource, Deref)]
pub struct PlayerCandyCollisionSound(Handle<AudioSource>);

#[derive(Resource, Deref)]
pub struct PlayerImage(Handle<Image>);

#[derive(Resource, Deref)]
pub struct CandyImage(Handle<Image>);

#[derive(Resource, Deref, DerefMut)]
pub struct CandySpawnTimer(Timer);

struct Rect {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .insert_resource(CandySpawnTimer(Timer::from_seconds(3.0, TimerMode::Repeating)))
        .insert_resource(CandyCounter(0))
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest())) // prevents blurry sprites
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update,
                     (
                         player_movement,
                         candy_movement,
                         spawn_candy_timer,
                         update_candy_direction.after(candy_movement),
                         player_candy_collision.after(player_movement).after(candy_movement),
                         confine_entity_movement.after(player_candy_collision),
                     ),
        )
        .run();
}

fn calculate_confinement_rect(window: &Window, image: &Image) -> Rect {
    let half_size = image.size().x / 2.0;

    let min_x = -(window.width() / 2.0) + half_size;
    let max_x = (window.width() / 2.0) - half_size;
    let min_y = -(window.height() / 2.0) + half_size;
    let max_y = (window.height() / 2.0) - half_size;

    Rect {
        min_x,
        max_x,
        min_y,
        max_y,
    }
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
        spawn_candy(&mut commands, window, &candy_image);
    }

    commands.insert_resource(player_image);
    commands.insert_resource(candy_image);
}

pub fn spawn_candy_timer(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<CandySpawnTimer>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    candy_image: Res<CandyImage>,
) {
    let window = window_query.get_single().unwrap();
    timer.tick(time.delta());
    if timer.just_finished() {
        spawn_candy(&mut commands, window, &candy_image);
    }
}

fn spawn_candy(commands: &mut Commands, window: &Window, candy_image: &CandyImage) {
    let random_pos_x = rand::random::<f32>() * window.width() - window.width() / 2.0;
    let random_pos_y = rand::random::<f32>() * window.height() - window.height() / 2.0;
    let random_dir_x = rand::random::<f32>();
    let random_dir_y = rand::random::<f32>();

    commands.spawn((
        SpriteBundle {
            transform: Transform::from_xyz(random_pos_x, random_pos_y, 0.0),
            texture: candy_image.0.clone(),
            ..default()
        },
        Candy {
            direction: Vec2::new(random_dir_x, random_dir_y).normalize(),
        },
    ));
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

pub fn candy_movement(
    mut candy_query: Query<(&mut Transform, &Candy), With<Candy>>,
    player_query: Query<&Transform, (With<Player>, Without<Candy>)>,
    time: Res<Time>,
) {
    // println!("2 {:?}", std::thread::current().id());
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    for (mut transform, candy) in candy_query.iter_mut() {
        let direction = Vec3::new(candy.direction.x, candy.direction.y, 0.0);
        //println!("candy direction: {:?}", &direction);
        transform.translation += direction * CANDY_SPEED * time.delta_seconds();

        let distance = transform.translation.distance(player_transform.translation);
        if distance < 300.0 {
            let direction = Vec3::new(transform.translation.x - player_transform.translation.x, transform.translation.y - player_transform.translation.y, 0.0).normalize();
            let force = 300.0 - distance;

            transform.translation += direction * time.delta_seconds() * force;
        }
    }
}

pub fn update_candy_direction(
    mut q: Query<(&Transform, &Handle<Image>, &mut Candy)>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    audio: Res<Audio>,
    sound: Res<CandyChangeDirectionSound>,
    images: Res<Assets<Image>>,
) {
//    println!("3 {:?}", std::thread::current().id());

    let window = window_query.get_single().unwrap();

    for (transform, image_handle, mut candy) in q.iter_mut() {
        let Some(image) = images.get(image_handle) else {
            continue;
        };

        let rect = calculate_confinement_rect(window, image);

        let mut changed_direction = false;
        let pos = transform.translation;

        if pos.x < rect.min_x || pos.x > rect.max_x {
            candy.direction.x *= -1.0;
            changed_direction = true;
        }

        if pos.y < rect.min_y || pos.y > rect.max_y {
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
    images: Res<Assets<Image>>,
) {
//    println!("4 {:?}", std::thread::current().id());
    let window = window_query.get_single().unwrap();
    for (mut transform, image_handle, player) in query.iter_mut() {
        let Some(image) = images.get(image_handle) else {
            continue;
        };

        let rect = calculate_confinement_rect(window, image);

        transform.translation.x = transform.translation.x.clamp(rect.min_x, rect.max_x);
        transform.translation.y = transform.translation.y.clamp(rect.min_y, rect.max_y);
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
