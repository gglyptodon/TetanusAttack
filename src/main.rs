use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;

mod game;
use game::{BlockColor, Cursor, Grid, SwapCmd};

const GRID_W: usize = 6;
const GRID_H: usize = 12;
const CELL_SIZE: f32 = 32.0;

#[derive(Resource)]
struct BlockSprites {
    entities: Vec<Entity>,
}

#[derive(Resource)]
struct CursorSprite(Entity);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Grid::new(GRID_W, GRID_H))
        .insert_resource(Cursor::new(0, 0))
        .add_systems(Startup, setup)
        .add_systems(Update, handle_input)
        .add_systems(Update, update_visuals)
        .run();
}

fn setup(mut commands: Commands, mut grid: ResMut<Grid>) {
    commands.spawn(Camera2dBundle::default());
    grid.fill_test_pattern();
    let sprites = spawn_grid(&mut commands, &grid);
    let cursor_sprite = spawn_cursor(&mut commands);
    commands.insert_resource(BlockSprites { entities: sprites });
    commands.insert_resource(CursorSprite(cursor_sprite));
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut grid: ResMut<Grid>,
    mut cursor: ResMut<Cursor>,
) {
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
            grid.resolve();
        }
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
