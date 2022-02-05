use crate::arena::*;
use crate::food::{Food, FoodCreateEvent};
use crate::arena::Size;
use bevy::prelude::*;
use rand::{prelude::Distribution, distributions::WeightedIndex, thread_rng};
use rand::Rng;

const ANT_COLOR: Color = Color::rgb(0.7, 0.7, 0.7);
const ANT_SPEED: f32 = 50.;
const QUEEN_SPEED: f32 = 20.;

#[derive(Default)]
pub struct KnownFood {
    pub locs: Vec<Entity>,
}

pub struct AntDeathEvent {
    ent: Entity,
}

#[derive(SystemLabel, Debug, Hash, PartialEq, Eq, Clone)]
enum AntPhase {
    CleanFood,
    FindFood,
    FoodGoal,
}

#[derive(SystemLabel, Debug, Hash, PartialEq, Eq, Clone)]
enum BigPhase {
    Decide,
    Move,
    Act,
    Ambient,
    Cleanup,
}

pub struct AntPlugin;
impl Plugin for AntPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(KnownFood::default())
            .add_event::<AntDeathEvent>()
            .add_startup_system(spawn_ant)
            .add_system_set(
                SystemSet::new()
                    .label(BigPhase::Decide)
                    .with_system(clean_food
                        .label(AntPhase::CleanFood)
                    )
                    .with_system(locate_food
                        .label(AntPhase::FindFood)
                        .after(AntPhase::CleanFood)
                    )
                    .with_system(add_food_goal
                        .label(AntPhase::FoodGoal)
                        .after(AntPhase::FindFood)
                    )
            )
            .add_system_set(
                SystemSet::new()
                    .label(BigPhase::Move)
                    .after(BigPhase::Decide)
                    .with_system(ant_movement)
                    .with_system(eat_food)
            )
            .add_system_set(
                SystemSet::new()
                    .label(BigPhase::Act)
                    .after(BigPhase::Move)
                    .with_system(start_eat_food)
                    .with_system(ant_begin_ai)
            )
            .add_system_set(
                SystemSet::new()
                    .label(BigPhase::Ambient)
                    .after(BigPhase::Act)
                    .with_system(health_degrade)
                    .with_system(hunger_degrade)
            )
            .add_system_set(
                SystemSet::new()
                    .label(BigPhase::Cleanup)
                    .after(BigPhase::Ambient)
                    .with_system(ant_coloration)
                    .with_system(ant_death_handler)
            );
    }
}

fn clean_food(
    mut known_foods: ResMut<KnownFood>,
    all_food: Query<Entity, With<Food>>,
) {
    let mut af = std::collections::HashSet::<Entity>::new();
    all_food.iter().for_each(|e| { af.insert(e); });
    known_foods.locs = known_foods.locs.iter_mut()
        .filter(|e| af.contains(*e))
        .map(|e| *e)
        .collect();
}

fn ant_begin_ai(
    mut commands: Commands,
    unassigned_ants: Query<Entity, (With<Ant>, Without<AntEating>, Without<AntMoving>)>,
) {
    for ant in unassigned_ants.iter() {
        commands.entity(ant).insert(AntMoving::default());
    }
}

fn pick_move_ai() -> MoveAI {
    let mut rng = thread_rng();
    match rng.gen_range(0, 8) {
        0 => MoveAI::North,
        1 => MoveAI::South,
        2 => MoveAI::East,
        3 => MoveAI::West,
        4 => MoveAI::NE,
        5 => MoveAI::SE,
        6 => MoveAI::SW,
        7 => MoveAI::NW,
        _ => MoveAI::Random,
    }
}

