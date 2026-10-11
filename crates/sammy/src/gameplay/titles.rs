//! Help signs ([`sammy_core::titles`]), drawn on top of the world (`Near`).

use bevy::prelude::*;
use formats::wmb::Object;

use super::player::TICKS;
use crate::schedule::GameSet;
#[allow(unused_imports)]
use bevy::prelude::{App, IntoScheduleConfigs, Plugin, PreUpdate, Update};

/// Help signs.
pub struct TitlesPlugin;

impl Plugin for TitlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_titles.in_set(GameSet::Titles));
    }
}


#[derive(Component, Deref, DerefMut)]
pub struct Title(sammy_core::titles::Title);

pub fn init_title(obj: &mut Object, entity: &mut EntityCommands) {
    let title = sammy_core::titles::Title::from_object(obj);
    // The sign is drawn where its trigger centre is.
    obj.origin = title.center;
    entity.insert((Title(title), Visibility::Hidden));
}

pub fn update_titles(
    time: Res<Time>,
    controls: Res<crate::platform::input::Controls>,
    scanners: Res<super::extro::Scanners>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(&mut Title, &mut Visibility, &MeshMaterial3d<StandardMaterial>)>,
) {
    let t = time.delta_secs() * TICKS;
    for (mut title, mut vis, mat) in &mut q {
        // Any id-25 scanner triggers a title (Intro2.wdl:547-556), not only Sammy.
        let nearest = scanners.nearest(title.center).map(|(_, d)| d);
        let shown = title.step(nearest, controls.confirm, t);
        *vis = if shown { Visibility::Inherited } else { Visibility::Hidden };
        if shown && let Some(mut m) = materials.get_mut(&mat.0) {
            m.base_color.set_alpha((title.alpha / 100.0).clamp(0.0, 1.0));
        }
    }
}
