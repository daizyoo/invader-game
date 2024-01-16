use std::net::{IpAddr, UdpSocket};
use std::str;
use std::time::Duration;

use bevy::prelude::*;
use bevy::sprite::collide_aabb::collide;

use bevy::time::common_conditions::on_timer;
use local_ip_address::local_ip;

use crate::entity::{AttackMethod, PlayerAttackBundle};
use crate::{game::*, method_impl};
use crate::{Audio, FontResource, SoundEvent, TextureResource};

const INITIAL_OPPONENT_POSITION: Vec2 = Vec2::new(0., 350.);
const PLAYER_ATTACK_SPEED: f32 = 400.;

const READ_TIMEOUT: Option<Duration> = Some(Duration::from_millis(15));

const PLAYER_HP: Hp = 50;
const MY_SPEED: f32 = 300.;

static mut SEND_TIMER: f32 = 0.0;

type Hp = isize;

pub struct VSPlayer;

impl Plugin for VSPlayer {
    fn build(&self, app: &mut App) {
        app.init_resource::<Game>()
            .add_event::<InfoUpdate>()
            .add_systems(OnEnter(GameMode::VS), vs_player_setup)
            .add_systems(
                Update,
                (
                    // 自分
                    player_collision,
                    move_my_player,
                    move_player_attack,
                    player_attack,
                    // 敵
                    opponent_collision,
                    move_opponent,
                    move_opponent_attack,
                    opponent_attack,
                    //
                    player_pos_send
                        .run_if(on_timer(Duration::from_secs_f32(unsafe { SEND_TIMER }))),
                    hp_update,
                    hp_recv,
                )
                    .run_if(in_state(GameMode::VS)),
            )
            .add_plugins(ConnectPlugin);
    }
}

#[derive(Event)]
struct InfoUpdate {
    my: Hp,
    op: Hp,
}

#[derive(Component)]
enum InfoSection {
    My,
    Op,
}

struct Player {
    hp: Hp,
}

#[derive(Resource, Default)]
struct Game {
    my: Player,
    opponent: Player,
}

#[derive(Resource)]
struct Server {
    position: UdpSocket,
    attack: UdpSocket,
    info: UdpSocket,
}

#[derive(Component)]
struct My;

#[derive(Component)]
struct Opponent;

#[derive(Component)]
struct MyAttack(Attack);

#[derive(Component)]
struct OpponentAttack(Attack);

impl Default for Player {
    fn default() -> Self {
        Player { hp: PLAYER_HP }
    }
}

impl Player {
    fn damage(&mut self, power: Hp) {
        self.hp -= power
    }
}

impl Game {
    fn info(&self) -> InfoUpdate {
        InfoUpdate {
            my: self.my.hp,
            op: self.opponent.hp,
        }
    }
}

impl Server {
    // 座標のやり取り
    const POSITION_PORT: &'static str = ":8000";
    // 攻撃のやり取り
    const ATTACK_PORT: &'static str = ":7000";
    // ゲーム情報のやり取り
    const INFO_PORT: &'static str = ":6000";

    fn new(target_ip: IpAddr) -> Server {
        let ip = local_ip().unwrap().to_string();

        let position = UdpSocket::bind(ip.clone() + Self::POSITION_PORT).unwrap();
        let attack = UdpSocket::bind(ip.clone() + Self::ATTACK_PORT).unwrap();
        let info = UdpSocket::bind(ip + Self::INFO_PORT).unwrap();

        let _ = position.connect(target_ip.to_string() + Self::POSITION_PORT);
        let _ = attack.connect(target_ip.to_string() + Self::ATTACK_PORT);
        let _ = info.connect(target_ip.to_string() + Self::INFO_PORT);

        position.set_read_timeout(READ_TIMEOUT).unwrap();
        attack.set_read_timeout(READ_TIMEOUT).unwrap();
        info.set_read_timeout(READ_TIMEOUT).unwrap();

        Server {
            position,
            attack,
            info,
        }
    }

    // 座標を送る
    #[inline]
    fn position_send(&self, buf: &[u8]) {
        if let Err(e) = self.position.send(buf) {
            println!("send: {}", e);
        }
    }
    // 攻撃
    #[inline]
    fn attack_send(&self, buf: &[u8]) {
        if let Err(e) = self.attack.send(buf) {
            println!("send: {}", e);
        }
    }
    // ダメージを受けたら
    #[inline]
    fn damage_send_my(&self, buf: String) {
        let buf = buf.to_owned() + " o";
        if let Err(e) = self.info.send(buf.as_bytes()) {
            println!("send: {}", e);
        }
    }
    #[inline]
    fn damage_send_opponent(&self, buf: String) {
        let buf = buf.to_owned() + " m";
        if let Err(e) = self.info.send(buf.as_bytes()) {
            println!("send: {}", e);
        }
    }
}

