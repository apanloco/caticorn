// TODO:
// [X] fix candy continuous spawning
// [X] fix cat-candy gravity
// [X] fix candy bounce sound when pressed to wall
// [X] tri-state: title, play, end
// [X] music
// [X] gulping sound
// [X] win state (press space to retry)
// [ ] start state (press space to start)
// [ ] spawn candy sound
// [ ] make cat fatter when eating?

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Turn on debug logs (specify multiple time for more verbose logs)
    #[arg(short = 'v', long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,
}

const PLAYER_SPEED: f32 = 600.0;
const CANDY_SPEED: f32 = 250.0;
const CANDY_SPAWN_TIMER_SECONDS: f32 = 1.0;

const NUMBER_OF_INITIAL_CANDIES: usize = 10;

#[derive(States, Default, Debug, Hash, Eq, PartialEq, Clone)]
pub enum GameState {
    #[default]
    Title,
    Playing,
    End,
}

#[derive(Component)]
pub struct Player {}

#[derive(Component)]
pub struct Candy {
    pub direction: Vec2,
    pub timestamp_changed_direction: f32,
}

#[derive(Component)]
pub struct Poop {}

#[derive(Component)]
pub struct Text {}

#[derive(Resource)]
pub struct CandyChangeDirectionSound {
    sounds: Vec<Handle<AudioSource>>,
}

impl CandyChangeDirectionSound {
    pub fn select_random(&self) -> Handle<AudioSource> {
        self.sounds[rand::random::<usize>() % self.sounds.len()].clone()
    }
}

#[derive(Resource)]
pub struct Music(Option<Handle<AudioSink>>);

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
    let args = Cli::parse();

    info!("args: {:?}", &args);

    App::new()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            filter: "wgpu=warn,naga=warn".into(),
            level: bevy::log::Level::DEBUG,
        }).set(ImagePlugin::default_nearest()))
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .insert_resource(CandySpawnTimer(Timer::from_seconds(CANDY_SPAWN_TIMER_SECONDS, TimerMode::Repeating)))
        .insert_resource(Music(None))
        .add_state::<GameState>()
        //.add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest())) // prevents blurry sprites
        .add_systems(Startup, setup)
        .add_systems(OnEnter(GameState::Title), title_setup)
        .add_systems(OnExit(GameState::Title), title_teardown)
        .add_systems(OnEnter(GameState::Playing), gameplay_setup)
        .add_systems(OnExit(GameState::Playing), gameplay_teardown)
        .add_systems(OnEnter(GameState::End), end_setup)
        .add_systems(OnExit(GameState::End), end_teardown)
        .add_systems(
            Update,
            (
                title_wait_for_keypress,
                title_player_pulse
            ).run_if(in_state(GameState::Title)),
        )
        .add_systems(
            Update,
            (
                gameplay_exit_to_title,
                gameplay_await_zero_candy,
                gameplay_player_movement,
                gameplay_candy_movement,
                gameplay_spawn_candy_timer,
                gameplay_update_candy_direction.after(gameplay_candy_movement),
                gameplay_player_candy_collision.after(gameplay_player_movement).after(gameplay_candy_movement),
                gameplay_confine_entity_movement.after(gameplay_player_candy_collision),
            ).run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            (
                end_sequence,
            ).run_if(in_state(GameState::End)),
        )
        .run();
}

fn calculate_confinement_rect(window: &Window, image: &Image, transform: &Transform) -> Rect {
    let half_size_x = (image.size().x * transform.scale.x) / 2.0;
    let half_size_y = (image.size().y * transform.scale.y) / 2.0;

    let min_x = -(window.width() / 2.0) + half_size_x;
    let max_x = (window.width() / 2.0) - half_size_x;
    let min_y = -(window.height() / 2.0) + half_size_y;
    let max_y = (window.height() / 2.0) - half_size_y;

    Rect {
        min_x,
        max_x,
        min_y,
        max_y,
    }
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    info!("setup");

    commands.spawn(Camera2dBundle::default());

    commands.insert_resource(CandyChangeDirectionSound {
        sounds: vec![asset_server.load("audio/candy_wall_collision_1.ogg"), asset_server.load("audio/candy_wall_collision_2.ogg")]
    });

    commands.insert_resource(PlayerCandyCollisionSound(asset_server.load("audio/caticorn_eat_candy.ogg")));

    let player_image = PlayerImage(
        asset_server.load("sprites/caticorn.png")
    );

    let candy_image = CandyImage(
        asset_server.load("sprites/donut.png"),
    );

    commands.spawn((
        SpriteBundle {
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            texture: player_image.clone(),
            ..default()
        },
        Player {},
    ));

    commands.insert_resource(player_image);
    commands.insert_resource(candy_image);
}

