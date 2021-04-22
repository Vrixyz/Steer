use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    input::mouse::MouseButtonInput,
    math::Quat,
    prelude::*,
    render::draw::OutsideFrustum,
    sprite::SpriteSettings,
};

use bevy_prototype_debug_lines::*;

use rand::Rng;

const CAMERA_SPEED: f32 = 1000.0;

pub struct PrintTimer(Timer);
pub struct Position(Transform);
pub struct MainCamera;
pub struct PlayerCommander;
pub struct PlayerCommanderTargetLine;

pub struct CommanderInput {
    pub target_pos: Vec2,
}

pub struct Velocity(Vec2);
pub struct SteeringManager {
    pub steeringTarget: Vec2,
}

const MAX_SPEED: f32 = 100.0;

/// From https://github.com/Unity-Technologies/UnityCsReference/blob/master/Runtime/Export/Math/Vector3.cs
fn move_towards(current: Vec2, target: Vec2, maxDistanceDelta: f32) -> Vec2 {
    // avoid vector ops because current scripting backends are terrible at inlining
    let toVector_x = target.x - current.x;
    let toVector_y = target.y - current.y;

    let sqdist = toVector_x * toVector_x + toVector_y * toVector_y;

    if sqdist == 0.0 || (maxDistanceDelta >= 0.0 && sqdist <= maxDistanceDelta * maxDistanceDelta) {
        return target;
    }
    let dist = sqdist.sqrt();

    return Vec2::new(
        current.x + toVector_x / dist * maxDistanceDelta,
        current.y + toVector_y / dist * maxDistanceDelta,
    );
}

impl SteeringManager {
    pub fn do_seek(current_position: Vec2, target: Vec2, current_speed: Vec2) -> Vec2 {
        let mut desired = target - current_position;
        let distance = desired.length();

        desired = desired.normalize_or_zero();
        let slowing_radius = 50.0;

        if distance <= slowing_radius {
            desired *= MAX_SPEED * distance / slowing_radius;
        } else {
            desired *= MAX_SPEED;
        }
        let force = desired - current_speed;

        return force;
    }
}

