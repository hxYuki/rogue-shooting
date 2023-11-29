use bevy::{math::vec3, prelude::*};
#[derive(Component)]
pub(crate) struct ForcedMove {
    pub(crate) direction: Vec2,
    pub(crate) speed: f32,
}

pub(crate) fn forced_move_system(
    mut moves: Query<(Entity, &ForcedMove, &mut Transform)>,
    time: Res<Time>,
) {
    moves.for_each_mut(|(_, forced_move, mut transform)| {
        transform.translation +=
            forced_move.direction.extend(0.0) * forced_move.speed * time.delta_seconds();
    });
}

#[derive(Component)]
pub(crate) struct Shocked {
    pub(crate) impact: f32,
    pub(crate) direction: Vec2,
}

#[derive(Component)]
pub(crate) struct ShockedTimer(Timer);

pub(crate) fn shock_system(
    mut commands: Commands,
    mut query: Query<(Entity, &Shocked, &ShockedTimer)>,
) {
    query.for_each_mut(|(entity, shocked, timer)| {
        commands.entity(entity).insert(ForcedMove {
            direction: shocked.direction,
            speed: shocked.impact * impact_function(timer.0.elapsed().as_secs_f32()) * 400.,
        });
    });

    fn impact_function(t: f32) -> f32 {
        16.6667 * t - 75. * t * t + 83.3333 * t * t * t
    }
}

pub(crate) fn shock_timer_system(
    mut commands: Commands,
    mut query: Query<(Entity, Option<&mut ShockedTimer>), With<Shocked>>,
    time: Res<Time>,
) {
    query.for_each_mut(|(entity, timer)| {
        if let Some(mut timer) = timer {
            if timer.0.tick(time.delta()).finished() {
                commands
                    .entity(entity)
                    .remove::<(Shocked, ShockedTimer, ForcedMove)>();
            }
        } else {
            commands
                .entity(entity)
                .insert(ShockedTimer(Timer::from_seconds(0.4, TimerMode::Once)));
        }
    });
}

