use crate::*;

pub struct SteeringManager {
    pub steering_target: Vec2,
}

pub const MAX_SPEED: f32 = 100.0;

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

pub fn steering_targets_influence(
    time: Res<Time>,
    mut steering_managers: Query<(&Transform, &mut Velocity, &mut SteeringManager)>,
) {
    for (_transform, mut velocity, mut manager) in steering_managers.iter_mut() {
        manager.steering_target = manager.steering_target.clamp_length_max(100.0);

        velocity.0 += manager.steering_target;
        velocity.0.clamp_length_max(MAX_SPEED);
    }
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
