
mod arena;
mod walls;
mod ant;
mod food;
mod fog;

use bevy::prelude::*;

use crate::arena::*;
use crate::walls::*;
use crate::ant::*;
use crate::food::*;
use crate::fog::FogOfWarPlugin;


fn main() {
    App::new()
        .add_plugin(ArenaPlugin)
        .add_plugin(WallPlugin)
        .add_plugin(AntPlugin)
        .add_plugin(FoodPlugin)
        .add_plugin(FogOfWarPlugin)
        .add_plugins(DefaultPlugins)
        .run();
}

