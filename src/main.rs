use bevy::{
    prelude::*,
    sprite::{collide_aabb::collide, SpriteSettings},
};

use bevy_prototype_debug_lines::*;

pub struct PrintTimer(Timer);
pub struct Position(Transform);
pub struct MainCamera;
pub struct PlayerCommander;
pub struct PlayerCommanderTargetLine;

#[derive(PartialEq)]
pub enum FireMode {
    Active,
    Idle,
}

// TODO: we can split input for movement and input for attack
pub struct CommanderInput {
    pub desired_direction: Vec2,
    pub fire_mode: FireMode,
    pub fire_target: Vec2,
}

pub struct AttackAbility {
    pub cooldown: f32,
    pub last_attack: f32,
}

pub struct Velocity(Vec2);
pub struct SteeringManager {
    pub steering_target: Vec2,
}

pub struct Shape {
    pub radius: f32,
}
pub struct DeathOnCollide;

pub struct MyAssets {
    pub bullet: Handle<ColorMaterial>,
    pub ally: Handle<ColorMaterial>,
}

const MAX_SPEED: f32 = 100.0;

impl SteeringManager {
    pub fn do_seek(current_position: Vec2, target: Vec2, current_speed: Vec2, mass: f32) -> Vec2 {
        let mut desired = target - current_position;
        let distance = desired.length();

        desired = desired.normalize_or_zero();
        let slowing_radius = 50.0;

        if distance <= slowing_radius {
            desired *= MAX_SPEED * distance / slowing_radius;
        } else {
            desired *= MAX_SPEED;
        }
        return Self::do_desired(desired, current_speed, mass);
    }
    pub fn do_desired(desired: Vec2, current_speed: Vec2, mass: f32) -> Vec2 {
        let force = desired - current_speed;

        return force / mass;
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
            desired_direction: Vec2::splat(0.0),
            fire_mode: FireMode::Idle,
            fire_target: Vec2::splat(0.0),
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(DebugLinesPlugin)
        .add_system(command_debug.system())
        .add_system(steering_debug.system())
        .add_system(velocity_debug.system())
        .add_startup_system(setup.system())
        //.add_system(tick.system().label("Tick"))
        .add_system(my_input_system.system())
        .add_system(commander_attack_apply.system())
        .add_system(steering_targets_influence.system().label("steering_update"))
        .add_system(commander_input_apply.system().before("steering_update"))
        .add_system(
            velocity
                .system()
                .after("steering_update")
                .label("velocity_update"),
        )
        .add_system(collisions_border.system().after("velocity_update"))
        .add_system(collisions_death.system().after("velocity_update"))
        //.add_system(move_camera.system())
        .run()
}

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let tile_size = Vec2::splat(64.0);
    let sprite_handle = materials.add(assets.load("branding/icon.png").into());

    commands.insert_resource(MyAssets {
        bullet: sprite_handle.clone(),
        ally: sprite_handle.clone(),
    });

    commands
        .spawn()
        .insert_bundle(OrthographicCameraBundle::new_2d())
        .insert(MainCamera)
        .insert(Position(Transform::from_translation(Vec3::new(
            0.0, 0.0, 1000.0,
        ))));

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
            steering_target: Vec2::default(),
        })
        .insert(AttackAbility {
            cooldown: 1.0 / 2.0,
            last_attack: 0.0,
        });
}

fn commander_input_apply(
    commander_input: ResMut<CommanderInput>,
    mut steering_managers: Query<(&mut SteeringManager, &Velocity, &PlayerCommander)>,
) {
    for (mut commander, velocity, _) in steering_managers.iter_mut() {
        commander.steering_target = SteeringManager::do_desired(
            commander_input.desired_direction * MAX_SPEED,
            velocity.0,
            20.0,
        );
    }
}
fn commander_attack_apply(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<MyAssets>,
    commander_input: ResMut<CommanderInput>,
    mut attacker: Query<(&Transform, &mut AttackAbility)>,
) {
    for (transform, mut attack) in attacker.iter_mut() {
        if commander_input.fire_mode == FireMode::Active {
            let time_since_startup = time.time_since_startup().as_secs_f32();
            if attack.last_attack + attack.cooldown < time_since_startup {
                attack.last_attack = time_since_startup;
                let position = transform.translation;
                commands
                    .spawn()
                    .insert_bundle(SpriteBundle {
                        material: assets.bullet.clone(),
                        sprite: Sprite::new(Vec2::splat(16.0)),
                        transform: Transform::from_translation(position),
                        ..Default::default()
                    })
                    .insert(DeathOnCollide)
                    .insert(Shape { radius: 16. })
                    .insert(Velocity(
                        (commander_input.fire_target - position.into()).normalize_or_zero()
                            * MAX_SPEED
                            * 2.0,
                    ));
            }
        }
    }
}
/*fn commander_input_apply(
    commander_input: ResMut<CommanderInput>,
    mut steering_managers: Query<(
        &Transform,
        &mut SteeringManager,
        &Velocity,
        &PlayerCommander,
    )>,
) {
    for (transform, mut commander, velocity, _) in steering_managers.iter_mut() {
        commander.steering_target
         = SteeringManager::do_seek(
            transform.translation.into(),
            commander_input.target_pos,
            velocity.0,
            20.0
        );
    }
}*/