pub fn title_setup(
    mut commands: Commands,
    mut player_query: Query<&mut Transform, With<Player>>,
    asset_server: Res<AssetServer>,
    player_image: Res<PlayerImage>,
    audio: Res<Audio>,
    audio_sinks: Res<Assets<AudioSink>>,
    mut music: ResMut<Music>,
    entities: Query<Entity, (Without<Camera>, Without<Window>, Without<Player>)>,
) {
    info!("title_setup");

    for entity in &entities {
        commands.entity(entity).despawn();
    }

    if let Ok(mut transform) = player_query.get_single_mut() {
        transform.translation = Vec3::default();
        transform.scale = Vec3::new(1.0, 1.0, 1.0);
    }

    commands.spawn((
        TextBundle::from_section(
            "press SPACE to start",
            TextStyle {
                font: asset_server.load("fonts/MesloLGS NF Regular.ttf"),
                font_size: 50.0,
                color: Color::WHITE,
            },
        )
            .with_text_alignment(TextAlignment::Left)
            .with_style(Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(5.0),
                right: Val::Px(15.0),
                ..default()
            }),
        Text {},
    ));

    let weak_handle = audio.play_with_settings(
        asset_server.load("music/music_title.ogg"),
        PlaybackSettings {
            repeat: true,
            volume: Default::default(),
            speed: 1.0,
        },
    );

    let strong_handle = audio_sinks.get_handle(weak_handle);

    music.0 = Some(strong_handle);
}

pub fn title_player_pulse(
    mut player_query: Query<&mut Transform, With<Player>>,
    time: Res<Time>,
) {
    //debug!("title_player_pulse");

    let seconds = time.elapsed_seconds();
    if let Ok(mut transform) = player_query.get_single_mut() {
        let size = ((2.0 * seconds).sin() + 3.0).abs() / 2.0;
        trace!("size: {}", size);
        transform.scale.x = size;
        transform.scale.y = size;
    }
}

pub fn title_teardown(
    mut commands: Commands,
    entities: Query<Entity, With<Text>>,
    mut music: ResMut<Music>,
    audio_sinks: Res<Assets<AudioSink>>,
) {
    info!("title_teardown");

    for entity in &entities {
        commands.entity(entity).despawn();
    }

    if music.0.is_some() {
        if let Some(sink) = audio_sinks.get(music.0.as_ref().unwrap()) {
            sink.stop();
        }
        music.0 = None;
    }
}

pub fn title_wait_for_keypress(
    keyboard_input: Res<Input<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        info!("state => Playing");
        next_state.set(GameState::Playing)
    }
}

pub fn gameplay_setup(
    mut commands: Commands,
    mut player_query: Query<&mut Transform, With<Player>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    candy_image: Res<CandyImage>,
    audio: Res<Audio>,
    audio_sinks: Res<Assets<AudioSink>>,
    asset_server: Res<AssetServer>,
    mut music: ResMut<Music>,
) {
    info!("gameplay_setup");

    if let Ok(mut transform) = player_query.get_single_mut() {
        transform.scale.x = 1.0;
        transform.scale.y = 1.0;
    }

    let window = window_query.get_single().unwrap();

    for _ in 0..NUMBER_OF_INITIAL_CANDIES {
        spawn_candy(&mut commands, window, &candy_image);
    }

    let weak_handle = audio.play_with_settings(
        asset_server.load("music/music_gameplay.ogg"),
        PlaybackSettings {
            repeat: true,
            volume: Default::default(),
            speed: 1.0,
        },
    );

    let strong_handle = audio_sinks.get_handle(weak_handle);

    music.0 = Some(strong_handle);
}

pub fn gameplay_teardown(
    mut commands: Commands,
    mut music: ResMut<Music>,
    audio_sinks: Res<Assets<AudioSink>>,
    entities: Query<Entity, (Without<Camera>, Without<Window>, Without<Player>)>,
) {
    info!("gameplay_teardown");

    for entity in &entities {
        commands.entity(entity).despawn();
    }

    if music.0.is_some() {
        if let Some(sink) = audio_sinks.get(music.0.as_ref().unwrap()) {
            sink.stop();
        }
        music.0 = None;
    }
}

pub fn gameplay_spawn_candy_timer(
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
            timestamp_changed_direction: 0.0,
        },
    ));
}

pub fn gameplay_await_zero_candy(
    mut q: Query<(&Transform, &Candy)>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    audio: Res<Audio>,
    sound: Res<CandyChangeDirectionSound>,
    images: Res<Assets<Image>>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let candy_left = q.iter().len();
    if candy_left < 1 {
        next_state.set(GameState::End);
    }
}

pub fn gameplay_player_movement(
    keyboard_input: Res<Input<KeyCode>>,
    mut player_query: Query<&mut Transform, With<Player>>,
    time: Res<Time>,
) {
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

        // if direction.length() > 0.0 {
        //     direction = direction.normalize();
        // }

        transform.translation += direction * PLAYER_SPEED * time.delta_seconds()
    }
}

pub fn gameplay_exit_to_title(
    keyboard_input: Res<Input<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.pressed(KeyCode::Escape) {
        next_state.set(GameState::Title);
    }
    if keyboard_input.pressed(KeyCode::Return) {
        next_state.set(GameState::End);
    }
}