fn ant_movement(
    time: Res<Time>,
    mut q: QuerySet<(
        QueryState<(&Position, &Size), (With<Ant>, With<AntMoving>)>, // ant positions for filtering colliders
        QueryState<(&Position, &Size), With<Collides>>, // possible colliders
        QueryState<(&mut Position, &Size, Option<&Destination>, &mut AntMoving, Option<&Queen>), With<Ant>>, // ant positions for moving the ants
    )>,
) {

    let dt = time.delta_seconds();
    let d_r = dt * ANT_SPEED;

    let ant_start: Vec<(Position, Size)> = q.q0().iter()
        .map(|(p, s)| (*p, *s)).collect();

    let occupied: Vec<(Position, Size)> = q.q1().iter()
        .filter(|(p_obj, s_obj)| {
            ant_start.iter()
                .any(|(p_ant, s_ant)| {
                    dist_between(p_ant, s_ant, p_obj, s_obj) < d_r * 2.
                })
        })
        .map(|(p, s)| (*p, *s))
        .collect();

    for (mut pos, size, opt_dest, mut ai, opt_queen) in q.q2().iter_mut() {

        let d_r = if opt_queen.is_some() {
            dt * QUEEN_SPEED
        } else {
            d_r
        };

        // We have a destination, let's go there
        if let Some(dest) = opt_dest {
            let start =  Vec2::from((pos.x, pos.y));
            let target = Vec2::from((dest.location.x, dest.location.y));
            let lerp_frac = d_r / start.distance(target);
            let outcome = start.lerp(target, lerp_frac);

            // if no collision take it
            if !occupied.iter().any(|(p_1, s_1)| collides(&pos, size, p_1, s_1)) {
                pos.x = outcome.x;
                pos.y = outcome.y;
                continue;
            }
            // else fall back to random walk
        }

        if ai.duration > 0. {
            ai.duration -= dt;
        }

        if ai.ai.is_none() || ai.duration < 0. {
            ai.duration = 5.0;
            ai.ai = Some(pick_move_ai());
        }

        // We don't have a destination, random walk
        let possibles = vec!(
            Position {y: pos.y + d_r, ..*pos},  // N
            Position {x: pos.x + d_r, ..*pos},  // E
            Position {y: pos.y - d_r, ..*pos},  // S
            Position {x: pos.x - d_r, ..*pos},  // W
            Position {x: pos.x + d_r, y: pos.y + d_r},  // NE
            Position {x: pos.x + d_r, y: pos.y - d_r},  // SE
            Position {x: pos.x - d_r, y: pos.y - d_r},  // SW
            Position {x: pos.x - d_r, y: pos.y + d_r}   // NW
        );

        let weights = generate_move_weights(ai.ai.unwrap());

        let max_width = ARENA_TILE_SIDE * ARENA_WIDTH_TILES as f32;
        let max_height = ARENA_TILE_SIDE * ARENA_HEIGHT_TILES as f32;

        let possibles: Vec<(Position, i32)> = possibles.into_iter()
            .zip(weights.into_iter())
            .filter(|(p, _)| p.x > 0. && p.x < max_width && p.y > 0. && p.y < max_height) // Don't go OOB
            .filter(|(p, _)| {
                !occupied.iter().any(|(p_1, s_1)| collides(p, size, p_1, s_1))
            })
            .collect();

        let outcome_vec: Vec<Position> = possibles.iter().map(|(p, _)| *p).collect();
        if outcome_vec.len() > 0 {
            let weight_vec: Vec<i32> = possibles.iter().map(|(_, w)| *w).collect();
            let dist = WeightedIndex::new(weight_vec).unwrap();
            // let dir = 0; // number 0 -> 4
            let mut rng = thread_rng();
            let newpos  = outcome_vec[dist.sample(&mut rng)];
            pos.x = newpos.x;
            pos.y = newpos.y;
        } else {
            println!("Help me Step Bro");
        }
    }
}

fn generate_move_weights(ai: MoveAI) -> Vec<i32> {
    match ai {
        MoveAI::North =>  vec![16, 4, 1, 4, 8, 1, 1, 8],
        MoveAI::East =>   vec![4, 16, 4, 1, 8, 8, 1, 1],
        MoveAI::South =>  vec![1, 4, 16, 4, 1, 8, 8, 1],
        MoveAI::West =>   vec![4, 1, 4, 16, 1, 1, 8, 8],
        MoveAI::NE =>     vec![4, 4, 1, 1, 16, 1, 1, 1],
        MoveAI::SE =>     vec![1, 4, 4, 1, 1, 16, 1, 1],
        MoveAI::SW =>     vec![1, 1, 4, 4, 1, 1, 16, 1],
        MoveAI::NW =>     vec![4, 1, 1, 4, 1, 1, 1, 16],
        MoveAI::Random => vec![1, 1, 1, 1, 1, 1, 1, 1],
    }
}

const ANT_DEATH_COLOR: Color = Color::RED;
fn ant_coloration(
    mut ant_sprites: Query<(&mut Sprite, &Health), With<Ant>>,
) {
    for (mut sprite, health) in ant_sprites.iter_mut() {
        let base = Vec3::from((ANT_COLOR.r(), ANT_COLOR.g(), ANT_COLOR.b()));
        let death = Vec3::from((ANT_DEATH_COLOR.r(), ANT_DEATH_COLOR.g(), ANT_DEATH_COLOR.b()));
        let new = base.lerp(death, 1.0 - health.pct);
        sprite.color = Color::rgb(new.x, new.y, new.z);
    }
}

