use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;

mod game;
use game::{BlockColor, Cursor, Grid, SwapCmd};

const GRID_W: usize = 6;
const GRID_H: usize = 12;
const CELL_SIZE: f32 = 32.0;
const FRAME_THICKNESS: f32 = 4.0;
const PANEL_WIDTH: f32 = 140.0;
const PANEL_GAP: f32 = 16.0;

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

#[derive(Resource, Default)]
struct GameOver(bool);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Grid::new(GRID_W, GRID_H))
        .insert_resource(Cursor::new(0, 0))
        .insert_resource(RiseTimer(Timer::from_seconds(2.5, TimerMode::Repeating)))
        .insert_resource(GravityTimer(Timer::from_seconds(0.1, TimerMode::Repeating)))
        .insert_resource(Score::default())
        .insert_resource(ElapsedTime::default())
        .insert_resource(GameOver::default())
        .add_systems(Startup, setup)
        .add_systems(Update, handle_input)
        .add_systems(Update, handle_restart)
        .add_systems(Update, apply_gravity_system)
        .add_systems(Update, update_time)
        .add_systems(Update, update_visuals)
        .add_systems(Update, update_ui_text)
        .add_systems(Update, rise_stack)
        .run();
}

fn setup(mut commands: Commands, mut grid: ResMut<Grid>) {
    commands.spawn(Camera2dBundle::default());
    grid.fill_test_pattern();
    spawn_frame_and_panel(&mut commands);
    spawn_background_grid(&mut commands, &grid);
    let sprites = spawn_grid(&mut commands, &grid);
    let cursor_sprite = spawn_cursor(&mut commands);
    let ui_texts = spawn_ui_texts(&mut commands);
    commands.insert_resource(BlockSprites { entities: sprites });
    commands.insert_resource(CursorSprite(cursor_sprite));
    commands.insert_resource(ui_texts);
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut grid: ResMut<Grid>,
    mut cursor: ResMut<Cursor>,
    game_over: Res<GameOver>,
    mut score: ResMut<Score>,
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

    if keys.just_pressed(KeyCode::Space) {
        let cmd = SwapCmd::right_of(cursor.x, cursor.y);
        if grid.swap_in_bounds(cmd) {
            score.0 += grid.resolve();
        }
    }
}

fn handle_restart(
    keys: Res<ButtonInput<KeyCode>>,
    mut grid: ResMut<Grid>,
    mut cursor: ResMut<Cursor>,
    mut rise_timer: ResMut<RiseTimer>,
    mut gravity_timer: ResMut<GravityTimer>,
    mut game_over: ResMut<GameOver>,
    mut score: ResMut<Score>,
    mut elapsed: ResMut<ElapsedTime>,
) {
    if !game_over.0 {
        return;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        grid.clear();
        grid.fill_test_pattern();
        *cursor = Cursor::new(0, 0);
        rise_timer.0.reset();
        gravity_timer.0.reset();
        score.0 = 0;
        elapsed.0 = 0.0;
        game_over.0 = false;
    }
}

fn rise_stack(
    time: Res<Time>,
    mut timer: ResMut<RiseTimer>,
    mut grid: ResMut<Grid>,
    mut game_over: ResMut<GameOver>,
    mut score: ResMut<Score>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        if grid.top_row_occupied() {
            game_over.0 = true;
            return;
        }
        grid.push_bottom_row();
        score.0 += grid.resolve();
    }
}

fn update_time(time: Res<Time>, mut elapsed: ResMut<ElapsedTime>, game_over: Res<GameOver>) {
    if game_over.0 {
        return;
    }
    elapsed.0 += time.delta_seconds();
}

fn apply_gravity_system(
    time: Res<Time>,
    mut timer: ResMut<GravityTimer>,
    mut grid: ResMut<Grid>,
    game_over: Res<GameOver>,
) {
    if game_over.0 {
        return;
    }
    if timer.0.tick(time.delta()).just_finished() {
        grid.apply_gravity_step();
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

fn spawn_frame_and_panel(commands: &mut Commands) {
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

    let panel_x = half_w + FRAME_THICKNESS + PANEL_GAP + PANEL_WIDTH / 2.0;
    let panel_size = Vec2::new(PANEL_WIDTH, grid_h + FRAME_THICKNESS * 2.0);
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::srgb(0.07, 0.07, 0.09),
            custom_size: Some(panel_size),
            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(panel_x, 0.0, -0.6)),
        ..Default::default()
    });

    let header_size = Vec2::new(PANEL_WIDTH - 12.0, 28.0);
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::srgb(0.12, 0.12, 0.16),
            custom_size: Some(header_size),
            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(
            panel_x,
            half_h - 20.0,
            -0.5,
        )),
        ..Default::default()
    });
}

fn spawn_ui_texts(commands: &mut Commands) -> UiTexts {
    let style = TextStyle {
        font: Default::default(),
        font_size: 20.0,
        color: Color::srgb(0.9, 0.9, 0.95),
    };

    let score = commands
        .spawn(TextBundle {
            text: Text::from_section("Score: 0", style.clone()),
            style: Style {
                position_type: PositionType::Absolute,
                right: Val::Px(24.0),
                top: Val::Px(40.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .id();

    let timer = commands
        .spawn(TextBundle {
            text: Text::from_section("Time: 0.0s", style),
            style: Style {
                position_type: PositionType::Absolute,
                right: Val::Px(24.0),
                top: Val::Px(70.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .id();

    let game_over = commands
        .spawn(TextBundle {
            text: Text::from_section("GAME OVER - Press R", TextStyle {
                font: Default::default(),
                font_size: 36.0,
                color: Color::srgb(0.95, 0.2, 0.2),
            }),
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .id();

    UiTexts { score, timer, game_over }
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
