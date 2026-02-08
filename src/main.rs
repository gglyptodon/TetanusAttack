use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rand::prelude::*;

mod game;
use game::{Block, BlockColor, Cursor, Grid, SwapCmd};

const GRID_W: usize = 6;
const GRID_H: usize = 12;
const CELL_SIZE: f32 = 32.0;
const BLOCK_INSET: f32 = 6.0;
const FRAME_THICKNESS: f32 = 4.0;
const CURSOR_BORDER_THICKNESS: f32 = 2.0;
const PANEL_WIDTH: f32 = 140.0;
const PANEL_GAP: f32 = 16.0;
const PLAYER_GAP: f32 = 80.0;
const RISE_SECONDS: f32 = 2.5;
const RISE_SPEEDUP_INTERVAL: f32 = 30.0;
const RISE_SPEEDUP_FACTOR: f32 = 0.89;
const RISE_MIN_SECONDS: f32 = 0.8;
const GRAVITY_STEP_SECONDS: f32 = 0.1;
const CLEAR_DELAY_SECONDS: f32 = 0.1;
const RISE_PAUSE_SECONDS: f32 = 0.6;
const INPUT_REPEAT_DELAY: f32 = 0.25;
const INPUT_REPEAT_INTERVAL: f32 = 0.08;
const GARBAGE_CHAIN_BONUS: u32 = 2;
const GARBAGE_CHAIN_CAP: u32 = 24;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
enum AppState {
    #[default]
    Title,
    Game,
    Pause,
}

#[derive(Resource, Debug, Clone, Copy, Eq, PartialEq)]
enum GameMode {
    OnePlayer,
    TwoPlayer,
}

#[derive(Resource, Default)]
struct MenuSelection {
    two_player: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
enum PlayerId {
    P1,
    P2,
}

#[derive(Debug, Clone, Copy)]
enum PanelSide {
    Left,
    Right,
}

#[derive(Resource)]
struct Players {
    p1: PlayerState,
    p2: PlayerState,
}

#[derive(Resource)]
struct PlayerViews {
    p1: PlayerView,
    p2: Option<PlayerView>,
}

struct PlayerState {
    grid: Grid,
    cursor: Cursor,
    score: u32,
    elapsed: f32,
    pending_clear: bool,
    settled: bool,
    clear_timer: Timer,
    gravity_timer: Timer,
    rise_timer: Timer,
    rise_pause_timer: Timer,
    rise_paused: bool,
    rise_level: u32,
    repeat_dir: Option<IVec2>,
    repeat_timer: Timer,
    repeat_initial: bool,
    chain_active: bool,
    chain_index: u32,
    chain_ended: bool,
    garbage_outgoing: u32,
    garbage_incoming: u32,
}

impl PlayerState {
    fn new() -> Self {
        Self {
            grid: Grid::new(GRID_W, GRID_H),
            cursor: Cursor::new(0, 0),
            score: 0,
            elapsed: 0.0,
            pending_clear: false,
            settled: true,
            clear_timer: Timer::from_seconds(CLEAR_DELAY_SECONDS, TimerMode::Repeating),
            gravity_timer: Timer::from_seconds(GRAVITY_STEP_SECONDS, TimerMode::Repeating),
            rise_timer: Timer::from_seconds(RISE_SECONDS, TimerMode::Repeating),
            rise_pause_timer: Timer::from_seconds(RISE_PAUSE_SECONDS, TimerMode::Repeating),
            rise_paused: false,
            rise_level: 0,
            repeat_dir: None,
            repeat_timer: Timer::from_seconds(INPUT_REPEAT_DELAY, TimerMode::Once),
            repeat_initial: true,
            chain_active: false,
            chain_index: 0,
            chain_ended: false,
            garbage_outgoing: 0,
            garbage_incoming: 0,
        }
    }
}

#[derive(Resource)]
struct UiTexts {
    score: Entity,
    timer: Entity,
    status: Entity,
}

struct PlayerView {
    blocks: Vec<Entity>,
    cursor: Entity,
    panel: Entity,
    ui: UiTexts,
    origin: Vec2,
    panel_side: PanelSide,
}

#[derive(Resource)]
struct MenuRoot(Entity);

#[derive(Resource)]
struct MenuTextEntities {
    one_player: Entity,
    two_player: Entity,
}

#[derive(Resource)]
struct PauseRoot(Entity);

#[derive(Component)]
struct GameEntity;

#[derive(Resource, Default)]
struct GameInitialized(bool);

#[derive(Resource, Default)]
struct MatchOver {
    active: bool,
    winner: Option<PlayerId>,
}

#[derive(Resource, Default)]
struct MatchOverTimer {
    seconds: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<AppState>()
        .insert_resource(Players {
            p1: PlayerState::new(),
            p2: PlayerState::new(),
        })
        .insert_resource(GameMode::OnePlayer)
        .insert_resource(MenuSelection::default())
        .insert_resource(MatchOver::default())
        .insert_resource(MatchOverTimer::default())
        .insert_resource(GameInitialized::default())
        .add_systems(Startup, setup_camera)
        .add_systems(OnEnter(AppState::Title), (cleanup_game, setup_menu).chain())
        .add_systems(OnExit(AppState::Title), cleanup_menu)
        .add_systems(OnEnter(AppState::Game), setup_game)
        .add_systems(OnEnter(AppState::Pause), setup_pause)
        .add_systems(OnExit(AppState::Pause), cleanup_pause)
        .add_systems(Update, handle_menu_input.run_if(in_state(AppState::Title)))
        .add_systems(Update, handle_pause_input.run_if(in_state(AppState::Pause)))
        .add_systems(Update, handle_input.run_if(in_state(AppState::Game)))
        .add_systems(Update, handle_pause_request.run_if(in_state(AppState::Game)))
        .add_systems(Update, handle_restart.run_if(in_state(AppState::Game)))
        .add_systems(Update, handle_game_over_back.run_if(in_state(AppState::Game)))
        .add_systems(Update, apply_gravity_system.run_if(in_state(AppState::Game)))
        .add_systems(Update, update_time.run_if(in_state(AppState::Game)))
        .add_systems(Update, update_game_over_timer.run_if(in_state(AppState::Game)))
        .add_systems(Update, update_panel_layout.run_if(in_state(AppState::Game)))
        .add_systems(Update, update_visuals.run_if(in_state(AppState::Game)))
        .add_systems(Update, update_ui_text.run_if(in_state(AppState::Game)))
        .add_systems(Update, rise_stack.run_if(in_state(AppState::Game)))
        .add_systems(Update, update_clear_delay.run_if(in_state(AppState::Game)))
        .add_systems(
            Update,
            resolve_garbage
                .run_if(in_state(AppState::Game))
                .after(update_clear_delay),
        )
        .add_systems(Update, update_rise_pause.run_if(in_state(AppState::Game)))
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn setup_menu(mut commands: Commands, selection: Res<MenuSelection>) {
    let root = commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Percent(0.0),
                top: Val::Percent(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                ..Default::default()
            },
            background_color: BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.9)),
            ..Default::default()
        })
        .id();

    let mut one_player = None;
    let mut two_player = None;
    commands.entity(root).with_children(|parent| {
        parent.spawn(TextBundle {
            text: Text::from_section(
                "TETANUS ATTACK",
                TextStyle {
                    font: Default::default(),
                    font_size: 42.0,
                    color: Color::srgb(0.9, 0.9, 0.95),
                },
            ),
            ..Default::default()
        });

        one_player = Some(parent.spawn(TextBundle {
            text: Text::from_section(
                "1 PLAYER",
                TextStyle {
                    font: Default::default(),
                    font_size: 28.0,
                    color: if selection.two_player {
                        Color::srgb(0.7, 0.7, 0.75)
                    } else {
                        Color::srgb(0.2, 0.9, 0.6)
                    },
                },
            ),
            ..Default::default()
        }).id());

        two_player = Some(parent.spawn(TextBundle {
            text: Text::from_section(
                "2 PLAYER",
                TextStyle {
                    font: Default::default(),
                    font_size: 28.0,
                    color: if selection.two_player {
                        Color::srgb(0.2, 0.9, 0.6)
                    } else {
                        Color::srgb(0.7, 0.7, 0.75)
                    },
                },
            ),
            ..Default::default()
        }).id());

        parent.spawn(TextBundle {
            text: Text::from_section(
                "Press Enter / Space / Start",
                TextStyle {
                    font: Default::default(),
                    font_size: 18.0,
                    color: Color::srgb(0.7, 0.7, 0.75),
                },
            ),
            ..Default::default()
        });
    });

    commands.insert_resource(MenuRoot(root));
    if let (Some(one_player), Some(two_player)) = (one_player, two_player) {
        commands.insert_resource(MenuTextEntities { one_player, two_player });
    }
}

