use std::{
    io::Read,
    net::{IpAddr, TcpListener},
    str::from_utf8,
    sync::mpsc,
    thread,
    time::Duration,
};

use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy_simple_text_input::{TextInput, TextInputSubmitEvent};
use local_ip_address::local_ip;

use crate::{despawn_screen, FontResource};

use super::{room_create, room_enter, GameMode, ResultResponse, RoomRequest, User};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, States)]
enum ConnectState {
    Wait,
    #[default]
    Disabled,
}

pub struct ConnectPlugin;

impl Plugin for ConnectPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RoomRequest::new(0, User::new("", local_ip().unwrap(), 0.0)))
            .add_state::<ConnectState>()
            .add_systems(OnEnter(GameMode::Connect), connect_setup)
            .add_systems(OnExit(GameMode::Connect), despawn_screen::<ConnectScreen>)
            .add_systems(OnEnter(ConnectState::Wait), wait)
            .add_systems(
                Update,
                (
                    connect_button_system,
                    input_event,
                    focus,
                    measure_delta_seconds.run_if(on_timer(Duration::from_micros(150))),
                )
                    .run_if(in_state(ConnectState::Disabled)),
            );
    }
}

fn parse_user(str: &str) -> User {
    #[derive(serde::Deserialize)]
    struct ParseUser {
        name: String,
        ip: [u8; 4],
        delta_seconds: f32,
    }
    let ParseUser {
        name,
        ip,
        delta_seconds,
    } = serde_json::from_str(str).unwrap();

    User {
        name,
        ip: IpAddr::from(ip),
        delta_seconds,
    }
}

const RECV_TIMEOUT: Duration = Duration::from_millis(500);

// 相手が部屋に入るまで待つ
fn wait(
    mut commands: Commands,
    room: Res<RoomRequest>,
    mut connect_state: ResMut<NextState<ConnectState>>,
    mut game_state: ResMut<NextState<GameMode>>,
) {
    // 相手の情報を受け取るサーバー
    let ip = room.user.ip.to_string();

    let (user_s, user_r) = mpsc::channel();
    let (state_s, state_r) = mpsc::channel();
    let (game_state_s, game_state_r) = mpsc::channel();

    thread::spawn(move || {
        let server = TcpListener::bind((ip, 8888)).expect("サーバーエラー");
        match server.accept() {
            Ok((mut socket, addr)) => {
                println!("to addr {:?}", addr);

                let mut buf = [0; 120];
                match socket.read(&mut buf) {
                    Ok(bytes) => {
                        println!("{}", bytes);
                        let str = from_utf8(&buf[..bytes]).unwrap();
                        println!("{}", str);
                        let user = parse_user(str);
                        println!("{:?}", user);

                        user_s.send(user).expect("user send err");
                        state_s.send(ConnectState::Disabled).unwrap();
                        game_state_s.send(GameMode::VS).unwrap();
                    }
                    Err(e) => eprintln!("{}", e),
                }
            }
            Err(e) => eprintln!("{}", e),
        }
    });

    loop {
        if let Ok(user) = user_r.recv_timeout(RECV_TIMEOUT) {
            commands.insert_resource(user);
            break;
        }
    }
    loop {
        if let Ok(s) = state_r.recv_timeout(RECV_TIMEOUT) {
            connect_state.set(s);
            break;
        }
    }
    loop {
        if let Ok(s) = game_state_r.recv_timeout(RECV_TIMEOUT) {
            game_state.set(s);
            break;
        }
    }

    println!("end server...");
}