fn steering_targets_influence(
    time: Res<Time>,
    mut steering_managers: Query<(&Transform, &mut Velocity, &mut SteeringManager)>,
) {
    for (_transform, mut velocity, mut manager) in steering_managers.iter_mut() {
        manager.steering_target = manager.steering_target.clamp_length_max(100.0);

        velocity.0 += manager.steering_target;
        velocity.0.clamp_length_max(MAX_SPEED);
    }
}

fn velocity(time: Res<Time>, mut vel: Query<(&mut Transform, &Velocity)>) {
    for mut v in vel.iter_mut() {
        v.0.translation += (v.1 .0 * time.delta_seconds()).extend(0.0);
    }
}
fn collisions_border(mut collision_checks: Query<(&mut Transform, &Velocity)>) {
    let bounds_x = (-300., 300.);
    let bounds_y = (-200., 200.);
    for (mut t, velocity) in collision_checks.iter_mut() {
        if t.translation.x < bounds_x.0 || bounds_x.1 < t.translation.x {
            t.translation.x = t.translation.x.clamp(bounds_x.0, bounds_x.1)
        }
        if t.translation.y < bounds_y.0 || bounds_y.1 < t.translation.y {
            t.translation.y = t.translation.y.clamp(bounds_y.0, bounds_y.1)
        }
    }
}
fn collisions_death(
    mut commands: Commands,
    mut collision_checks: QuerySet<(
        Query<(Entity, &Transform, &Shape), With<DeathOnCollide>>,
        Query<(Entity, &Transform, &Shape), With<DeathOnCollide>>,
    )>,
) {
    let bounds_x = (-300., 300.);
    let bounds_y = (-200., 200.);
    let mut firstCheck = 1;
    for (e1, t1, s1) in collision_checks.q0().iter() {
        for (e2, t2, s2) in collision_checks.q1().iter().skip(firstCheck) {
            if collide(
                t1.translation,
                Vec2::splat(s1.radius),
                t2.translation,
                Vec2::splat(s1.radius),
            )
            .is_some()
            {
                commands.entity(e1).despawn();
                commands.entity(e2).despawn();
            }
        }
        firstCheck += 1;
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
    window: Res<Windows>,
    mut commander_input: ResMut<CommanderInput>,
    mouse_button_input: Res<Input<MouseButton>>,
    keyboard_input: Res<Input<KeyCode>>,
    mut ev_cursor: EventReader<CursorMoved>,
    // query to get camera transform
    q_camera: Query<&Transform, With<MainCamera>>,
) {
    if mouse_button_input.pressed(MouseButton::Left) {
        commander_input.fire_mode = FireMode::Active;
    } else {
        commander_input.fire_mode = FireMode::Idle;
    }
    if keyboard_input.is_changed() {
        let mut movement = Vec2::splat(0.0);
        if keyboard_input.pressed(KeyCode::Z) {
            movement += Vec2::Y;
        }
        if keyboard_input.pressed(KeyCode::S) {
            movement += -Vec2::Y;
        }
        if keyboard_input.pressed(KeyCode::D) {
            movement += Vec2::X;
        }
        if keyboard_input.pressed(KeyCode::Q) {
            movement += -Vec2::X;
        }
        commander_input.desired_direction = movement.normalize_or_zero();
    }

    if let Some(pos) = ev_cursor.iter().last() {
        let camera_transform = q_camera.iter().next().unwrap();
        let wnd = window.get(pos.id).unwrap();
        let size = Vec2::new(wnd.width() as f32, wnd.height() as f32);

        // the default orthographic projection is in pixels from the center;
        // just undo the translation
        let p = pos.position - size / 2.0;

        // apply the camera transform
        let pos_wld = camera_transform.compute_matrix() * p.extend(0.0).extend(1.0);

        commander_input.fire_target = pos_wld.into();
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
    lines.line(
        c.1.translation,
        c.1.translation + commander_input.desired_direction.extend(0.0) * MAX_SPEED,
        0.0,
    );
}

pub fn steering_debug(
    _commander_input: Res<CommanderInput>,
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
        start + c.0.steering_target.extend(0.0),
        0.0,
        Color::RED,
    );
}
pub fn velocity_debug(
    _commander_input: Res<CommanderInput>,
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