fn vs_player_setup(
    mut commands: Commands,
    texture: Res<TextureResource>,
    font: Res<FontResource>,
    opponent: Res<User>,
) {
    commands.insert_resource(Server::new(opponent.ip));
    unsafe { SEND_TIMER = opponent.delta_seconds }

    commands.spawn((
        SpriteBundle {
            transform: Transform {
                translation: INITIAL_PLAYER_POSITION.extend(0.),
                scale: PLAYER_SIZE.extend(0.0),
                ..default()
            },
            texture: texture.player.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::new(2., 2.)),
                ..default()
            },
            ..default()
        },
        My,
    ));
    commands.spawn((
        SpriteBundle {
            transform: Transform {
                translation: INITIAL_OPPONENT_POSITION.extend(0.),
                scale: PLAYER_SIZE.extend(0.0),
                ..default()
            },
            texture: texture.player.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::new(2., 2.)),
                flip_y: true,
                ..default()
            },
            ..default()
        },
        Opponent,
    ));

    let text_style = TextStyle {
        font: font.0.clone(),
        font_size: 40.0,
        color: Color::WHITE,
    };

    commands
        .spawn(NodeBundle {
            style: Style {
                top: Val::Px(7.),
                left: Val::Px(7.),
                flex_direction: FlexDirection::Column,
                position_type: PositionType::Absolute,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    "My: ".to_owned() + &PLAYER_HP.to_string(),
                    text_style.clone(),
                ),
                InfoSection::My,
            ));
            parent.spawn((
                TextBundle::from_section("Op: ".to_owned() + &PLAYER_HP.to_string(), text_style),
                InfoSection::Op,
            ));
        });
}

// hpの情報を更新
fn hp_update(mut text_query: Query<(&mut Text, &InfoSection)>, mut event: EventReader<InfoUpdate>) {
    for hp in event.read() {
        for (mut text, section) in &mut text_query {
            match section {
                InfoSection::My => text.sections[0].value = "My: ".to_owned() + &hp.my.to_string(),
                InfoSection::Op => text.sections[0].value = "Op: ".to_owned() + &hp.op.to_string(),
            }
        }
    }
}

// プレイヤーに攻撃が当たると
fn player_collision(
    mut commands: Commands,
    player_query: Query<&Transform, With<My>>,
    attack_query: Query<(Entity, &Transform), With<OpponentAttack>>,
    mut game: ResMut<Game>,
    server: Res<Server>,
    mut info_event: EventWriter<InfoUpdate>,
) {
    let transform = player_query.single();

    let player_pos = transform.translation;
    let player_size = transform.scale.xy();

    for (entity, transfrom) in &attack_query {
        let collision = collide(
            player_pos,
            player_size,
            transfrom.translation,
            transfrom.scale.xy(),
        );
        if collision.is_some() {
            game.my.damage(1);

            server.damage_send_my(game.my.hp.to_string());

            info_event.send(game.info());

            commands.entity(entity).despawn();
        }
    }
}

// 敵に攻撃が当たると
fn opponent_collision(
    mut commands: Commands,
    opponent_query: Query<&Transform, With<Opponent>>,
    attack_query: Query<(Entity, &Transform), With<MyAttack>>,
    mut game: ResMut<Game>,
    server: Res<Server>,
    mut info_event: EventWriter<InfoUpdate>,
) {
    let transform = opponent_query.single();

    let translation = transform.translation;
    let size = transform.scale.xy();

    for (entity, transform) in &attack_query {
        let collision = collide(
            translation,
            size,
            transform.translation,
            transform.scale.xy(),
        );
        if collision.is_some() {
            game.opponent.damage(1);

            server.damage_send_opponent(game.opponent.hp.to_string());

            info_event.send(game.info());

            commands.entity(entity).despawn();
        }
    }
}

