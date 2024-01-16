mod connect;
mod game_menu;
mod game_over;
mod server;

pub use connect::ConnectPlugin;
pub use game_menu::*;
pub use game_over::*;
pub use server::*;

use std::marker::PhantomData;
use std::time::Duration;

use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

use self::AttackType::*;
use crate::entity::{AttackMethod, DamageEventMethod, EnemyPlugin, PlayerMethod, PlayerPlugin};
use crate::game_mode::*;

use crate::despawn_screen;
use crate::MainState;
use crate::{WINDOW_HEIGHT, WINDOW_WIDTH};

// プレイヤーが見えなくならないよう
pub const CLAMP_Y: f32 = WINDOW_HEIGHT / 2.0;
pub const CLAMP_X: f32 = WINDOW_WIDTH / 2.0;

// プレイヤーの初期位置
pub const INITIAL_PLAYER_POSITION: Vec2 = Vec2::new(0.0, -450.0);
// pub const INITIAL_OPPONENT_POSITION: Vec2 = Vec2::new(0.0, 450.0);
// HP
pub const INITIAL_PLAYER_HP: isize = 10;
pub const INITIAL_KILLCOUNT: usize = 0;
// 速度
pub const PLAYER_SPEED: f32 = 450.;
// サイズ
pub const PLAYER_SIZE: Vec2 = Vec2::new(50.0, 50.0);

// 攻撃の速度
// pub const PLAYER_ATTACK_SPEED: f32 = 20.0;
pub const ENEMY_ATTACK_SPEED: f32 = 0.8;

// 攻撃が消える場所
pub const ENEMY_ATTACK_DESPAWN_POINT: f32 = -WINDOW_HEIGHT / 2.;
pub const PLAYER_ATTACK_DESPAWN_POINT: f32 = WINDOW_HEIGHT / 2.;

pub const ENEMY_CREAT_NUMBER: usize = 10;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, States, Component)]
pub enum GameMode {
    Single,
    Tow,
    VS,
    Connect,
    #[default]
    Disabled,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameMode>()
            .add_event::<UpdateInfo>()
            .add_systems(OnEnter(MainState::Game), game_menu_setup)
            .add_systems(OnExit(GameMode::Disabled), despawn_screen::<GameMenuScreen>)
            .add_systems(
                Update,
                game_menu_system.run_if(in_state(GameMode::Disabled)),
            )
            .add_plugins((SinglePlay, TwoPlay, VSPlayer));
    }
}

pub struct GamePlayPlugin<P: Clone, A: Clone, E: Clone> {
    pub setting: PluginSetting<P, A, E>,
}

impl<P: Clone, A: Clone + PartialEq, E: Clone> PluginGroup for GamePlayPlugin<P, A, E>
where
    P: Component + PlayerMethod,
    A: Component + AttackMethod,
    E: Event + DamageEventMethod,
{
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(PlayerPlugin::<P, A, E> {
                setting: self.setting.clone(),
            })
            .add(EnemyPlugin::<P, A, E> {
                setting: self.setting,
            })
    }
}

#[derive(Clone, Copy)]
pub struct PluginSetting<P: Clone, A: Clone, E: Clone> {
    pub player: PhantomData<P>,
    pub enemy: PhantomData<A>,
    pub event: PhantomData<E>,
    pub player_attack_timer: Duration,
    pub enemy_create_timer: Duration,
    pub enemy_attack_timer: Duration,
    pub in_state: GameMode,
}

impl<P: Clone, A: Clone, E: Clone> Default for PluginSetting<P, A, E> {
    fn default() -> Self {
        PluginSetting {
            player: PhantomData::<P>,
            enemy: PhantomData::<A>,
            event: PhantomData::<E>,
            player_attack_timer: Duration::from_secs_f32(0.15),
            enemy_create_timer: Duration::from_secs_f32(5.),
            enemy_attack_timer: Duration::from_secs_f32(2.),
            in_state: GameMode::Disabled,
        }
    }
}

/// #Example
///
/// #[derive(Component, Clone, PartialEq)]
/// struct PlayerAttack(Attack);
///
#[derive(Component, Clone, Copy, PartialEq)]
pub struct Attack {
    pub hp: isize,
    pub attack: AttackType,
}

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub enum AttackType {
    Normal,
    Power,
    Shotgun,
    Shotgun2,
    Shotgun3,
    Shotgun4,
    Shotgun5,
    Rebound(bool),
    EnemyNormal,
}

impl Attack {
    #[inline]
    pub const fn new(attack: AttackType) -> Attack {
        Attack {
            hp: attack.power(),
            attack,
        }
    }
    #[inline]
    pub fn damage(&mut self, damage: isize) {
        self.hp -= damage
    }
}

impl AttackType {
    #[inline]
    pub fn custom_scale(&self) -> Vec2 {
        match self {
            Power => Vec2::new(4., 4.),
            _ => Vec2::new(2., 2.),
        }
    }
    #[inline]
    pub const fn scale(&self) -> Vec2 {
        match self {
            Normal => Vec2::new(20., 20.),
            Power => Vec2::new(40., 40.),
            Shotgun | Shotgun2 | Shotgun3 | Shotgun4 | Shotgun5 => Vec2::new(20., 20.),
            Rebound(_) => Vec2::new(20., 20.),
            EnemyNormal => Vec2::new(20., 20.),
        }
    }
    #[inline]
    pub const fn power(&self) -> isize {
        match self {
            Normal => 11,
            Power => 20,
            Shotgun => 4,
            Shotgun2 => 5,
            Shotgun3 => 6,
            Shotgun4 => 5,
            Shotgun5 => 4,
            Rebound(_) => 8,
            EnemyNormal => 3,
        }
    }
    #[inline]
    pub const fn y_speed(&self) -> f32 {
        match self {
            Rebound(_) => 2.0,
            _ => 15.0,
        }
    }
    #[inline]
    pub const fn x_speed(&self) -> f32 {
        match self {
            Shotgun => 3.0,
            Shotgun2 => 7.0,
            Shotgun3 => 0.0,
            Shotgun4 => -7.0,
            Shotgun5 => -3.0,
            Rebound(true) => 10.0,
            Rebound(false) => -10.0,
            _ => 0.0,
        }
    }
    #[inline]
    pub fn list(&self) -> Vec<AttackType> {
        match &self {
            Shotgun => vec![Shotgun, Shotgun2, Shotgun3, Shotgun4, Shotgun5],
            Rebound(_) => vec![Rebound(false), Rebound(true)],
            _ => vec![*self],
        }
    }
}

#[derive(Event, Default)]
pub struct UpdateInfo;

#[derive(Component)]
pub struct PlayerInfoScreen;
