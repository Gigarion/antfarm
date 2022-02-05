// fn position_translation(windows: Res<Windows>,
//     mut q: Query<
//         (&Position, &mut Transform),
//         Changed<Position>
//     >,
// ) {
//     fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
//         let tile_size = bound_window / bound_game;
//         pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
//     }
//     let window = windows.get_primary().unwrap();
//     for (pos, mut transform) in q.iter_mut() {
//         transform.translation = Vec3::new(
//             convert(pos.x as f32, window.width() as f32, ARENA_WIDTH as f32),
//             convert(pos.y as f32, window.height() as f32, ARENA_HEIGHT as f32),
//             transform.translation.z,
//         );
//     }
// }




// #[derive(Component)]
// struct Ant;

// #[derive(Component)]
// struct Food;

// #[derive(Default)]
// struct VisitedPositions(Vec<Position>);

// const SNAKE_HEAD_COLOR: Color = Color::rgb(0.7, 0.7, 0.7);
// const SNAKE_SEGMENT_COLOR: Color = Color::rgb(0.3, 0.3, 0.3);
// const FOOD_COLOR: Color = Color::rgb(1.0, 0.0, 1.0);


// #[derive(Component, Clone, Copy, PartialEq, Eq)]
// struct Position {
//     x: i32,
//     y: i32,
// }


// #[derive(SystemLabel, Debug, Hash, PartialEq, Eq, Clone)]
// enum MovePhase {
//     Movement,
//     MoveTrigger,
//     Events,
// }
// TODO: Add check-for-death, death to systems properly labeled



// fn rotate_ant(time: Res<Time>, mut q: Query<&mut Transform, With<Ant>>) {
//     let delta = time.delta_seconds();

//     for mut t in q.iter_mut() {
//         t.rotate(Quat::from_rotation_z(1.0 * delta));
//     }
// }

// fn food_spawner(mut commands: Commands, positions: Query<&Position>) {

//     let used_positions: Vec<Position> = positions.iter().map(|p| *p).collect();
//     let pos = loop {
//         let pos = Position {
//             x: (random::<f32>() * ARENA_WIDTH as f32) as i32,
//             y: (random::<f32>() * ARENA_HEIGHT as f32) as i32,
//         };
//         if !used_positions.contains(&pos) {
//             break pos;
//         }
//     };

//     commands.spawn_bundle(
//         SpriteBundle {
//             sprite: Sprite {
//                 color: FOOD_COLOR,
//                 ..Default::default()
//             },
//             ..Default::default()
//         })
//         .insert(Food)
//         .insert(pos)
//         .insert(Size::square(0.8))
//         .insert(Layer::Main1);
// }


// #[derive(SystemLabel, Debug, Hash, PartialEq, Eq, Clone)]
// pub enum LocationProcessor {
//     Position,
//     Layer,
// }

// // TODO: have ant wander randomly
// fn ant_movement(
//     mut q: QuerySet<(
//         QueryState<&mut Position, With<Ant>>, // ant positions
//         QueryState<&Position, With<Collides>>, // possible colliders
//     )>,
// ) {

//     let occupied: Vec<Position> = q.q1().iter().map(|p| *p).collect();

//     for mut pos in q.q0().iter_mut() {
//         let possibles: Vec<Position> = vec!(
//             Position {x: pos.x + 1, ..*pos},
//             Position {x: pos.x - 1, ..*pos},
//             Position {y: pos.y + 1, ..*pos},
//             Position {y: pos.y - 1, ..*pos},
//             Position {x: pos.x + 1, y: pos.y - 1},
//             Position {x: pos.x - 1, y: pos.y - 1},
//             Position {x: pos.x + 1, y: pos.y + 1},
//             Position {x: pos.x - 1, y: pos.y + 1},
//         );

//         let possibles: Vec<Position> = possibles.into_iter()
//             .filter(|p| !occupied.contains(&p)) // Don't include collisions
//             .filter(|p| p.x > 0 && p.x < ARENA_WIDTH as i32 && p.y > 0 && p.y < ARENA_HEIGHT as i32) // Don't go OOB
//             .collect();

//         if possibles.len() > 0 {
//             let dir = random::<usize>() % possibles.len(); // number 0 -> 4
//             pos.x = possibles[dir].x;
//             pos.y = possibles[dir].y;
//         } else {
//             println!("Help me Step Bro");
//         }

//     }
// }

// struct AntEatsFood(Entity);

// fn ant_food_collide(
//     ant_positions: Query<&Position, With<Ant>>,
//     food_positions: Query<(&Position, Entity), With<Food>>,
//     mut eat_writer: EventWriter<AntEatsFood>,
// ) {
//     for (pos, food_ent) in food_positions.iter() {
//         eat_writer.send_batch(
//             ant_positions
//             .iter()
//             .filter_map(|p| {
//                 if p == pos {
//                     Some(AntEatsFood(food_ent))
//                 } else {
//                     None
//                 }
//             })
//         );
//     }
// }

// fn read_eat_food(
//     mut commands: Commands,
//     mut eat_reader: EventReader<AntEatsFood>,
// ) {
//     if let Some(eat_event) = eat_reader.iter().next() {
//         commands.entity(eat_event.0).despawn();
//         println!("Ate a food, bye");
//     }
// }

// struct AntDeathEvent;

// fn check_ant_death(
//     mut commands: Commands,
//     mut ant_positions: Query<(&Position, Entity), With<Ant>>,
//     mut death_writer: EventWriter<AntDeathEvent>,
// ) {
//     for (pos, ant_ent) in ant_positions.iter_mut() {
//         if pos.x < 0
//             || pos.x >= ARENA_WIDTH as i32
//             || pos.y < 0
//             || pos.y >= ARENA_HEIGHT as i32
//         {
//             commands.entity(ant_ent).despawn();
//             println!("Caleb it's gone");
//             death_writer.send(AntDeathEvent);
//         }
//     }
// }

// fn spawn_ant_reader(
//     commands: Commands,
//     mut death_events: EventReader<AntDeathEvent>,
// ) {
//     if death_events.iter().next().is_some() {
//         spawn_ant(commands);
//     }
// }