fn cleanup_menu(
    mut commands: Commands,
    menu: Res<MenuRoot>,
    menu_texts: Option<Res<MenuTextEntities>>,
) {
    commands.entity(menu.0).despawn_recursive();
    if let Some(menu_texts) = menu_texts {
        commands.remove_resource::<MenuTextEntities>();
    }
}

fn setup_pause(mut commands: Commands) {
    let root = commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Percent(0.0),
                top: Val::Percent(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                ..Default::default()
            },
            background_color: BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.75)),
            ..Default::default()
        })
        .id();

    commands.entity(root).with_children(|parent| {
        parent.spawn(TextBundle {
            text: Text::from_section(
                "PAUSED",
                TextStyle {
                    font: Default::default(),
                    font_size: 36.0,
                    color: Color::srgb(0.9, 0.9, 0.95),
                },
            ),
            ..Default::default()
        });

        parent.spawn(TextBundle {
            text: Text::from_section(
                "Press Esc / Tab / Start\nto Resume",
                TextStyle {
                    font: Default::default(),
                    font_size: 18.0,
                    color: Color::srgb(0.7, 0.7, 0.75),
                },
            ).with_justify(JustifyText::Center),
            ..Default::default()
        });
    });

    commands.insert_resource(PauseRoot(root));
}

fn cleanup_pause(mut commands: Commands, pause: Res<PauseRoot>) {
    commands.entity(pause.0).despawn_recursive();
}

fn cleanup_game(
    mut commands: Commands,
    entities: Query<Entity, With<GameEntity>>,
    mut initialized: ResMut<GameInitialized>,
) {
    for entity in &entities {
        commands.entity(entity).despawn_recursive();
    }
    initialized.0 = false;
}

fn handle_menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<GamepadButton>>,
    gamepads: Res<Gamepads>,
    mut selection: ResMut<MenuSelection>,
    mut mode: ResMut<GameMode>,
    menu_texts: Res<MenuTextEntities>,
    mut text_query: Query<&mut Text>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let mut changed = false;
    if keys.just_pressed(KeyCode::ArrowUp)
        || keys.just_pressed(KeyCode::ArrowDown)
        || keys.just_pressed(KeyCode::KeyW)
        || keys.just_pressed(KeyCode::KeyS)
    {
        selection.two_player = !selection.two_player;
        changed = true;
    }
    for gamepad_id in gamepads.iter() {
        if buttons.just_pressed(GamepadButton::new(gamepad_id, GamepadButtonType::DPadUp))
            || buttons.just_pressed(GamepadButton::new(gamepad_id, GamepadButtonType::DPadDown))
        {
            selection.two_player = !selection.two_player;
            changed = true;
            break;
        }
    }
    if changed {
        if let Ok(mut text) = text_query.get_mut(menu_texts.one_player) {
            text.sections[0].style.color = if selection.two_player {
                Color::srgb(0.7, 0.7, 0.75)
            } else {
                Color::srgb(0.2, 0.9, 0.6)
            };
        }
        if let Ok(mut text) = text_query.get_mut(menu_texts.two_player) {
            text.sections[0].style.color = if selection.two_player {
                Color::srgb(0.2, 0.9, 0.6)
            } else {
                Color::srgb(0.7, 0.7, 0.75)
            };
        }
    }

    let keyboard = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space);
    let mut gamepad = false;
    for gamepad_id in gamepads.iter() {
        gamepad |= buttons.just_pressed(GamepadButton::new(gamepad_id, GamepadButtonType::Start));
        gamepad |= buttons.just_pressed(GamepadButton::new(gamepad_id, GamepadButtonType::South));
    }
    if keyboard || gamepad {
        *mode = if selection.two_player {
            GameMode::TwoPlayer
        } else {
            GameMode::OnePlayer
        };
        next_state.set(AppState::Game);
    }
}