///This example is for performance testing purposes.
///See https://github.com/bevyengine/bevy/pull/1492
fn main() {
    App::build()
        //.add_plugin(LogDiagnosticsPlugin::default())
        //.add_plugin(FrameTimeDiagnosticsPlugin::default())
        .insert_resource(SpriteSettings {
            // NOTE: this is an experimental feature that doesn't work in all cases
            frustum_culling_enabled: true,
        })
        .insert_resource(CommanderInput {
            target_pos: Vec2::splat(0.0),
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(DebugLinesPlugin)
        .add_system(command_debug.system())
        .add_system(steering_debug.system())
        .add_system(velocity_debug.system())
        .add_startup_system(setup.system())
        //.add_system(tick.system().label("Tick"))
        .add_system(my_input_system.system())
        .add_system(steering_targets_influence.system().label("steering_update"))
        .add_system(commander_input_apply.system().before("steering_update"))
        .add_system(velocity.system().after("steering_update"))
        //.add_system(move_camera.system())
        .run()
}

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut rng = rand::thread_rng();

    let tile_size = Vec2::splat(64.0);
    let map_size = Vec2::splat(320.0);

    let half_x = (map_size.x / 2.0) as i32;
    let half_y = (map_size.y / 2.0) as i32;

    let sprite_handle = materials.add(assets.load("branding/icon.png").into());

    commands
        .spawn()
        .insert_bundle(OrthographicCameraBundle::new_2d())
        .insert(MainCamera)
        .insert(Position(Transform::from_translation(Vec3::new(
            0.0, 0.0, 1000.0,
        ))));

    let position = Vec2::new(0.0, 0.0);
    let translation = (position * tile_size).extend(0.0);
    let rotation = Quat::from_rotation_z(rng.gen::<f32>());

    commands
        .spawn()
        .insert_bundle(SpriteBundle {
            material: sprite_handle.clone(),
            sprite: Sprite::new(tile_size),
            ..Default::default()
        })
        .insert(PlayerCommander)
        .insert(Velocity(Vec2::default()))
        .insert(SteeringManager {
            steeringTarget: Vec2::default(),
        });
}

fn commander_input_apply(
    commander_input: ResMut<CommanderInput>,
    mut steering_managers: Query<(
        &Transform,
        &mut SteeringManager,
        &Velocity,
        &PlayerCommander,
    )>,
) {
    for (transform, mut commander, velocity, _) in steering_managers.iter_mut() {
        commander.steeringTarget = SteeringManager::do_seek(
            transform.translation.into(),
            commander_input.target_pos,
            velocity.0,
        );
    }
}

fn steering_targets_influence(
    mut steering_managers: Query<(&Transform, &mut Velocity, &mut SteeringManager)>,
) {
    for (transform, mut velocity, mut manager) in steering_managers.iter_mut() {
        manager.steeringTarget = manager.steeringTarget.clamp_length_max(10.0);

        velocity.0 += manager.steeringTarget;
        velocity.0.clamp_length_max(MAX_SPEED);
    }
}

fn velocity(time: Res<Time>, mut vel: Query<(&mut Transform, &Velocity)>) {
    for mut v in vel.iter_mut() {
        v.0.translation += (v.1 .0 * time.delta_seconds()).extend(0.0);
    }
}

fn move_camera(
    mut commander_camera: QuerySet<(
        Query<(&PlayerCommander, &Transform)>,
        Query<(&mut Transform, &mut Position, &MainCamera)>,
    )>,
) {
    let commander_translation = match commander_camera.q0_mut().iter().last() {
        Some(it) => it.1.translation,
        _ => return,
    };
    let mut camera = match commander_camera.q1_mut().iter_mut().last() {
        Some(it) => it,
        _ => return,
    };
    camera.0.translation = commander_translation;
}

pub fn my_input_system(
    window: ResMut<Windows>,
    // query to get camera transform
    q_camera: Query<&Transform, With<MainCamera>>,
    mut ev_cursor: EventReader<CursorMoved>,
    mut ev_mousebtn: EventReader<MouseButtonInput>,
    mut commander_input: ResMut<CommanderInput>,
) {
    for ev in ev_mousebtn.iter() {
        if ev.state.is_pressed() {
            if let Some(pos) = ev_cursor.iter().last() {
                let camera_transform = q_camera.iter().next().unwrap();
                let wnd = window.get(pos.id).unwrap();
                let size = Vec2::new(wnd.width() as f32, wnd.height() as f32);

                // the default orthographic projection is in pixels from the center;
                // just undo the translation
                let p = pos.position - size / 2.0;

                // apply the camera transform
                let pos_wld = camera_transform.compute_matrix() * p.extend(0.0).extend(1.0);
                commander_input.target_pos = pos_wld.into();
                eprintln!(
                    "Just pressed mouse button: {:?} at {:?}",
                    ev.button, pos.position
                );
            }
        }
    }
}
pub fn command_debug(
    commander_input: Res<CommanderInput>,
    commander: Query<(&PlayerCommander, &Transform)>,
    mut lines: ResMut<DebugLines>,
) {
    let c = match commander.iter().last() {
        Some(it) => it,
        _ => return,
    };
    lines.line(c.1.translation, commander_input.target_pos.extend(0.0), 0.0);
}

pub fn steering_debug(
    commander_input: Res<CommanderInput>,
    commander: Query<(&SteeringManager, &Transform, &Velocity)>,
    mut lines: ResMut<DebugLines>,
) {
    let c = match commander.iter().last() {
        Some(it) => it,
        _ => return,
    };
    let start = c.1.translation + c.2 .0.extend(0.0);
    lines.line_colored(
        start,
        start + c.0.steeringTarget.extend(0.0),
        0.0,
        Color::RED,
    );
}
pub fn velocity_debug(
    commander_input: Res<CommanderInput>,
    commander: Query<(&Velocity, &Transform)>,
    mut lines: ResMut<DebugLines>,
) {
    let c = match commander.iter().last() {
        Some(it) => it,
        _ => return,
    };
    lines.line_colored(
        c.1.translation,
        c.1.translation + c.0 .0.extend(0.0),
        0.0,
        Color::GREEN,
    );
}
