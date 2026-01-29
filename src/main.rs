use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

mod game;
use game::{BlockColor, Cursor, Grid, SwapCmd};

const GRID_W: usize = 6;
const GRID_H: usize = 12;
const CELL_SIZE: f32 = 32.0;
const FRAME_THICKNESS: f32 = 4.0;
const PANEL_WIDTH: f32 = 140.0;
const PANEL_GAP: f32 = 16.0;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
enum AppState {
    #[default]
    Title,
    Game,
}

#[derive(Resource)]
struct BlockSprites {
    entities: Vec<Entity>,
}

#[derive(Resource)]
struct CursorSprite(Entity);

#[derive(Resource)]
struct RiseTimer(Timer);

#[derive(Resource)]
struct GravityTimer(Timer);

#[derive(Resource)]
struct ClearDelayTimer(Timer);

#[derive(Resource, Default)]
struct PendingClear(bool);

#[derive(Resource)]
struct RisePauseTimer(Timer);

#[derive(Resource, Default)]
struct RisePaused(bool);

#[derive(Resource, Default)]
struct Score(u32);

#[derive(Resource, Default)]
struct ElapsedTime(f32);

#[derive(Resource)]
struct UiTexts {
    score: Entity,
    timer: Entity,
    game_over: Entity,
}

#[derive(Resource)]
struct PanelEntity(Entity);

#[derive(Resource)]
struct MenuRoot(Entity);

#[derive(Resource, Default)]
struct GameOver(bool);

#[derive(Resource, Default)]
struct GameOverTimer {
    seconds: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<AppState>()
        .insert_resource(Grid::new(GRID_W, GRID_H))
        .insert_resource(Cursor::new(0, 0))
        .insert_resource(RiseTimer(Timer::from_seconds(2.5, TimerMode::Repeating)))
        .insert_resource(RisePauseTimer(Timer::from_seconds(0.6, TimerMode::Repeating)))
        .insert_resource(RisePaused::default())
        .insert_resource(GravityTimer(Timer::from_seconds(0.1, TimerMode::Repeating)))
        .insert_resource(ClearDelayTimer(Timer::from_seconds(0.01, TimerMode::Repeating)))
        .insert_resource(PendingClear::default())
        .insert_resource(Score::default())
        .insert_resource(ElapsedTime::default())
        .insert_resource(GameOver::default())
        .insert_resource(GameOverTimer::default())
        .add_systems(Startup, setup_camera)
        .add_systems(OnEnter(AppState::Title), setup_menu)
        .add_systems(OnExit(AppState::Title), cleanup_menu)
        .add_systems(OnEnter(AppState::Game), setup_game)
        .add_systems(Update, handle_menu_input.run_if(in_state(AppState::Title)))
        .add_systems(Update, handle_input.run_if(in_state(AppState::Game)))
        .add_systems(Update, handle_restart.run_if(in_state(AppState::Game)))
        .add_systems(Update, apply_gravity_system.run_if(in_state(AppState::Game)))
        .add_systems(Update, update_time.run_if(in_state(AppState::Game)))
        .add_systems(Update, update_game_over_timer.run_if(in_state(AppState::Game)))
        .add_systems(Update, update_panel_layout.run_if(in_state(AppState::Game)))
        .add_systems(Update, update_visuals.run_if(in_state(AppState::Game)))
        .add_systems(Update, update_ui_text.run_if(in_state(AppState::Game)))
        .add_systems(Update, rise_stack.run_if(in_state(AppState::Game)))
        .add_systems(Update, update_rise_pause.run_if(in_state(AppState::Game)))
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn setup_menu(mut commands: Commands) {
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

        parent.spawn(TextBundle {
            text: Text::from_section(
                "1 PLAYER",
                TextStyle {
                    font: Default::default(),
                    font_size: 28.0,
                    color: Color::srgb(0.2, 0.9, 0.6),
                },
            ),
            ..Default::default()
        });

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
}

fn cleanup_menu(mut commands: Commands, menu: Res<MenuRoot>) {
    commands.entity(menu.0).despawn_recursive();
}

fn handle_menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<GamepadButton>>,
    gamepads: Res<Gamepads>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let keyboard = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space);
    let mut gamepad = false;
    for gamepad_id in gamepads.iter() {
        gamepad |= buttons.just_pressed(GamepadButton::new(gamepad_id, GamepadButtonType::Start));
        gamepad |= buttons.just_pressed(GamepadButton::new(gamepad_id, GamepadButtonType::South));
    }
    if keyboard || gamepad {
        next_state.set(AppState::Game);
    }
}