// プレイヤーを動かす
fn move_my_player(
    mut my_query: Query<&mut Transform, With<My>>,
    key: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    println!("{}", time.delta_seconds());
    let translation = &mut my_query.single_mut().translation;

    let mut direction_x = 0.0;
    let mut direction_y = 0.0;

    if key.pressed(KeyCode::Up) {
        direction_y += 1.0
    }
    if key.pressed(KeyCode::Down) {
        direction_y -= 1.0
    }
    if key.pressed(KeyCode::Right) {
        direction_x += 1.0
    }
    if key.pressed(KeyCode::Left) {
        direction_x -= 1.0
    }

    let new_position_x = translation.x + direction_x * MY_SPEED * time.delta_seconds();
    let new_position_y = translation.y + direction_y * MY_SPEED * time.delta_seconds();

    translation.x = new_position_x.clamp(-CLAMP_X, CLAMP_X);
    translation.y = new_position_y.clamp(-CLAMP_Y, CLAMP_Y);
}

// プレイヤーの攻撃
fn player_attack(
    mut commands: Commands,
    player_query: Query<&Transform, With<My>>,
    texture: Res<TextureResource>,
    key: Res<Input<KeyCode>>,
    server: Res<Server>,
    mut sound_event: EventWriter<SoundEvent>,
) {
    if key.just_pressed(KeyCode::Space) {
        let translation = player_query.single().translation.floor();
        let x = translation.x;
        let y = translation.y;

        server.attack_send(format!("{} {}", x, y).as_bytes());

        commands.spawn(PlayerAttackBundle::new(
            MyAttack::new(AttackType::Power),
            texture.player_attack.clone(),
            translation,
        ));

        sound_event.send(SoundEvent(Audio::PlayerAttack));
    }
}

// 敵の攻撃
fn opponent_attack(mut commands: Commands, texture: Res<TextureResource>, server: Res<Server>) {
    let mut buf = [0; 9];
    let Ok(buf_size) = server.attack.recv(&mut buf) else {
        return;
    };
    commands.spawn(PlayerAttackBundle::new(
        OpponentAttack::new(AttackType::Normal),
        texture.player_attack.clone(),
        to_pos(&buf[..buf_size]),
    ));
}

// プレイヤーの攻撃を動かす
fn move_player_attack(
    mut commands: Commands,
    mut attack_query: Query<(Entity, &mut Transform), With<MyAttack>>,
    time: Res<Time>,
) {
    for (entity, mut transform) in &mut attack_query {
        transform.translation.y += PLAYER_ATTACK_SPEED * time.delta_seconds();
        if PLAYER_ATTACK_DESPAWN_POINT < transform.translation.y {
            commands.entity(entity).despawn()
        }
    }
}

// 敵の攻撃を動かす
fn move_opponent_attack(
    mut commands: Commands,
    mut attack_query: Query<(Entity, &mut Transform), With<OpponentAttack>>,
    time: Res<Time>,
) {
    for (entity, mut transform) in &mut attack_query {
        transform.translation.y -= PLAYER_ATTACK_SPEED * time.delta_seconds();
        if -PLAYER_ATTACK_DESPAWN_POINT > transform.translation.y {
            commands.entity(entity).despawn()
        }
    }
}

// 敵を動かす
fn move_opponent(mut query: Query<&mut Transform, With<Opponent>>, server: Res<Server>) {
    let mut buf = [0; 21];
    let Ok(buf_size) = server.position.recv(&mut buf) else {
        return;
    };

    let buf = &buf[..buf_size];

    println!("{}", str::from_utf8(buf).unwrap());

    query.single_mut().translation = to_pos(buf);
}

// bufを座標に変換する
#[inline]
fn to_pos(buf: &[u8]) -> Vec3 {
    let str = str::from_utf8(buf).unwrap();
    let pos: Vec<&str> = str.split_whitespace().collect();

    let x: f32 = pos[0].trim().parse().unwrap();
    let y: f32 = pos[1].trim().parse().unwrap();

    Vec3::new(-x, -y, 0.0)
}

fn player_pos_send(player_query: Query<&Transform, With<My>>, server: Res<Server>) {
    let pos = player_query.single().translation;
    let pos = format!("{} {}", pos.x, pos.y);
    server.position_send(pos.as_bytes());
}

// hpを受信する
fn hp_recv(mut game: ResMut<Game>, server: Res<Server>, mut event: EventWriter<InfoUpdate>) {
    let mut buf = [0; 6];
    let Ok(buf_size) = server.info.recv(&mut buf) else {
        return;
    };

    let str = str::from_utf8(&buf[..buf_size]).unwrap();

    let mut vec: Vec<&str> = str.split_whitespace().collect();
    let hp: Hp = vec[0].trim().parse().unwrap();
    match vec.pop().unwrap() {
        "m" => game.my.hp = hp,
        "o" => game.opponent.hp = hp,
        _ => panic!(),
    }

    event.send(game.info());
}

method_impl!(My, MyAttack);
method_impl!(Opponent, OpponentAttack);