fn handle_pause_input(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<GamepadButton>>,
    gamepads: Res<Gamepads>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let keyboard = keys.just_pressed(KeyCode::Escape)
        || keys.just_pressed(KeyCode::Tab)
        || keys.just_pressed(KeyCode::Backspace);
    let mut gamepad = false;
    for gamepad_id in gamepads.iter() {
        gamepad |= buttons.just_pressed(GamepadButton::new(gamepad_id, GamepadButtonType::Start));
    }
    if keyboard || gamepad {
        next_state.set(AppState::Game);
    }
}

fn handle_pause_request(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<GamepadButton>>,
    gamepads: Res<Gamepads>,
    match_over: Res<MatchOver>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if match_over.active {
        return;
    }
    let keyboard = keys.just_pressed(KeyCode::Escape)
        || keys.just_pressed(KeyCode::Tab)
        || keys.just_pressed(KeyCode::Backspace);
    let mut gamepad = false;
    for gamepad_id in gamepads.iter() {
        gamepad |= buttons.just_pressed(GamepadButton::new(gamepad_id, GamepadButtonType::Start));
    }
    if keyboard || gamepad {
        next_state.set(AppState::Pause);
    }
}

fn setup_game(
    mut commands: Commands,
    mut players: ResMut<Players>,
    mode: Res<GameMode>,
    mut match_over: ResMut<MatchOver>,
    mut match_over_timer: ResMut<MatchOverTimer>,
    mut initialized: ResMut<GameInitialized>,
) {
    if initialized.0 {
        return;
    }
    reset_player(&mut players.p1);
    reset_player(&mut players.p2);
    match_over.active = false;
    match_over.winner = None;
    match_over_timer.seconds = 0.0;

    let (p1_origin, p2_origin) = compute_player_origins(*mode);

    let p1_view = spawn_player_view(
        &mut commands,
        &players.p1.grid,
        p1_origin,
        PanelSide::Right,
    );

    let p2_view = if *mode == GameMode::TwoPlayer {
        Some(spawn_player_view(
            &mut commands,
            &players.p2.grid,
            p2_origin,
            PanelSide::Left,
        ))
    } else {
        None
    };

    commands.insert_resource(PlayerViews { p1: p1_view, p2: p2_view });
    initialized.0 = true;
}

fn reset_player(player: &mut PlayerState) {
    player.grid.clear();
    player.grid.fill_test_pattern();
    player.cursor = Cursor::new(0, 0);
    player.score = 0;
    player.elapsed = 0.0;
    player.pending_clear = false;
    player.settled = true;
    player.clear_timer.reset();
    player.gravity_timer.reset();
    player.rise_timer.reset();
    player.rise_pause_timer.reset();
    player.rise_paused = false;
    player.rise_level = 0;
    player.rise_timer = Timer::from_seconds(RISE_SECONDS, TimerMode::Repeating);
    player.chain_active = false;
    player.chain_index = 0;
    player.chain_ended = false;
    player.garbage_outgoing = 0;
    player.garbage_incoming = 0;
}

fn compute_player_origins(mode: GameMode) -> (Vec2, Vec2) {
    let grid_w = GRID_W as f32 * CELL_SIZE;
    let total_player_w = grid_w + PANEL_WIDTH + PANEL_GAP;
    match mode {
        GameMode::OnePlayer => (Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0)),
        GameMode::TwoPlayer => {
            let p2_center_x = -(total_player_w / 2.0 + PLAYER_GAP / 2.0);
            let p1_center_x = total_player_w / 2.0 + PLAYER_GAP / 2.0;

            let p1_grid_center_x = p1_center_x - total_player_w / 2.0 + grid_w / 2.0;
            let p2_grid_center_x =
                p2_center_x - total_player_w / 2.0 + PANEL_WIDTH + PANEL_GAP + grid_w / 2.0;

            (
                Vec2::new(p1_grid_center_x, 0.0),
                Vec2::new(p2_grid_center_x, 0.0),
            )
        }
    }
}

fn spawn_player_view(
    commands: &mut Commands,
    grid: &Grid,
    origin: Vec2,
    panel_side: PanelSide,
) -> PlayerView {
    let panel = spawn_frame_and_panel(commands, origin, panel_side);
    spawn_background_grid(commands, grid, origin);
    let blocks = spawn_grid(commands, grid, origin);
    let cursor = spawn_cursor(commands, origin);
    let ui = spawn_ui_texts(commands, panel);
    PlayerView {
        blocks,
        cursor,
        panel,
        ui,
        origin,
        panel_side,
    }
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<GamepadButton>>,
    gamepads: Res<Gamepads>,
    time: Res<Time>,
    mut players: ResMut<Players>,
    mode: Res<GameMode>,
    match_over: Res<MatchOver>,
) {
    if match_over.active {
        return;
    }
    let delta = time.delta();
    let gamepad_ids: Vec<_> = gamepads.iter().collect();
    let p1_gamepad = gamepad_ids.first().copied();
    let p2_gamepad = if *mode == GameMode::TwoPlayer {
        gamepad_ids.get(1).copied()
    } else {
        None
    };

    handle_keyboard_p1(keys.as_ref(), &mut players.p1);
    if *mode == GameMode::TwoPlayer {
        handle_keyboard_p2(keys.as_ref(), &mut players.p2);
    }

    handle_gamepad(p1_gamepad, buttons.as_ref(), &mut players.p1);
    if *mode == GameMode::TwoPlayer {
        handle_gamepad(p2_gamepad, buttons.as_ref(), &mut players.p2);
    }

    handle_repeat_p1(keys.as_ref(), buttons.as_ref(), p1_gamepad, &mut players.p1, delta);
    if *mode == GameMode::TwoPlayer {
        handle_repeat_p2(keys.as_ref(), buttons.as_ref(), p2_gamepad, &mut players.p2, delta);
    }
}

fn handle_keyboard_p1(keys: &ButtonInput<KeyCode>, player: &mut PlayerState) {
    if keys.just_pressed(KeyCode::Space) {
        try_swap(player);
    }
}

