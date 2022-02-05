use bevy::prelude::*;
use crate::arena::*;

pub struct WallPlugin;

impl Plugin for WallPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_startup_system(startup_spawn_tiles.after(StartupLabels::Screen));
    }
}

fn startup_spawn_tiles(mut commands: Commands) {
    for row in [0, ARENA_HEIGHT_TILES-1].into_iter() {
        for col in 0..ARENA_WIDTH_TILES {
            spawn_tile(&mut commands, ARENA_TILE_SIDE * col as f32, ARENA_TILE_SIDE * row as f32);
        }
    }
    for col in [0, ARENA_WIDTH_TILES-1].into_iter() {
        for row in 0..ARENA_HEIGHT_TILES {
            spawn_tile(&mut commands, ARENA_TILE_SIDE * col as f32, ARENA_TILE_SIDE * row as f32);
        }
    }
}

fn spawn_tile(commands: &mut Commands, x: f32, y: f32) {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.9, 0.9, 0.9),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Position{ x, y })
        .insert(crate::arena::Size::square(0.95))
        .insert(Layer::Main1)
        .insert(Collides);
}