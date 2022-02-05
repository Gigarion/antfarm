use bevy::prelude::*;
use crate::arena::*;
use crate::arena::Size;
use crate::ant::VisibleRange;


#[derive(SystemLabel, Debug, Hash, PartialEq, Eq, Clone)]
enum FogPhase {
    Detect,
}
pub struct FogOfWarPlugin;
impl Plugin for FogOfWarPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_startup_system(startup_spawn_fog)
            .add_system(find_visible.label(FogPhase::Detect))
            .add_system(fog_killer.after(FogPhase::Detect))
            .add_event::<FogDieEvent>();
    }
}

pub struct FogDieEvent {
    fogs: Vec<Entity>,
}

#[derive(Component)]
struct Fog;

fn startup_spawn_fog(
    mut commands: Commands,
) {
    for row in 0..ARENA_HEIGHT_TILES {
        for col in 0..ARENA_WIDTH_TILES {
            spawn_fog(&mut commands, ARENA_TILE_SIDE * col as f32, ARENA_TILE_SIDE * row as f32);
        }
    }
}

fn spawn_fog(commands: &mut Commands, x: f32, y: f32) {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.4, 0.4, 0.4),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Position{ x, y })
        .insert(crate::arena::Size::square(0.99))
        .insert(Layer::Sky)
        .insert(Fog);
}

fn find_visible(
    mut fog_death_writer: EventWriter<FogDieEvent>,
    fogs: Query<(&Position, &Size, Entity), With<Fog>>,
    lookers: Query<(&Position, &VisibleRange)>,
) {

    let fogs: Vec<Entity> = fogs.iter().filter(|(fog_p, fog_s, _)| {
        lookers.iter().any(|(l_p, l_v)| {
            collides(l_p, &l_v.size, fog_p, fog_s)
        })
    })
    .map(|(_, _, e)| {
        e
    })
    .collect();

    fog_death_writer.send(FogDieEvent{fogs})
}

fn fog_killer(
    mut commands: Commands,
    mut fog_death: EventReader<FogDieEvent>,
) {
    for event in fog_death.iter() {
        for fog_ent in event.fogs.iter() {
            commands.entity(*fog_ent).despawn();
        }
    }
}