fn handle_keyboard_p2(keys: &ButtonInput<KeyCode>, player: &mut PlayerState) {
    if keys.just_pressed(KeyCode::ShiftLeft) {
        try_swap(player);
    }
}

fn handle_gamepad(
    gamepad: Option<Gamepad>,
    buttons: &ButtonInput<GamepadButton>,
    player: &mut PlayerState,
) {
    let Some(gamepad) = gamepad else {
        return;
    };
    let swap = buttons.just_pressed(GamepadButton::new(gamepad, GamepadButtonType::South))
        || buttons.just_pressed(GamepadButton::new(gamepad, GamepadButtonType::East))
        || buttons.just_pressed(GamepadButton::new(gamepad, GamepadButtonType::West))
        || buttons.just_pressed(GamepadButton::new(gamepad, GamepadButtonType::North));
    if swap {
        try_swap(player);
    }
}

fn handle_repeat_p1(
    keys: &ButtonInput<KeyCode>,
    buttons: &ButtonInput<GamepadButton>,
    gamepad: Option<Gamepad>,
    player: &mut PlayerState,
    delta: std::time::Duration,
) {
    let (left_jp, left_p) = dir_state_p1(keys, buttons, gamepad, Direction::Left);
    let (right_jp, right_p) = dir_state_p1(keys, buttons, gamepad, Direction::Right);
    let (up_jp, up_p) = dir_state_p1(keys, buttons, gamepad, Direction::Up);
    let (down_jp, down_p) = dir_state_p1(keys, buttons, gamepad, Direction::Down);

    let dir = select_direction(
        player.repeat_dir,
        &[
            (left_jp, IVec2::new(-1, 0)),
            (right_jp, IVec2::new(1, 0)),
            (up_jp, IVec2::new(0, 1)),
            (down_jp, IVec2::new(0, -1)),
        ],
        &[
            (left_p, IVec2::new(-1, 0)),
            (right_p, IVec2::new(1, 0)),
            (up_p, IVec2::new(0, 1)),
            (down_p, IVec2::new(0, -1)),
        ],
    );
    update_repeat_move(player, dir, delta);
}

fn handle_repeat_p2(
    keys: &ButtonInput<KeyCode>,
    buttons: &ButtonInput<GamepadButton>,
    gamepad: Option<Gamepad>,
    player: &mut PlayerState,
    delta: std::time::Duration,
) {
    let (left_jp, left_p) = dir_state_p2(keys, buttons, gamepad, Direction::Left);
    let (right_jp, right_p) = dir_state_p2(keys, buttons, gamepad, Direction::Right);
    let (up_jp, up_p) = dir_state_p2(keys, buttons, gamepad, Direction::Up);
    let (down_jp, down_p) = dir_state_p2(keys, buttons, gamepad, Direction::Down);

    let dir = select_direction(
        player.repeat_dir,
        &[
            (left_jp, IVec2::new(-1, 0)),
            (right_jp, IVec2::new(1, 0)),
            (up_jp, IVec2::new(0, 1)),
            (down_jp, IVec2::new(0, -1)),
        ],
        &[
            (left_p, IVec2::new(-1, 0)),
            (right_p, IVec2::new(1, 0)),
            (up_p, IVec2::new(0, 1)),
            (down_p, IVec2::new(0, -1)),
        ],
    );
    update_repeat_move(player, dir, delta);
}

#[derive(Clone, Copy)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
}

fn dir_state_p1(
    keys: &ButtonInput<KeyCode>,
    buttons: &ButtonInput<GamepadButton>,
    gamepad: Option<Gamepad>,
    dir: Direction,
) -> (bool, bool) {
    let (key, button) = match dir {
        Direction::Left => (KeyCode::ArrowLeft, GamepadButtonType::DPadLeft),
        Direction::Right => (KeyCode::ArrowRight, GamepadButtonType::DPadRight),
        Direction::Up => (KeyCode::ArrowUp, GamepadButtonType::DPadUp),
        Direction::Down => (KeyCode::ArrowDown, GamepadButtonType::DPadDown),
    };
    let gp_pressed = gamepad.map_or(false, |pad| {
        buttons.pressed(GamepadButton::new(pad, button))
    });
    let gp_just = gamepad.map_or(false, |pad| {
        buttons.just_pressed(GamepadButton::new(pad, button))
    });
    (keys.just_pressed(key) || gp_just, keys.pressed(key) || gp_pressed)
}

fn dir_state_p2(
    keys: &ButtonInput<KeyCode>,
    buttons: &ButtonInput<GamepadButton>,
    gamepad: Option<Gamepad>,
    dir: Direction,
) -> (bool, bool) {
    let (key, button) = match dir {
        Direction::Left => (KeyCode::KeyA, GamepadButtonType::DPadLeft),
        Direction::Right => (KeyCode::KeyD, GamepadButtonType::DPadRight),
        Direction::Up => (KeyCode::KeyW, GamepadButtonType::DPadUp),
        Direction::Down => (KeyCode::KeyS, GamepadButtonType::DPadDown),
    };
    let gp_pressed = gamepad.map_or(false, |pad| {
        buttons.pressed(GamepadButton::new(pad, button))
    });
    let gp_just = gamepad.map_or(false, |pad| {
        buttons.just_pressed(GamepadButton::new(pad, button))
    });
    (keys.just_pressed(key) || gp_just, keys.pressed(key) || gp_pressed)
}

fn select_direction(
    current: Option<IVec2>,
    just_pressed: &[(bool, IVec2)],
    pressed: &[(bool, IVec2)],
) -> Option<IVec2> {
    for (is_just, dir) in just_pressed {
        if *is_just {
            return Some(*dir);
        }
    }
    if let Some(dir) = current {
        if pressed.iter().any(|(is_pressed, d)| *is_pressed && *d == dir) {
            return Some(dir);
        }
    }
    for (is_pressed, dir) in pressed {
        if *is_pressed {
            return Some(*dir);
        }
    }
    None
}

