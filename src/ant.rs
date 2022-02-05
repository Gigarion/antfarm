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
    time: Res<Time>,
    mut commands: Commands,
    unassigned_ants: Query<Entity, (With<Ant>, Without<AntEating>, Without<AntAI>)>,
    mut ais: Query<&mut AntAI, With<Ant>>,
) {
    for ant in unassigned_ants.iter() {
        commands.entity(ant).insert(AntAI::default());
    }

    let dt = time.delta_seconds();
    for mut ai in ais.iter_mut() {

        // Handle special AI first
        match ai.ai {
            // Need to set
            AiGoal::None => {
                *ai = AntAI::random_move_ai();
            },
            // Remain until explicitly cleared
            AiGoal::Wait | AiGoal::Destination{..} => continue,
            _ => (),
        }

        // Now do duration
        if ai.duration > 0. {
            ai.duration -= dt;
        }
        if ai.duration <= 0. {
            *ai = AntAI::random_move_ai();
        }
    }
}


fn ant_movement(
    time: Res<Time>,
    mut q: QuerySet<(
        QueryState<(&Position, &Size), With<Ant>>, // ant positions for filtering colliders
        QueryState<(&Position, &Size), With<Collides>>, // possible colliders
        QueryState<(&mut Position, &Size, &mut AntAI, Option<&Queen>), With<Ant>>, // ant positions for moving the ants
    )>,
) {

    let dt = time.delta_seconds();
    let d_r = dt * ANT_SPEED;

    let ant_start: Vec<(Position, Size)> = q.q0().iter().map(|(p, s)| (*p, *s)).collect();

    // Trim occupied down to only things within 2 move spaces.
    let occupied: Vec<(Position, Size)> = q.q1().iter()
        .filter(|(p_obj, s_obj)| {
            ant_start.iter()
                .any(|(p_ant, s_ant)| {
                    dist_between(p_ant, s_ant, p_obj, s_obj) < d_r * 2.
                })
        })
        .map(|(p, s)| (*p, *s))
        .collect();

    for (mut pos, size, ai, opt_queen) in q.q2().iter_mut() {

        let d_r = if opt_queen.is_some() {
            dt * QUEEN_SPEED
        } else {
            d_r
        };

        match ai.ai {
            // We have a goal, go to it
            AiGoal::Destination{dest} => {
                let start =  Vec2::from((pos.x, pos.y));
                let target = Vec2::from((dest.x, dest.y));
                let lerp_frac = d_r / start.distance(target);
                let outcome = start.lerp(target, lerp_frac);

                // if no collision take it
                if !occupied.iter().any(|(p_1, s_1)| collides(&pos, size, p_1, s_1)) {
                    pos.x = outcome.x;
                    pos.y = outcome.y;
                    continue;
                }
            },

            // We need to stay here. Do so.
            AiGoal::None | AiGoal:: Wait => continue,

            // Everything else has a movement
            _ => {},
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

        let weights = generate_move_weights(ai.ai);

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
        // short-circuit to avoid the rng
        if outcome_vec.len() == 1 {
            pos.x = outcome_vec[0].x;
            pos.y = outcome_vec[0].y;
        } else if outcome_vec.len() > 0 {
            // AI weighted distribution
            let weight_vec: Vec<i32> = possibles.iter().map(|(_, w)| *w).collect();
            let dist = WeightedIndex::new(weight_vec).unwrap();

            let mut rng = thread_rng();
            let newpos  = outcome_vec[dist.sample(&mut rng)];
            pos.x = newpos.x;
            pos.y = newpos.y;
        } else {
            println!("Help me Step Bro");
        }
    }
}

fn generate_move_weights(ai: AiGoal) -> Vec<i32> {
    match ai {
        AiGoal::North =>  vec![16, 4, 1, 4, 8, 1, 1, 8],
        AiGoal::East =>   vec![4, 16, 4, 1, 8, 8, 1, 1],
        AiGoal::South =>  vec![1, 4, 16, 4, 1, 8, 8, 1],
        AiGoal::West =>   vec![4, 1, 4, 16, 1, 1, 8, 8],
        AiGoal::NE =>     vec![4, 4, 1, 1, 16, 1, 1, 1],
        AiGoal::SE =>     vec![1, 4, 4, 1, 1, 16, 1, 1],
        AiGoal::SW =>     vec![1, 1, 4, 4, 1, 1, 16, 1],
        AiGoal::NW =>     vec![4, 1, 1, 4, 1, 1, 1, 16],
        AiGoal::Random | AiGoal::Destination{..} => vec![1, 1, 1, 1, 1, 1, 1, 1],
        _ => panic!("Asked to select move weights for an unsupported goal {:?}", ai),
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
    mut ants: Query<(Entity, &Position, &Size, &mut AntAI), With<FindFood>>,
) {
    let mut available_food: Vec<(Position, Entity)> = known_food.locs.iter()
        .filter_map(|e| {
            match food_pos.get(*e) {
                Ok(v) => Some((*v, *e)),
                Err(_) => None,
            }
        })
        .collect();

    for (e, p, s, mut ai) in ants.iter_mut() {
        if let AiGoal::Destination{dest} = ai.ai {
            if let Some((food_pos, food_ent)) = available_food
                .iter_mut()
                .filter(|(f_p, _)| {
                    dest == *f_p
                }).next()
            {
                if dist_between(p, s, &food_pos, &crate::arena::Size::square(0.5)) < 0.6 {
                    println!("adding AntEating");
                    commands.entity(e).insert(AntEating{food_ent: *food_ent});
                    commands.entity(e).remove::<FindFood>();
                    ai.ai = AiGoal::Wait;
                    break;
                }
            } else {
                // This isn't the food you're looking for, try for another food goal
                *ai = AntAI::default();
                commands.entity(e).remove::<FindFood>();
            }
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
            // First food check to avoid double-despawning the food.  We've already
            // despawned it if we see negative before eating any ourselves.
            if food.quantity <= 0. {
                commands.entity(e).insert(AntAI::default());
                commands.entity(e).remove::<AntEating>();
                continue;
            }

            if h.pct < 1.0 && food.quantity > 0. {
                h.pct += dt * EAT_RATE;
                food.quantity -= dt * EAT_RATE;
            }

            if food.quantity <= 0. {
                commands.entity(eating.food_ent).despawn();
                commands.entity(e).insert(AntAI::default());
                commands.entity(e).remove::<AntEating>();
                println!("Food gone");
            }

            if h.pct > 1.0 {
                h.pct = 1.0;
                commands.entity(e).insert(AntAI::default());
                commands.entity(e).remove::<AntEating>();
                println!("Done eating");
            }
        } else {
            commands.entity(e).remove::<AntEating>();
            commands.entity(e).insert(AntAI::default());
        }
    }
}

// Once ant hunger drops to a certain point, give them a destination
fn add_food_goal(
    mut commands: Commands,
    known_food: Res<KnownFood>,
    food_pos: Query<&Position, With<Food>>,
    ants: Query<(Entity, &Position, &Size, &Hunger), (With<Ant>, Without<FindFood>, Without<AntEating>)>,
) {
    for (e, ant_pos, ant_size,  hunger) in ants.iter() {
        if hunger.pct < 0.22 && !known_food.locs.is_empty() {
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
                commands.entity(e).insert(FindFood);
                commands.entity(e).insert(AntAI {
                    ai: AiGoal::Destination {
                        dest: *food_place,
                    },
                    duration: 100000.,
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

#[derive(Component, Clone, Copy, PartialEq, PartialOrd)]
struct FindFood;

#[derive(Component, PartialEq, PartialOrd)]
struct AntEating {
    food_ent: Entity,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum AiGoal {
    North,
    South,
    East,
    West,
    NE,
    SE,
    SW,
    NW,
    Random,
    Destination {
        dest: Position,
    },
    Wait,
    None,
}

#[derive(Component)]
struct AntAI {
    ai: AiGoal,
    duration: f32,
}

impl Default for AntAI {
    fn default() -> Self {
        AntAI {
            ai: AiGoal::None,
            duration: 0.0,
        }
    }
}

impl AntAI {
    fn random_move_ai() -> AntAI {
        let mut rng = thread_rng();
        let ai = match rng.gen_range(0, 8) {
            0 => AiGoal::North,
            1 => AiGoal::South,
            2 => AiGoal::East,
            3 => AiGoal::West,
            4 => AiGoal::NE,
            5 => AiGoal::SE,
            6 => AiGoal::SW,
            7 => AiGoal::NW,
            _ => AiGoal::Random,
        };

        AntAI {
            ai,
            duration: 5.0,
        }
    }
}