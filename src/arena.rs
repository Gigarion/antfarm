use bevy::prelude::*;


#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Tile,
    Grid,
    Main1,
    Main2,
    Sky,
}

// Arena coordinates
#[derive(Component, Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

// edges are a proportion of one arena tile side

#[derive(Component, Clone, Copy, Debug)]
pub struct Size {
    width: f32,
    height: f32,
}

impl Size {
    pub fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x,
        }
    }

    pub fn radius(&self) -> f32 {
        return self.width * ARENA_TILE_SIDE / 2. * std::f32::consts::SQRT_2;
    }
}

pub fn collides(p1: &Position, s1: &Size, p2: &Position, s2: &Size) -> bool {
    let sum_radius = s1.radius() + s2.radius();
    Vec2::from((p1.x, p1.y)).distance(Vec2::from((p2.x, p2.y))) < sum_radius
}

pub fn dist_between(p1: &Position, s1: &Size, p2: &Position, s2: &Size) -> f32 {
    let v1 = Vec2::from((p1.x, p1.y));
    let v2 = Vec2::from((p2.x, p2.y));
    v1.distance(v2) - s1.radius() - s2.radius()
}

// // TODO: CollisionGroups?
#[derive(Component)]
pub struct Collides;


#[derive(Default)]
pub struct ArenaStats {
    // window mechanics.  How big overall, how much is buffered away from arena
    window_width: f32,
    window_height: f32,

    arena_width: f32,
    arena_height: f32,
}

pub const ARENA_WIDTH_TILES : u32 = 200;
pub const ARENA_HEIGHT_TILES : u32 = 100;
pub const ARENA_TILE_SIDE : f32 = 8.;

fn update_window_stats(
    windows: Res<Windows>,
    mut screen_builder: ResMut<ArenaStats>
) {
    let window = windows.get_primary().unwrap();
    if window.height() != screen_builder.window_height || window.width() != screen_builder.window_width {
        screen_builder.window_height = window.height();
        screen_builder.window_width = window.width();

        let reserved_width = ARENA_WIDTH_TILES as f32 * ARENA_TILE_SIDE;
        let reserved_height = ARENA_HEIGHT_TILES as f32 * ARENA_TILE_SIDE;

        // let side_buffers = (screen_builder.window_width - reserved_width) / 2.;
        // let vertical_buffers = (screen_builder.window_height - reserved_height) / 2.;

        screen_builder.arena_width = reserved_width;
        screen_builder.arena_height = reserved_height;
    }
}

fn layer_to_z(layer: Layer) -> f32 {
    match layer {
        Layer::Tile => 0.01,
        Layer::Grid => 0.02,
        Layer::Main1 => 0.1,
        Layer::Main2 => 0.2,
        Layer::Sky => 1.0,
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn size_scaling(
    mut q: Query<
        (&Size, &mut Transform),
        Changed<Size>,
    >
) {
    for (sprite_size, mut transform) in q.iter_mut() {
        transform.scale = Vec3::new(
            sprite_size.width * ARENA_TILE_SIDE,
            sprite_size.height * ARENA_TILE_SIDE,
            transform.scale.z,
        );
    }
}

fn layer_fixer(
    mut q: Query<
        (&mut Transform, &Layer),
        Changed<Transform>,
    >
) {
    for (mut t, l) in q.iter_mut() {
        t.translation.z = layer_to_z(*l);
    }
}

#[derive(SystemLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub enum StartupLabels {
    Screen,
}

pub struct ArenaPlugin;

impl Plugin for ArenaPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(WindowDescriptor {
                title: "Ant Farm".to_string(),
                width: 850.,
                height: 850.,
                ..Default::default()
            })
            .insert_resource(ArenaStats::default())
            .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
            .add_startup_system(update_window_stats.label(StartupLabels::Screen))
            .add_startup_system(startup_spawn_arena.after(StartupLabels::Screen))
            .add_startup_system(setup_camera)
            .add_system(size_scaling)
            .add_system(position_translation)
            .add_system(layer_fixer)
            .add_system(update_window_stats);
    }
}

fn startup_spawn_arena(
    mut commands: Commands,
    screen_builder: Res<ArenaStats>
) {
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            color: Color::rgba(0.3, 0.7, 0.5, 0.1),
            ..Default::default()
        },
        ..Default::default()
    })
    .insert(Size::square(ARENA_WIDTH_TILES as f32))
    .insert(Position{x: (screen_builder.arena_width / 2.) - ARENA_TILE_SIDE / 2. , y: (screen_builder.arena_height / 2.) - ARENA_TILE_SIDE / 2.})
    .insert(Layer::Tile);

    for row in 0..ARENA_HEIGHT_TILES {
        for col in 0..ARENA_WIDTH_TILES {
            spawn_grid(&mut commands, ARENA_TILE_SIDE * col as f32, ARENA_TILE_SIDE * row as f32);
        }
    }
}

fn spawn_grid(commands: &mut Commands, x: f32, y: f32) {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(0.1, 0.1, 0.1, 0.3),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Position{ x, y })
        .insert(crate::arena::Size::square(0.95))
        .insert(Layer::Grid);
}

fn position_translation(screen: Res<ArenaStats>,
    mut q: Query<
        (&Position, &mut Transform),
        Changed<Position>,
    >,
) {
    for (pos, mut transform) in q.iter_mut() {
        transform.translation = Vec3::new(
            pos.x - (screen.arena_width / 2.),
            pos.y - (screen.arena_height / 2.),
            transform.translation.z,
        );
    }
}