fn update_repeat_move(
    player: &mut PlayerState,
    dir: Option<IVec2>,
    delta: std::time::Duration,
) {
    if let Some(dir) = dir {
        let dir_changed = player.repeat_dir != Some(dir);
        if dir_changed {
            player.repeat_dir = Some(dir);
            player.repeat_initial = true;
            player.repeat_timer = Timer::from_seconds(INPUT_REPEAT_DELAY, TimerMode::Once);
            move_cursor(player, dir);
            return;
        }
        if player.repeat_timer.tick(delta).just_finished() {
            move_cursor(player, dir);
            if player.repeat_initial {
                player.repeat_initial = false;
                player.repeat_timer =
                    Timer::from_seconds(INPUT_REPEAT_INTERVAL, TimerMode::Repeating);
            }
        }
    } else {
        player.repeat_dir = None;
        player.repeat_initial = true;
        player.repeat_timer.reset();
    }
}

fn move_cursor(player: &mut PlayerState, dir: IVec2) {
    player
        .cursor
        .move_by(dir.x as isize, dir.y as isize, player.grid.width, player.grid.height);
}

fn try_swap(player: &mut PlayerState) {
    let cmd = SwapCmd::right_of(player.cursor.x, player.cursor.y);
    if player.grid.swap_in_bounds(cmd) && player.grid.has_matches() {
        player.pending_clear = true;
        player.clear_timer.reset();
    }
}

fn handle_restart(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<GamepadButton>>,
    mut players: ResMut<Players>,
    mut match_over: ResMut<MatchOver>,
    mut match_over_timer: ResMut<MatchOverTimer>,
) {
    if !match_over.active {
        return;
    }
    if match_over_timer.seconds < 1.0 {
        return;
    }
    let keyboard_restart = keys.get_just_pressed().any(|k| {
        *k != KeyCode::Escape && *k != KeyCode::Tab && *k != KeyCode::Backspace
    });
    let gamepad_restart = buttons
        .get_just_pressed()
        .any(|b| !matches!(b.button_type, GamepadButtonType::DPadUp
            | GamepadButtonType::DPadDown
            | GamepadButtonType::DPadLeft
            | GamepadButtonType::DPadRight
            | GamepadButtonType::Start
            | GamepadButtonType::Select
            | GamepadButtonType::Mode));
    if keyboard_restart || gamepad_restart {
        reset_player(&mut players.p1);
        reset_player(&mut players.p2);
        match_over_timer.seconds = 0.0;
        match_over.active = false;
        match_over.winner = None;
    }
}

fn handle_game_over_back(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<GamepadButton>>,
    match_over: Res<MatchOver>,
    match_over_timer: Res<MatchOverTimer>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if !match_over.active || match_over_timer.seconds < 1.0 {
        return;
    }
    let escape = keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Backspace);
    let mut gamepad = false;
    for button in buttons.get_just_pressed() {
        if matches!(
            button.button_type,
            GamepadButtonType::Start | GamepadButtonType::Select | GamepadButtonType::Mode
        ) {
            gamepad = true;
            break;
        }
    }
    if escape || gamepad {
        next_state.set(AppState::Title);
    }
}

fn rise_stack(
    time: Res<Time>,
    mut players: ResMut<Players>,
    mut match_over: ResMut<MatchOver>,
    mut match_over_timer: ResMut<MatchOverTimer>,
    mode: Res<GameMode>,
) {
    if match_over.active {
        return;
    }
    let delta = time.delta();
    let p1_over = rise_player(delta, &mut players.p1);
    let p2_over = if *mode == GameMode::TwoPlayer {
        rise_player(delta, &mut players.p2)
    } else {
        false
    };

    if p1_over {
        match_over.active = true;
        match_over.winner = if *mode == GameMode::TwoPlayer {
            Some(PlayerId::P2)
        } else {
            None
        };
        match_over_timer.seconds = 0.0;
    } else if p2_over {
        match_over.active = true;
        match_over.winner = Some(PlayerId::P1);
        match_over_timer.seconds = 0.0;
    }
}

fn rise_player(delta: std::time::Duration, player: &mut PlayerState) -> bool {
    if player.rise_timer.tick(delta).just_finished() {
        if player.rise_paused {
            return false;
        }
        if !player.settled || player.grid.has_falling_garbage() {
            return false;
        }
        if player.grid.top_row_occupied() {
            return true;
        }
        player.grid.push_bottom_row();
        if player.cursor.y + 1 < player.grid.height {
            player.cursor.y += 1;
        }
        if player.grid.has_matches() {
            player.pending_clear = true;
            player.clear_timer.reset();
        }
    }
    false
}

fn update_time(
    time: Res<Time>,
    mut players: ResMut<Players>,
    match_over: Res<MatchOver>,
    mode: Res<GameMode>,
) {
    if match_over.active {
        return;
    }
    let delta = time.delta_seconds();
    players.p1.elapsed += delta;
    update_rise_speed(&mut players.p1);
    if *mode == GameMode::TwoPlayer {
        players.p2.elapsed += delta;
        update_rise_speed(&mut players.p2);
    }
}

fn update_rise_speed(player: &mut PlayerState) {
    let level = (player.elapsed / RISE_SPEEDUP_INTERVAL).floor() as u32;
    if level <= player.rise_level {
        return;
    }
    player.rise_level = level;
    let mut seconds = RISE_SECONDS * RISE_SPEEDUP_FACTOR.powi(level as i32);
    if seconds < RISE_MIN_SECONDS {
        seconds = RISE_MIN_SECONDS;
    }
    player.rise_timer = Timer::from_seconds(seconds, TimerMode::Repeating);
}

fn update_game_over_timer(
    time: Res<Time>,
    mut timer: ResMut<MatchOverTimer>,
    match_over: Res<MatchOver>,
) {
    if match_over.active && timer.seconds < 1.0 {
        timer.seconds += time.delta_seconds();
    }
}

fn apply_gravity_system(
    time: Res<Time>,
    mut players: ResMut<Players>,
    match_over: Res<MatchOver>,
    mode: Res<GameMode>,
) {
    if match_over.active {
        return;
    }
    process_player_gravity(time.delta(), &mut players.p1);
    if *mode == GameMode::TwoPlayer {
        process_player_gravity(time.delta(), &mut players.p2);
    }
}

