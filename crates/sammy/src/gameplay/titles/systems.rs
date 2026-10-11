#[allow(unused_imports)]
use super::*;

pub fn update_titles(
    time: Res<Time>,
    controls: Res<crate::platform::input::Controls>,
    scanners: Res<crate::gameplay::extro::Scanners>,
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
