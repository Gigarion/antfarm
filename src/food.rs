use bevy::prelude::*;
use crate::arena::*;
use crate::arena::Size;
use crate::ant::KnownFood;
use rand::prelude::random;

#[derive(Debug)]
pub struct FoodCreateEvent {
    pub x: f32,
    pub y: f32,
    pub quantity: f32,
}

pub struct FoodPlugin;
impl Plugin for FoodPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_startup_system(food_spawner)
            .add_system(food_coloration)
            .add_system(food_create_handler)
            .add_event::<FoodCreateEvent>();
    }
}

#[derive(Component)]
pub struct Food {
    pub quantity: f32,
}

pub fn food_spawner(
    mut commands: Commands,
    food_count: Query<&Food>,
) {
    let mut current_food = food_count.iter().count();
    while current_food < 40 {
        let x: f32 = random::<f32>() * ((ARENA_WIDTH_TILES - 4) as f32 * ARENA_TILE_SIDE) + (2. * ARENA_TILE_SIDE);
        let y: f32 = random::<f32>() * ((ARENA_HEIGHT_TILES - 4) as f32 * ARENA_TILE_SIDE) + (2. * ARENA_TILE_SIDE);

        commands.spawn_bundle(FoodBundle::new(x, y, 3.));
        current_food += 1;
    }
}

pub fn food_create_handler(
    mut commands: Commands,
    mut food_reader: EventReader<FoodCreateEvent>
) {
    for food_creation in food_reader.iter() {
        let bundle: FoodBundle = food_creation.into();
        commands.spawn_bundle(bundle);
    }
}


const KNOWN_FOOD_COLOR: Color = Color::ORANGE;
fn food_coloration(
    known_food: Res<KnownFood>,
    mut food_sprites: Query<&mut Sprite, With<Food>>,
) {
    for food_ent in known_food.locs.iter() {
        if let Ok(mut sprite) = food_sprites.get_mut(*food_ent) {
            sprite.color = KNOWN_FOOD_COLOR;
        }
    }
}

#[derive(Bundle)]
struct FoodBundle {
    #[bundle]
    sprite: SpriteBundle,

    food: Food,
    position: Position,
    layer: Layer,
    size: Size,
}

impl Default for FoodBundle {
    fn default() -> Self {
        FoodBundle {
            sprite: SpriteBundle {
                sprite: Sprite {
                    color: Color::PURPLE,
                    ..Default::default()
                },
                ..Default::default()
            },
            food: Food {quantity: 3.},
            position: Position { x: 500., y: 500.},
            layer: Layer::Main1,
            size: crate::arena::Size::square(0.3)
        }
    }
}

impl FoodBundle {
    fn new(x: f32, y: f32, quantity: f32) -> FoodBundle {
        FoodBundle {
            position: Position {x, y},
            food: Food {quantity},
            ..FoodBundle::default()
        }
    }
}

impl From<&FoodCreateEvent> for FoodBundle {
    fn from(event: &FoodCreateEvent) -> Self {
        FoodBundle {
            position: Position {x: event.x, y: event.y},
            food: Food {quantity: event.quantity},
            ..FoodBundle::default()
        }
    }
}