fn process_player_gravity(delta: std::time::Duration, player: &mut PlayerState) {
    if player.gravity_timer.tick(delta).just_finished() {
        let moved = player.grid.apply_gravity_step();
        if !moved {
            player.settled = true;
            let has_matches = player.grid.has_matches();
            if !player.pending_clear && has_matches {
                player.pending_clear = true;
                player.clear_timer.reset();
            }
            if player.chain_active && !player.pending_clear && !has_matches {
                player.chain_active = false;
                player.chain_index = 0;
                player.chain_ended = true;
                let converted = player.grid.convert_cracked_garbage();
                if converted > 0 && player.grid.has_matches() {
                    player.pending_clear = true;
                    player.clear_timer.reset();
                }
            }
        } else {
            player.settled = false;
            player.pending_clear = false;
        }
    }
}

fn update_clear_delay(
    time: Res<Time>,
    mut players: ResMut<Players>,
    match_over: Res<MatchOver>,
    mode: Res<GameMode>,
) {
    if match_over.active {
        return;
    }
    let delta = time.delta();
    process_clear_delay(delta, &mut players.p1);
    if *mode == GameMode::TwoPlayer {
        process_clear_delay(delta, &mut players.p2);
    }
}

fn process_clear_delay(delta: std::time::Duration, player: &mut PlayerState) {
    if !player.pending_clear || !player.settled {
        return;
    }
    if player.clear_timer.tick(delta).just_finished() {
        let stats = player.grid.clear_matches_once_with_stats();
        if stats.cleared > 0 {
            player.rise_paused = true;
            player.rise_pause_timer.reset();
            player.score += stats.cleared;
            player.grid.crack_adjacent_garbage(&stats.marks);
            if !player.chain_active {
                player.chain_active = true;
                player.chain_index = 1;
            } else {
                player.chain_index += 1;
            }
            add_garbage_for_clear(player, stats.cleared, stats.groups);
        }
        player.pending_clear = false;
    }
}

fn add_garbage_for_clear(player: &mut PlayerState, cleared: u32, groups: u32) {
    let combo_units = cleared.saturating_sub(3);
    let multi_units = groups.saturating_sub(1);
    let chain_units = if player.chain_index > 1 {
        GARBAGE_CHAIN_BONUS * (player.chain_index - 1)
    } else {
        0
    };
    let total = combo_units + multi_units + chain_units;
    if cleared < 4 && player.chain_index < 2 {
        return;
    }
    if total == 0 {
        return;
    }
    let remaining = GARBAGE_CHAIN_CAP.saturating_sub(player.garbage_outgoing);
    if remaining == 0 {
        return;
    }
    player.garbage_outgoing += total.min(remaining);
}

fn resolve_garbage(
    mut players: ResMut<Players>,
    match_over: Res<MatchOver>,
    mode: Res<GameMode>,
) {
    if match_over.active || *mode != GameMode::TwoPlayer {
        return;
    }

    if players.p1.chain_ended {
        if players.p1.garbage_outgoing > 0 {
            players.p2.garbage_incoming =
                players.p2.garbage_incoming.saturating_add(players.p1.garbage_outgoing);
            players.p1.garbage_outgoing = 0;
        }
        players.p1.chain_ended = false;
    }
    if players.p2.chain_ended {
        if players.p2.garbage_outgoing > 0 {
            players.p1.garbage_incoming =
                players.p1.garbage_incoming.saturating_add(players.p2.garbage_outgoing);
            players.p2.garbage_outgoing = 0;
        }
        players.p2.chain_ended = false;
    }

    let cancel = players.p1.garbage_incoming.min(players.p2.garbage_incoming);
    if cancel > 0 {
        players.p1.garbage_incoming -= cancel;
        players.p2.garbage_incoming -= cancel;
    }

    apply_incoming_garbage(&mut players.p1);
    apply_incoming_garbage(&mut players.p2);
}

fn apply_incoming_garbage(player: &mut PlayerState) {
    if player.garbage_incoming == 0 {
        return;
    }
    if player.pending_clear || !player.settled || player.rise_paused {
        return;
    }
    let units = player.garbage_incoming;
    player.garbage_incoming = 0;
    let mut rng = thread_rng();

    let rows = build_garbage_rows(player.grid.width, units, &mut rng);
    if !player.grid.insert_garbage_rows_from_top(&rows) {
        player.garbage_incoming = player.garbage_incoming.saturating_add(units);
        return;
    }
    player.settled = false;
}

fn build_garbage_rows(width: usize, units: u32, rng: &mut ThreadRng) -> Vec<Vec<bool>> {
    if units == 0 || width == 0 {
        return Vec::new();
    }
    let units = units as usize;
    let full_rows = units / width;
    let rem = units % width;
    let mut rows = Vec::with_capacity(full_rows + if rem > 0 { 1 } else { 0 });
    for _ in 0..full_rows {
        rows.push(vec![true; width]);
    }
    if rem > 0 {
        rows.push(build_partial_garbage_row(width, rem, rng));
    }
    rows
}

fn build_partial_garbage_row(width: usize, blocks: usize, rng: &mut ThreadRng) -> Vec<bool> {
    let mut mask = vec![false; width];
    if blocks >= width {
        mask.fill(true);
        return mask;
    }

    let max_start = width - blocks;
    let start = rng.gen_range(0..=max_start);
    for x in start..start + blocks {
        mask[x] = true;
    }

    mask
}

fn update_rise_pause(
    time: Res<Time>,
    mut players: ResMut<Players>,
    match_over: Res<MatchOver>,
    mode: Res<GameMode>,
) {
    if match_over.active {
        return;
    }
    let delta = time.delta();
    tick_rise_pause(delta, &mut players.p1);
    if *mode == GameMode::TwoPlayer {
        tick_rise_pause(delta, &mut players.p2);
    }
}

fn tick_rise_pause(delta: std::time::Duration, player: &mut PlayerState) {
    if player.rise_paused && player.rise_pause_timer.tick(delta).just_finished() {
        player.rise_paused = false;
    }
}

fn spawn_grid(commands: &mut Commands, grid: &Grid, origin: Vec2) -> Vec<Entity> {
    let mut entities = Vec::with_capacity(grid.width * grid.height);
    for y in 0..grid.height {
        for x in 0..grid.width {
            let pos = cell_center(grid, x, y, origin);
            let entity = commands
                .spawn(SpriteBundle {
                    sprite: Sprite {
                        color: Color::srgba(0.0, 0.0, 0.0, 0.0),
                        custom_size: Some(Vec2::splat(CELL_SIZE - BLOCK_INSET)),
                        ..Default::default()
                    },
                    transform: Transform::from_translation(pos),
                    ..Default::default()
                })
                .insert(GameEntity)
                .id();
            entities.push(entity);
        }
    }
    entities
}