fn locate_food(
    mut known_food: ResMut<KnownFood>,
    mut q: QuerySet<(
        QueryState<(&Position, &Size, Entity), With<Food>>,
        QueryState<(&Position, &VisibleRange),  With<Ant>>,
    )>
) {
    let unseen_food: Vec<(Position, Size, Entity)> = q.q0()
        .iter_mut()
        .filter(|(_, _, e)| {
            !known_food.locs.contains(e)
        })
        .map(|(p, s, e)| {
            (*p, *s, e)
        })
        .collect();

    for (ant_p, ant_v) in q.q1().iter() {
        for (food_p, food_s, ent) in unseen_food.iter() {
            if collides(ant_p, &ant_v.size, food_p, food_s) {
                known_food.locs.push(*ent);
                println!("Added food at {:?}", food_p);
            }
        }
    }
}

const HEALTH_DEGRADATION_RATE: f32 = 0.1;
fn health_degrade(
    time: Res<Time>,
    mut death_writer: EventWriter<AntDeathEvent>,
    mut hitpoints: Query<(&mut Health, &Hunger, Entity)>,
) {
    let dt = time.delta_seconds();
    for (mut health, hunger, ent) in hitpoints.iter_mut() {
        if hunger.pct < 0.01 {
            health.pct -= dt * HEALTH_DEGRADATION_RATE;
        }
        if health.pct < 0. {
            death_writer.send(AntDeathEvent{ent});
        }
    }
}

fn ant_death_handler(
    mut commands: Commands,
    mut deaths: EventReader<AntDeathEvent>,
    locations: Query<&Position, With<Ant>>,
    mut food_spawner: EventWriter<FoodCreateEvent>,
) {
    for death in deaths.iter() {
        if let Ok(p) = locations.get(death.ent){
            food_spawner.send(FoodCreateEvent{
                x: p.x,
                y: p.y,
                quantity: 1.0,
            });
            commands.entity(death.ent).despawn();
        }
    }
}


const HUNGER_DEGRADATION_RATE: f32 = 0.025;
fn hunger_degrade(
    time: Res<Time>,
    mut hitpoints: Query<&mut Hunger, Without<AntEating>>,
) {
    let dt = time.delta_seconds();
    for mut hunger in hitpoints.iter_mut() {
        hunger.pct -= dt * HUNGER_DEGRADATION_RATE;
        if hunger.pct < 0. {
            hunger.pct = 0.;
        }
    }
}


#[derive(Component)]
struct Ant;

fn spawn_ant(
    mut commands: Commands,
    ants: Query<&Ant>,
) {
    let mut current_ants = ants.iter().count();
    while current_ants < 30 {
        commands.spawn_bundle(AntBundle::default());
        current_ants += 1;
    }

    commands.spawn_bundle(QueenBundle::default());
}

fn start_eat_food(
    mut commands: Commands,
    known_food: Res<KnownFood>,
    food_pos: Query<&Position, With<Food>>,
    ants: Query<(Entity, &Hunger, &Position, &Size, &Destination), (With<Ant>, With<AntMoving>)>,
) {
    let available_food: Vec<(Position, Entity)> = known_food.locs.iter()
        .filter_map(|e| {
            match food_pos.get(*e) {
                Ok(v) => Some((*v, *e)),
                Err(_) => None,
            }
        })
        .collect();

    for (e, h, p, s, dest) in ants.iter() {
        for (food_p, food_ent) in available_food.iter() {
            if h.pct < 0.5 && dist_between(p, s, food_p, &crate::arena::Size::square(0.5)) < 0.6 {
                commands.entity(e).insert(AntEating{food_ent: *food_ent});
                commands.entity(e).remove::<AntMoving>();
                break;
            }
        }
        if dist_between(p, s, &dest.location,&crate::arena::Size::square(0.5)) < 0.6 {
            commands.entity(e).remove::<Destination>();
        }
    }
}

const EAT_RATE: f32 = 0.25;
fn eat_food(
    mut commands: Commands,
    time: Res<Time>,
    mut eating_ants: Query<(&mut Hunger, &AntEating, Entity), With<Ant>>,
    mut food: Query<&mut Food>,
) {
    let dt = time.delta_seconds();
    for (mut h, eating, e) in eating_ants.iter_mut() {
        if let Ok(mut food) = food.get_mut(eating.food_ent) {
            if h.pct < 1.0 && food.quantity > 0. {
                h.pct += dt * EAT_RATE;
                food.quantity -= dt * EAT_RATE;
            }

            if food.quantity <= 0. {
                commands.entity(eating.food_ent).despawn();
                commands.entity(e).insert(AntMoving::default());
                commands.entity(e).remove::<AntEating>();
                commands.entity(e).remove::<Destination>();
                println!("Food gone");
            }

            if h.pct > 1.0 {
                h.pct = 1.0;
                commands.entity(e).insert(AntMoving::default());
                commands.entity(e).remove::<AntEating>();
                commands.entity(e).remove::<Destination>();
                println!("Done eating");
            }
        } else {
            commands.entity(e).remove::<AntEating>();
            commands.entity(e).remove::<Destination>();
            commands.entity(e).insert(AntMoving::default());
        }
    }
}