fn setup_game(
    mut commands: Commands,
    mut grid: ResMut<Grid>,
    mut cursor: ResMut<Cursor>,
    mut pending_clear: ResMut<PendingClear>,
    mut clear_timer: ResMut<ClearDelayTimer>,
    mut score: ResMut<Score>,
    mut elapsed: ResMut<ElapsedTime>,
    mut game_over: ResMut<GameOver>,
    mut game_over_timer: ResMut<GameOverTimer>,
    mut rise_timer: ResMut<RiseTimer>,
    mut rise_pause_timer: ResMut<RisePauseTimer>,
    mut rise_paused: ResMut<RisePaused>,
    mut gravity_timer: ResMut<GravityTimer>,
) {
    grid.clear();
    grid.fill_test_pattern();
    *cursor = Cursor::new(0, 0);
    pending_clear.0 = false;
    clear_timer.0.reset();
    score.0 = 0;
    elapsed.0 = 0.0;
    game_over.0 = false;
    game_over_timer.seconds = 0.0;
    rise_timer.0.reset();
    rise_pause_timer.0.reset();
    rise_paused.0 = false;
    gravity_timer.0.reset();

    let panel = spawn_frame_and_panel(&mut commands);
    spawn_background_grid(&mut commands, &grid);
    let sprites = spawn_grid(&mut commands, &grid);
    let cursor_sprite = spawn_cursor(&mut commands);
    let ui_texts = spawn_ui_texts(&mut commands, panel);
    commands.insert_resource(BlockSprites { entities: sprites });
    commands.insert_resource(CursorSprite(cursor_sprite));
    commands.insert_resource(ui_texts);
    commands.insert_resource(PanelEntity(panel));
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<GamepadButton>>,
    gamepads: Res<Gamepads>,
    mut grid: ResMut<Grid>,
    mut cursor: ResMut<Cursor>,
    game_over: Res<GameOver>,
    mut pending_clear: ResMut<PendingClear>,
    mut clear_timer: ResMut<ClearDelayTimer>,
) {
    if game_over.0 {
        return;
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        cursor.move_by(-1, 0, grid.width, grid.height);
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        cursor.move_by(1, 0, grid.width, grid.height);
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        cursor.move_by(0, 1, grid.width, grid.height);
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        cursor.move_by(0, -1, grid.width, grid.height);
    }

    let mut swap_pressed = keys.just_pressed(KeyCode::Space);

    for gamepad in gamepads.iter() {
        if buttons.just_pressed(GamepadButton::new(gamepad, GamepadButtonType::DPadLeft)) {
            cursor.move_by(-1, 0, grid.width, grid.height);
        }
        if buttons.just_pressed(GamepadButton::new(gamepad, GamepadButtonType::DPadRight)) {
            cursor.move_by(1, 0, grid.width, grid.height);
        }
        if buttons.just_pressed(GamepadButton::new(gamepad, GamepadButtonType::DPadUp)) {
            cursor.move_by(0, 1, grid.width, grid.height);
        }
        if buttons.just_pressed(GamepadButton::new(gamepad, GamepadButtonType::DPadDown)) {
            cursor.move_by(0, -1, grid.width, grid.height);
        }

        swap_pressed |= buttons.just_pressed(GamepadButton::new(gamepad, GamepadButtonType::South));
        swap_pressed |= buttons.just_pressed(GamepadButton::new(gamepad, GamepadButtonType::East));
        swap_pressed |= buttons.just_pressed(GamepadButton::new(gamepad, GamepadButtonType::West));
        swap_pressed |= buttons.just_pressed(GamepadButton::new(gamepad, GamepadButtonType::North));
    }

    if swap_pressed {
        let cmd = SwapCmd::right_of(cursor.x, cursor.y);
        if grid.swap_in_bounds(cmd) {
            if grid.has_matches() {
                pending_clear.0 = true;
                clear_timer.0.reset();
            }
        }
    }
}