fn spawn_background_grid(commands: &mut Commands, grid: &Grid, origin: Vec2) {
    for y in 0..grid.height {
        for x in 0..grid.width {
            let pos = cell_center(grid, x, y, origin);
            commands
                .spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::srgba(0.1, 0.1, 0.12, 0.35),
                    custom_size: Some(Vec2::splat(CELL_SIZE - 1.0)),
                    ..Default::default()
                },
                transform: Transform::from_translation(pos - Vec3::new(0.0, 0.0, 1.0)),
                ..Default::default()
            })
            .insert(GameEntity);
        }
    }
}

fn spawn_frame_and_panel(commands: &mut Commands, origin: Vec2, _panel_side: PanelSide) -> Entity {
    let grid_w = GRID_W as f32 * CELL_SIZE;
    let grid_h = GRID_H as f32 * CELL_SIZE;
    let half_w = grid_w / 2.0;
    let half_h = grid_h / 2.0;
    let border_color = Color::srgb(0.12, 0.12, 0.16);

    let origin3 = Vec3::new(origin.x, origin.y, 0.0);
    let top = origin3 + Vec3::new(0.0, half_h + FRAME_THICKNESS / 2.0, -0.5);
    let bottom = origin3 + Vec3::new(0.0, -half_h - FRAME_THICKNESS / 2.0, -0.5);
    let left = origin3 + Vec3::new(-half_w - FRAME_THICKNESS / 2.0, 0.0, -0.5);
    let right = origin3 + Vec3::new(half_w + FRAME_THICKNESS / 2.0, 0.0, -0.5);

    let horizontal_size = Vec2::new(grid_w + FRAME_THICKNESS * 2.0, FRAME_THICKNESS);
    let vertical_size = Vec2::new(FRAME_THICKNESS, grid_h);

    for (pos, size) in [
        (top, horizontal_size),
        (bottom, horizontal_size),
        (left, vertical_size),
        (right, vertical_size),
    ] {
        commands
            .spawn(SpriteBundle {
            sprite: Sprite {
                color: border_color,
                custom_size: Some(size),
                ..Default::default()
            },
            transform: Transform::from_translation(pos),
            ..Default::default()
        })
        .insert(GameEntity);
    }

    let panel = commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(PANEL_WIDTH),
                height: Val::Px(grid_h + FRAME_THICKNESS * 2.0),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            background_color: BackgroundColor(Color::srgb(0.07, 0.07, 0.09)),
            ..Default::default()
        })
        .insert(GameEntity)
        .id();

    commands.entity(panel).with_children(|parent| {
        parent
            .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Px(28.0),
                ..Default::default()
            },
            background_color: BackgroundColor(Color::srgb(0.12, 0.12, 0.16)),
            ..Default::default()
        })
        .insert(GameEntity);
    });

    panel
}