fn connect_setup(mut commands: Commands, font: Res<FontResource>) {
    let button_bundle = ButtonBundle {
        style: Style {
            width: Val::Px(200.),
            height: Val::Px(60.),
            margin: UiRect::vertical(Val::Px(20.)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ..default()
    };
    let button_text_style = TextStyle {
        font: font.0.clone(),
        font_size: 40.,
        color: Color::BLACK,
    };
    let text_style = TextStyle {
        font: font.0.clone(),
        font_size: 60.,
        color: Color::WHITE,
    };
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                ..default()
            },
            ConnectScreen,
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section("name", text_style.clone()));
            parent.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Px(200.0),
                        border: UiRect::all(Val::Px(5.0)),
                        padding: UiRect::all(Val::Px(5.0)),
                        margin: UiRect::vertical(Val::Px(5.)),
                        ..default()
                    },
                    border_color: BorderColor(Color::BLACK),
                    background_color: Color::WHITE.into(),
                    ..default()
                },
                TextInput {
                    text_style: TextStyle {
                        font: font.0.clone(),
                        font_size: 40.,
                        color: Color::BLACK,
                    },
                    ..default()
                },
                InfoSection::Name,
            ));
            parent.spawn(TextBundle::from_section("room id", text_style.clone()));
            parent.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Px(200.0),
                        border: UiRect::all(Val::Px(5.0)),
                        padding: UiRect::all(Val::Px(5.0)),
                        margin: UiRect::vertical(Val::Px(5.)),
                        ..default()
                    },
                    border_color: BorderColor(Color::BLACK),
                    background_color: Color::WHITE.into(),
                    ..default()
                },
                TextInput {
                    text_style: TextStyle {
                        font: font.0.clone(),
                        font_size: 40.,
                        color: Color::BLACK,
                    },
                    ..default()
                },
                InfoSection::RoomId,
            ));
            parent
                .spawn((button_bundle.clone(), ConnectSection::Create))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Create",
                        button_text_style.clone(),
                    ));
                });
            parent
                .spawn((button_bundle, ConnectSection::Enter))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section("Enter", button_text_style));
                });
        });
}

fn focus(
    query: Query<(Entity, &Interaction), Changed<Interaction>>,
    mut text_input_query: Query<(Entity, &mut TextInput)>,
) {
    for (interaction_entity, interaction) in &query {
        if *interaction == Interaction::Pressed {
            for (entity, mut text_input) in &mut text_input_query {
                if entity == interaction_entity {
                    text_input.inactive = false
                } else {
                    text_input.inactive = true
                }
            }
        }
    }
}

// 何秒毎に送ればいいか
fn measure_delta_seconds(
    mut delta_seconds: Local<(f32, u32)>,
    time: Res<Time>,
    mut room: ResMut<RoomRequest>,
) {
    delta_seconds.1 += 1;
    delta_seconds.0 += time.delta_seconds();
    let delta_time = delta_seconds.0 / delta_seconds.1 as f32;
    room.user.delta_seconds = delta_time;
}

#[derive(Component)]
struct ConnectScreen;

#[derive(Component)]
enum InfoSection {
    Name,
    RoomId,
}

#[derive(Component)]
enum ConnectSection {
    Create,
    Enter,
}

fn connect_button_system(
    mut commands: Commands,
    interaction: Query<(&Interaction, &ConnectSection), (Changed<Interaction>, With<Button>)>,
    room: Res<RoomRequest>,
    mut connect_state: ResMut<NextState<ConnectState>>,
    mut game_state: ResMut<NextState<GameMode>>,
) {
    for (interaction, section) in &interaction {
        if *interaction == Interaction::Pressed {
            let room = room.clone();
            match *section {
                ConnectSection::Create => {
                    if let Ok(res) = room_create(room) {
                        match res {
                            ResultResponse::Ok { .. } => {
                                connect_state.set(ConnectState::Wait);
                            }
                            ResultResponse::Err(m) => println!("{}", m),
                        }
                    } else {
                        eprintln!("エラー")
                    }
                }
                ConnectSection::Enter => {
                    if let Ok(res) = room_enter(room) {
                        match res {
                            ResultResponse::Ok { user, .. } => {
                                commands.insert_resource(user.unwrap());

                                connect_state.set(ConnectState::Disabled);
                                game_state.set(GameMode::VS);
                            }
                            ResultResponse::Err(m) => println!("{}", m),
                        }
                    } else {
                        eprintln!("エラー")
                    }
                }
            }
        }
    }
}

fn input_event(
    text_input_query: Query<(Entity, &InfoSection)>,
    mut event: EventReader<TextInputSubmitEvent>,
    mut room: ResMut<RoomRequest>,
) {
    for event in event.read() {
        for (entity, section) in &text_input_query {
            if entity == event.entity {
                let value = event.value.clone();
                match *section {
                    InfoSection::Name => room.user.name = value,
                    InfoSection::RoomId => {
                        if let Ok(value) = value.trim().parse() {
                            room.room_id = value
                        } else {
                            println!("数字ではありません");
                        }
                    }
                }
                println!("{:#?}", room);
                return;
            }
        }
    }
}