fn handle_restart(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<GamepadButton>>,
    mut grid: ResMut<Grid>,
    mut cursor: ResMut<Cursor>,
    mut rise_timer: ResMut<RiseTimer>,
    mut rise_pause_timer: ResMut<RisePauseTimer>,
    mut rise_paused: ResMut<RisePaused>,
    mut gravity_timer: ResMut<GravityTimer>,
    mut clear_timer: ResMut<ClearDelayTimer>,
    mut pending_clear: ResMut<PendingClear>,
    mut game_over: ResMut<GameOver>,
    mut score: ResMut<Score>,
    mut elapsed: ResMut<ElapsedTime>,
    mut game_over_timer: ResMut<GameOverTimer>,
) {
    if !game_over.0 {
        return;
    }
    if game_over_timer.seconds < 1.0 {
        return;
    }
    let keyboard_pressed = keys.get_just_pressed().next().is_some();
    let gamepad_pressed = buttons
        .get_just_pressed()
        .any(|b| !matches!(b.button_type, GamepadButtonType::DPadUp
            | GamepadButtonType::DPadDown
            | GamepadButtonType::DPadLeft
            | GamepadButtonType::DPadRight));
    if keyboard_pressed || gamepad_pressed {
        grid.clear();
        grid.fill_test_pattern();
        *cursor = Cursor::new(0, 0);
        rise_timer.0.reset();
        rise_pause_timer.0.reset();
        rise_paused.0 = false;
        gravity_timer.0.reset();
        clear_timer.0.reset();
        pending_clear.0 = false;
        score.0 = 0;
        elapsed.0 = 0.0;
        game_over_timer.seconds = 0.0;
        game_over.0 = false;
    }
}

fn rise_stack(
    time: Res<Time>,
    mut timer: ResMut<RiseTimer>,
    mut grid: ResMut<Grid>,
    mut game_over: ResMut<GameOver>,
    mut cursor: ResMut<Cursor>,
    mut pending_clear: ResMut<PendingClear>,
    mut clear_timer: ResMut<ClearDelayTimer>,
    mut game_over_timer: ResMut<GameOverTimer>,
    rise_paused: Res<RisePaused>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        if rise_paused.0 {
            return;
        }
        if grid.top_row_occupied() {
            game_over.0 = true;
            game_over_timer.seconds = 0.0;
            return;
        }
        grid.push_bottom_row();
        if cursor.y + 1 < grid.height {
            cursor.y += 1;
        }
        if grid.has_matches() {
            pending_clear.0 = true;
            clear_timer.0.reset();
        }
    }
}

fn update_time(time: Res<Time>, mut elapsed: ResMut<ElapsedTime>, game_over: Res<GameOver>) {
    if game_over.0 {
        return;
    }
    elapsed.0 += time.delta_seconds();
}

fn update_game_over_timer(time: Res<Time>, mut timer: ResMut<GameOverTimer>, game_over: Res<GameOver>) {
    if game_over.0 && timer.seconds < 1.0 {
        timer.seconds += time.delta_seconds();
    }
}

fn apply_gravity_system(
    time: Res<Time>,
    mut timer: ResMut<GravityTimer>,
    mut grid: ResMut<Grid>,
    game_over: Res<GameOver>,
    mut score: ResMut<Score>,
    mut pending_clear: ResMut<PendingClear>,
    mut clear_timer: ResMut<ClearDelayTimer>,
    mut rise_paused: ResMut<RisePaused>,
    mut rise_pause_timer: ResMut<RisePauseTimer>,
) {
    if game_over.0 {
        return;
    }
    if timer.0.tick(time.delta()).just_finished() {
        let moved = grid.apply_gravity_step();
        if !moved {
            if pending_clear.0 {
                if clear_timer.0.tick(time.delta()).just_finished() {
                    let cleared = grid.clear_matches_once();
                    if cleared > 0 {
                        rise_paused.0 = true;
                        rise_pause_timer.0.reset();
                        score.0 += cleared;
                    }
                    pending_clear.0 = false;
                }
            } else if grid.has_matches() {
                pending_clear.0 = true;
                clear_timer.0.reset();
            }
        } else {
            pending_clear.0 = false;
        }
    }
}