fn spawn_ui_texts(commands: &mut Commands, panel: Entity) -> UiTexts {
    let panel_margin = 16.0;
    let style = TextStyle {
        font: Default::default(),
        font_size: 20.0,
        color: Color::srgb(0.9, 0.9, 0.95),
    };

    let score = commands
        .spawn(TextBundle {
            text: Text::from_section("Score: 0", style.clone()),
            style: Style {
                margin: UiRect::all(Val::Px(panel_margin)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(GameEntity)
        .set_parent(panel)
        .id();

    let timer = commands
        .spawn(TextBundle {
            text: Text::from_section("Time: 0.0s", style),
            style: Style {
                margin: UiRect::left(Val::Px(panel_margin)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(GameEntity)
        .set_parent(panel)
        .id();

    let status = commands
        .spawn(TextBundle {
            text: Text::from_section(
                "GAME OVER - Press Any Button",
                TextStyle {
                    font: Default::default(),
                    font_size: 22.0,
                    color: Color::srgb(0.95, 0.2, 0.2),
                },
            ),
            style: Style {
                margin: UiRect::left(Val::Px(panel_margin)),
                ..Default::default()
            },
            visibility: Visibility::Hidden,
            ..Default::default()
        })
        .insert(GameEntity)
        .set_parent(panel)
        .id();

    UiTexts {
        score,
        timer,
        status,
    }
}

fn update_ui_text(
    players: Res<Players>,
    match_over: Res<MatchOver>,
    views: Res<PlayerViews>,
    mode: Res<GameMode>,
    mut text_query: Query<&mut Text>,
    mut vis_query: Query<&mut Visibility>,
) {
    update_player_ui(
        PlayerId::P1,
        &players.p1,
        &views.p1.ui,
        &match_over,
        &mut text_query,
        &mut vis_query,
    );
    if *mode == GameMode::TwoPlayer {
        if let Some(p2_view) = &views.p2 {
            update_player_ui(
                PlayerId::P2,
                &players.p2,
                &p2_view.ui,
                &match_over,
                &mut text_query,
                &mut vis_query,
            );
        }
    }
}

fn update_player_ui(
    player_id: PlayerId,
    player: &PlayerState,
    ui: &UiTexts,
    match_over: &MatchOver,
    text_query: &mut Query<&mut Text>,
    vis_query: &mut Query<&mut Visibility>,
) {
    if let Ok(mut text) = text_query.get_mut(ui.score) {
        text.sections[0].value = format!("Score: {}", player.score);
    }
    if let Ok(mut text) = text_query.get_mut(ui.timer) {
        text.sections[0].value = format!("Time: {:.1}s", player.elapsed);
    }

    if let Ok(mut visibility) = vis_query.get_mut(ui.status) {
        if match_over.active {
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }

    if match_over.active {
        if let Ok(mut text) = text_query.get_mut(ui.status) {
            let winner = match_over.winner;
            if winner == Some(player_id) {
                text.sections[0].value = "YOU WIN - Press Any Button".to_string();
            } else {
                text.sections[0].value = "GAME OVER - Press Any Button".to_string();
            }
        }
    }
}

fn update_panel_layout(
    windows: Query<&Window, With<PrimaryWindow>>,
    views: Res<PlayerViews>,
    mode: Res<GameMode>,
    mut style_query: Query<&mut Style>,
) {
    let window = match windows.get_single() {
        Ok(window) => window,
        Err(_) => return,
    };

    let grid_w = GRID_W as f32 * CELL_SIZE;
    let grid_h = GRID_H as f32 * CELL_SIZE;
    let panel_h = grid_h + FRAME_THICKNESS * 2.0;
    let top = (window.height() - panel_h) / 2.0;

    position_panel(
        &views.p1,
        window.width(),
        grid_w,
        panel_h,
        top,
        &mut style_query,
    );
    if *mode == GameMode::TwoPlayer {
        if let Some(p2_view) = &views.p2 {
            position_panel(
                p2_view,
                window.width(),
                grid_w,
                panel_h,
                top,
                &mut style_query,
            );
        }
    }
}

fn position_panel(
    view: &PlayerView,
    window_w: f32,
    grid_w: f32,
    panel_h: f32,
    top: f32,
    style_query: &mut Query<&mut Style>,
) {
    let left = match view.panel_side {
        PanelSide::Right => window_w / 2.0 + view.origin.x + grid_w / 2.0 + PANEL_GAP,
        PanelSide::Left => {
            window_w / 2.0 + view.origin.x - grid_w / 2.0 - PANEL_GAP - PANEL_WIDTH
        }
    };

    if let Ok(mut style) = style_query.get_mut(view.panel) {
        style.left = Val::Px(left);
        style.top = Val::Px(top.max(0.0));
        style.width = Val::Px(PANEL_WIDTH);
        style.height = Val::Px(panel_h);
    }
}

fn spawn_cursor(commands: &mut Commands, origin: Vec2) -> Entity {
    let width = CELL_SIZE * 2.0;
    let height = CELL_SIZE;
    let thickness = CURSOR_BORDER_THICKNESS;
    let color = Color::srgb(1.0, 1.0, 1.0);

    let cursor = commands
        .spawn(SpatialBundle {
            transform: Transform::from_translation(Vec3::new(origin.x, origin.y, 1.0)),
            ..Default::default()
        })
        .insert(GameEntity)
        .id();

    commands.entity(cursor).with_children(|parent| {
        let horizontal = Vec2::new(width, thickness);
        let vertical = Vec2::new(thickness, height);

        let top_y = height / 2.0 - thickness / 2.0;
        let bottom_y = -height / 2.0 + thickness / 2.0;
        let left_x = -width / 2.0 + thickness / 2.0;
        let right_x = width / 2.0 - thickness / 2.0;

        for (pos, size) in [
            (Vec3::new(0.0, top_y, 0.0), horizontal),
            (Vec3::new(0.0, bottom_y, 0.0), horizontal),
            (Vec3::new(left_x, 0.0, 0.0), vertical),
            (Vec3::new(right_x, 0.0, 0.0), vertical),
        ] {
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(size),
                    ..Default::default()
                },
                transform: Transform::from_translation(pos),
                ..Default::default()
            });
        }
    });

    cursor
}

fn update_visuals(
    players: Res<Players>,
    views: Res<PlayerViews>,
    mode: Res<GameMode>,
    mut sprite_query: Query<&mut Sprite>,
    mut transform_query: Query<&mut Transform>,
) {
    update_player_visuals(
        &players.p1,
        &views.p1,
        &mut sprite_query,
        &mut transform_query,
    );
    if *mode == GameMode::TwoPlayer {
        if let Some(p2_view) = &views.p2 {
            update_player_visuals(
                &players.p2,
                p2_view,
                &mut sprite_query,
                &mut transform_query,
            );
        }
    }
}

fn update_player_visuals(
    player: &PlayerState,
    view: &PlayerView,
    sprite_query: &mut Query<&mut Sprite>,
    transform_query: &mut Query<&mut Transform>,
) {
    for y in 0..player.grid.height {
        for x in 0..player.grid.width {
            let idx = y * player.grid.width + x;
            let color = match player.grid.get(x, y) {
                Some(Block::Normal { color }) => match color {
                    BlockColor::Red => Color::srgb(0.9, 0.36, 0.5),
                    BlockColor::Green => Color::srgb(0.18, 0.78, 0.5),
                    BlockColor::Blue => Color::srgb(0.36, 0.52, 0.96),
                    BlockColor::Yellow => Color::srgb(0.95, 0.76, 0.28),
                    BlockColor::Purple => Color::srgb(0.62, 0.4, 0.9),
                },
                Some(Block::Garbage { cracked: true }) => Color::srgb(0.58, 0.6, 0.62),
                Some(Block::Garbage { cracked: false }) => Color::srgb(0.36, 0.38, 0.4),
                None => Color::srgba(0.0, 0.0, 0.0, 0.0),
            };
            if let Some(entity) = view.blocks.get(idx) {
                if let Ok(mut sprite) = sprite_query.get_mut(*entity) {
                    sprite.color = color;
                }
            }
        }
    }

    let pos = cursor_center(&player.grid, player.cursor.x, player.cursor.y, view.origin);
    if let Ok(mut transform) = transform_query.get_mut(view.cursor) {
        *transform = Transform::from_translation(pos);
    }
}

fn cell_center(grid: &Grid, x: usize, y: usize, origin: Vec2) -> Vec3 {
    let origin_x = -((grid.width as f32) * CELL_SIZE) / 2.0 + CELL_SIZE / 2.0 + origin.x;
    let origin_y = -((grid.height as f32) * CELL_SIZE) / 2.0 + CELL_SIZE / 2.0 + origin.y;
    Vec3::new(
        origin_x + x as f32 * CELL_SIZE,
        origin_y + y as f32 * CELL_SIZE,
        0.0,
    )
}

fn cursor_center(grid: &Grid, x: usize, y: usize, origin: Vec2) -> Vec3 {
    let origin_x = -((grid.width as f32) * CELL_SIZE) / 2.0 + CELL_SIZE + origin.x;
    let origin_y = -((grid.height as f32) * CELL_SIZE) / 2.0 + CELL_SIZE / 2.0 + origin.y;
    Vec3::new(
        origin_x + x as f32 * CELL_SIZE,
        origin_y + y as f32 * CELL_SIZE,
        1.0,
    )
}