pub fn gameplay_candy_movement(
    mut candy_query: Query<(&mut Transform, &Candy), With<Candy>>,
    player_query: Query<&Transform, (With<Player>, Without<Candy>)>,
    time: Res<Time>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        error!("player query failed");
        return;
    };
    for (mut transform, candy) in candy_query.iter_mut() {
        let direction = Vec3::new(candy.direction.x, candy.direction.y, 0.0);
        transform.translation += direction * CANDY_SPEED * time.delta_seconds();

        let mut distance = transform.translation.distance(player_transform.translation);
        if distance < 200.0 {
            if distance < 25.0 {
                distance = 25.0;
            }
            let direction = Vec3::new(transform.translation.x - player_transform.translation.x, transform.translation.y - player_transform.translation.y, 0.0).normalize();
            let force = 400.0 - distance;

            transform.translation += direction * time.delta_seconds() * force;
        }
    }
}

pub fn gameplay_update_candy_direction(
    mut q: Query<(&Transform, &Handle<Image>, &mut Candy)>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    audio: Res<Audio>,
    sound: Res<CandyChangeDirectionSound>,
    images: Res<Assets<Image>>,
    time: Res<Time>,
) {
    let window = window_query.get_single().unwrap();

    for (transform, image_handle, mut candy) in q.iter_mut() {
        let Some(image) = images.get(image_handle) else {
            continue;
        };

        let rect = calculate_confinement_rect(window, image, transform);

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
            if time.elapsed_seconds() - candy.timestamp_changed_direction > 0.1 {
                audio.play(sound.select_random());
            } else {}
            candy.timestamp_changed_direction = time.elapsed_seconds();
        }
    }
}

pub fn gameplay_confine_entity_movement(
    mut query: Query<(&mut Transform, &Handle<Image>, Option<&Player>)>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    images: Res<Assets<Image>>,
) {
    let window = window_query.get_single().unwrap();
    for (mut transform, image_handle, player) in query.iter_mut() {
        let Some(image) = images.get(image_handle) else {
            continue;
        };

        let rect = calculate_confinement_rect(window, image, &transform);

        transform.translation.x = transform.translation.x.clamp(rect.min_x, rect.max_x);
        transform.translation.y = transform.translation.y.clamp(rect.min_y, rect.max_y);
    }
}

pub fn gameplay_player_candy_collision(
    mut commands: Commands,
    mut player_query: Query<(Entity, &Handle<Image>, &mut Transform), (With<Player>, Without<Candy>)>,
    candy_query: Query<(Entity, &Handle<Image>, &Transform), (With<Candy>, Without<Player>)>,
    audio: Res<Audio>,
    sound: Res<PlayerCandyCollisionSound>,
    images: Res<Assets<Image>>,
) {
    if let Ok((player_entity, player_image_handle, mut player_transform)) = player_query.get_single_mut() {
        let Some(player_image) = images.get(player_image_handle) else {
            error!("failed to get player image");
            return;
        };
        for (candy_entity, candy_image_handle, candy_transform) in candy_query.iter() {
            let Some(candy_image) = images.get(candy_image_handle) else {
                continue;
            };
            let mut distance = player_transform.translation.distance(candy_transform.translation);
            let half_size_player = (player_image.size().x + player_image.size().x) / 4.0 * player_transform.scale.x;
            let half_size_candy = (candy_image.size().x + candy_image.size().x) / 4.0 * candy_transform.scale.x;
            distance -= half_size_player;
            distance -= half_size_candy;
            if distance < -20.0 {
                audio.play(sound.clone());
                commands.entity(candy_entity).despawn();
                player_transform.scale.x += 0.03;
                player_transform.scale.y += 0.03;
            }
        }
    }
}


pub fn end_setup(
    mut commands: Commands,
    mut player_query: Query<&mut Transform, With<Player>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    audio: Res<Audio>,
    asset_server: Res<AssetServer>,
) {
    info!("end_setup");
    if let Ok(mut transform) = player_query.get_single_mut() {
        audio.play(asset_server.load("audio/end_fart.ogg"));

        commands.spawn((
            SpriteBundle {
                transform: transform.clone(),
                texture: asset_server.load("sprites/poop.png"),
                ..default()
            },
            Poop {},
        ));
    }
}

pub fn end_sequence(
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut player_query: Query<&mut Transform, (With<Player>, Without<Candy>, Without<Poop>)>,
    mut poop_query: Query<&mut Transform, (With<Poop>, Without<Candy>, Without<Player>)>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    debug!("end_sequence");

    let window = window_query.get_single().unwrap();

    if let Ok(mut transform) = player_query.get_single_mut() {
        transform.translation.x -= PLAYER_SPEED * time.delta_seconds();
    }
    if let Ok(mut transform) = poop_query.get_single_mut() {
        transform.translation.x += PLAYER_SPEED * time.delta_seconds();

        let max_x = window.width() * 1.5;

        if transform.translation.x > max_x {
            next_state.set(GameState::Title);
        }
    }
}

pub fn end_teardown(
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
    entities: Query<Entity, (Without<Camera>, Without<Window>, Without<Player>)>,
) {
    info!("end_teardown");
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}