fn update_rise_pause(
    time: Res<Time>,
    mut rise_pause_timer: ResMut<RisePauseTimer>,
    mut rise_paused: ResMut<RisePaused>,
    game_over: Res<GameOver>,
) {
    if game_over.0 {
        return;
    }
    if rise_paused.0 && rise_pause_timer.0.tick(time.delta()).just_finished() {
        rise_paused.0 = false;
    }
}

fn spawn_grid(commands: &mut Commands, grid: &Grid) -> Vec<Entity> {
    let mut entities = Vec::with_capacity(grid.width * grid.height);
    for y in 0..grid.height {
        for x in 0..grid.width {
            let pos = cell_center(grid, x, y);
            let entity = commands
                .spawn(SpriteBundle {
                    sprite: Sprite {
                        color: Color::srgba(0.0, 0.0, 0.0, 0.0),
                        custom_size: Some(Vec2::splat(CELL_SIZE - 2.0)),
                        ..Default::default()
                    },
                    transform: Transform::from_translation(pos),
                    ..Default::default()
                })
                .id();
            entities.push(entity);
        }
    }
    entities
}

fn spawn_background_grid(commands: &mut Commands, grid: &Grid) {
    for y in 0..grid.height {
        for x in 0..grid.width {
            let pos = cell_center(grid, x, y);
            commands.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::srgba(0.1, 0.1, 0.12, 0.35),
                    custom_size: Some(Vec2::splat(CELL_SIZE - 1.0)),
                    ..Default::default()
                },
                transform: Transform::from_translation(pos - Vec3::new(0.0, 0.0, 1.0)),
                ..Default::default()
            });
        }
    }
}

fn spawn_frame_and_panel(commands: &mut Commands) -> Entity {
    let grid_w = GRID_W as f32 * CELL_SIZE;
    let grid_h = GRID_H as f32 * CELL_SIZE;
    let half_w = grid_w / 2.0;
    let half_h = grid_h / 2.0;
    let border_color = Color::srgb(0.12, 0.12, 0.16);

    let top = Vec3::new(0.0, half_h + FRAME_THICKNESS / 2.0, -0.5);
    let bottom = Vec3::new(0.0, -half_h - FRAME_THICKNESS / 2.0, -0.5);
    let left = Vec3::new(-half_w - FRAME_THICKNESS / 2.0, 0.0, -0.5);
    let right = Vec3::new(half_w + FRAME_THICKNESS / 2.0, 0.0, -0.5);

    let horizontal_size = Vec2::new(grid_w + FRAME_THICKNESS * 2.0, FRAME_THICKNESS);
    let vertical_size = Vec2::new(FRAME_THICKNESS, grid_h);

    for (pos, size) in [
        (top, horizontal_size),
        (bottom, horizontal_size),
        (left, vertical_size),
        (right, vertical_size),
    ] {
        commands.spawn(SpriteBundle {
            sprite: Sprite {
                color: border_color,
                custom_size: Some(size),
                ..Default::default()
            },
            transform: Transform::from_translation(pos),
            ..Default::default()
        });
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
        .id();

    commands.entity(panel).with_children(|parent| {
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Px(28.0),
                ..Default::default()
            },
            background_color: BackgroundColor(Color::srgb(0.12, 0.12, 0.16)),
            ..Default::default()
        });
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
        .set_parent(panel)
        .id();

    let game_over = commands
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
        .set_parent(panel)
        .id();

    UiTexts {
        score,
        timer,
        game_over,
    }
}