// Once ant hunger drops to a certain point, give them a destination
fn add_food_goal(
    mut commands: Commands,
    known_food: Res<KnownFood>,
    food_pos: Query<&Position, With<Food>>,
    ants: Query<(Entity, &Position, &Size, &Hunger), (With<Ant>, Without<Destination>)>,
) {
    for (e, ant_pos, ant_size,  hunger) in ants.iter() {
        if hunger.pct < 0.2 && !known_food.locs.is_empty() {
            let mut max_dist: f32 = 1000000.;
            let mut best = known_food.locs[0];
            for food in known_food.locs.iter() {
                if let Ok(p) = food_pos.get(*food) {
                    let new_dist = dist_between(ant_pos, ant_size, p, &crate::arena::Size::square(0.5));
                    if new_dist < max_dist {
                        max_dist = new_dist;
                        best = *food;
                    }
                }
            }

            // Select destination food:
            if let Ok(food_place) = food_pos.get(best) {
                commands.entity(e).insert(Destination {
                    location: *food_place,
                });
            }
        }
    }
}

#[derive(Bundle)]
struct AntBundle {
    #[bundle]
    sprite: SpriteBundle,

    ant: Ant,
    position: Position,
    health: Health,
    hunger: Hunger,
    layer: Layer,
    size: Size,
    visibility: VisibleRange,
}

impl Default for AntBundle {
    fn default() -> Self {
        AntBundle {
            sprite: SpriteBundle {
                sprite: Sprite {
                    color: ANT_COLOR,
                    ..Default::default()
                },
                ..Default::default()
            },
            ant: Ant,
            position: Position { x: 200., y: 200.},
            health: Health::full(),
            hunger: Hunger::full(),
            layer: Layer::Main2,
            size: Size::square(0.6),
            visibility: VisibleRange::new(5.0),
        }
    }
}

#[derive(Component)]
struct Queen;

#[derive(Bundle)]
struct QueenBundle {
    #[bundle]
    sprite: SpriteBundle,

    queen: Queen,
    ant: Ant,
    position: Position,
    health: Health,
    hunger: Hunger,
    layer: Layer,
    size: Size,
    visibility: VisibleRange,
}

impl Default for QueenBundle {
    fn default() -> Self {
        QueenBundle {
            sprite: SpriteBundle {
                sprite: Sprite {
                    color: ANT_COLOR,
                    ..Default::default()
                },
                ..Default::default()
            },
            queen: Queen,
            ant: Ant,
            position: Position { x: 200., y: 200.},
            health: Health::full(),
            hunger: Hunger::full(),
            layer: Layer::Main2,
            size: Size::square(2.0),
            visibility: VisibleRange::new(5.0),
        }
    }
}

impl AntBundle {
    fn new(x: f32, y: f32) -> Self {
        AntBundle {
            position: Position {x, y},
            ..AntBundle::default()
        }
    }
}

#[derive(Component)]
struct Health {
    pct: f32,
}

impl Health {
    fn full() -> Health {
        Health { pct: 1.0 }
    }
}

#[derive(Component)]
struct Hunger {
    pct: f32,
}

impl Hunger {
    fn full() -> Self {
        Hunger {
            pct: 1.0
        }
    }
}

// Can see for this radius
#[derive(Component, Clone, Copy)]
pub struct VisibleRange {
    pub size: Size,
}

impl VisibleRange {
    pub fn new(radius: f32) -> Self {
        VisibleRange {
            size: Size::square(radius),
        }
    }
}

#[derive(Component)]
struct Destination {
    pub location: Position,
}

#[derive(Component, PartialEq, PartialOrd)]
struct AntEating {
    food_ent: Entity,
}

#[derive(Clone, Copy)]
enum MoveAI {
    North,
    South,
    East,
    West,
    NE,
    SE,
    SW,
    NW,
    Random,
}

#[derive(Component, Default)]
struct AntMoving {
    ai: Option<MoveAI>,
    duration: f32,
}