fn update_ui_text(
    score: Res<Score>,
    elapsed: Res<ElapsedTime>,
    game_over: Res<GameOver>,
    ui: Res<UiTexts>,
    mut text_query: Query<&mut Text>,
    mut vis_query: Query<&mut Visibility>,
) {
    if let Ok(mut text) = text_query.get_mut(ui.score) {
        text.sections[0].value = format!("Score: {}", score.0);
    }
    if let Ok(mut text) = text_query.get_mut(ui.timer) {
        text.sections[0].value = format!("Time: {:.1}s", elapsed.0);
    }
    if let Ok(mut visibility) = vis_query.get_mut(ui.game_over) {
        *visibility = if game_over.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn update_panel_layout(
    windows: Query<&Window, With<PrimaryWindow>>,
    panel: Res<PanelEntity>,
    mut style_query: Query<&mut Style>,
) {
    let window = match windows.get_single() {
        Ok(window) => window,
        Err(_) => return,
    };

    let grid_w = GRID_W as f32 * CELL_SIZE;
    let grid_h = GRID_H as f32 * CELL_SIZE;
    let panel_w = PANEL_WIDTH;
    let panel_h = grid_h + FRAME_THICKNESS * 2.0;
    let left = (window.width() / 2.0) + (grid_w / 2.0) + PANEL_GAP;
    let top = (window.height() - panel_h) / 2.0;

    if let Ok(mut style) = style_query.get_mut(panel.0) {
        style.left = Val::Px(left);
        style.top = Val::Px(top.max(0.0));
        style.width = Val::Px(panel_w);
        style.height = Val::Px(panel_h);
    }
}

fn spawn_cursor(commands: &mut Commands) -> Entity {
    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::srgba(1.0, 1.0, 1.0, 0.2),
                custom_size: Some(Vec2::new(CELL_SIZE * 2.0, CELL_SIZE)),
                ..Default::default()
            },
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
            ..Default::default()
        })
        .id()
}

fn update_visuals(
    grid: Res<Grid>,
    cursor: Res<Cursor>,
    sprites: Res<BlockSprites>,
    cursor_sprite: Res<CursorSprite>,
    mut sprite_query: Query<&mut Sprite>,
    mut transform_query: Query<&mut Transform>,
) {
    for y in 0..grid.height {
        for x in 0..grid.width {
            let idx = y * grid.width + x;
            let color = match grid.get(x, y).map(|b| b.color) {
                Some(BlockColor::Red) => Color::srgb(0.85, 0.2, 0.2),
                Some(BlockColor::Green) => Color::srgb(0.2, 0.8, 0.3),
                Some(BlockColor::Blue) => Color::srgb(0.2, 0.4, 0.9),
                Some(BlockColor::Yellow) => Color::srgb(0.9, 0.8, 0.2),
                Some(BlockColor::Purple) => Color::srgb(0.6, 0.3, 0.9),
                None => Color::srgba(0.0, 0.0, 0.0, 0.0),
            };
            if let Some(entity) = sprites.entities.get(idx) {
                if let Ok(mut sprite) = sprite_query.get_mut(*entity) {
                    sprite.color = color;
                }
            }
        }
    }

    let pos = cursor_center(&grid, cursor.x, cursor.y);
    if let Ok(mut transform) = transform_query.get_mut(cursor_sprite.0) {
        *transform = Transform::from_translation(pos);
    }
}

fn cell_center(grid: &Grid, x: usize, y: usize) -> Vec3 {
    let origin_x = -((grid.width as f32) * CELL_SIZE) / 2.0 + CELL_SIZE / 2.0;
    let origin_y = -((grid.height as f32) * CELL_SIZE) / 2.0 + CELL_SIZE / 2.0;
    Vec3::new(
        origin_x + x as f32 * CELL_SIZE,
        origin_y + y as f32 * CELL_SIZE,
        0.0,
    )
}

fn cursor_center(grid: &Grid, x: usize, y: usize) -> Vec3 {
    let origin_x = -((grid.width as f32) * CELL_SIZE) / 2.0 + CELL_SIZE;
    let origin_y = -((grid.height as f32) * CELL_SIZE) / 2.0 + CELL_SIZE / 2.0;
    Vec3::new(
        origin_x + x as f32 * CELL_SIZE,
        origin_y + y as f32 * CELL_SIZE,
        1.0,
    )
}
