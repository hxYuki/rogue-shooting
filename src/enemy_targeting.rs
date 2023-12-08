use crate::*;
// use bevy::prelude::*;

#[derive(Component)]
pub(crate) enum MoveTargetingType {
    Chase,
    Follow,
    Outflank,
}

pub fn move_targeting_system(
    mut commands: Commands,
    mut query: Query<(Entity, &MoveTargetingType, &Transform), With<Enemy>>,
    player: Query<(Entity, &Transform, &LinearVelocity), (With<Player>, With<Life>)>,
) {
    let Some((_, player_transform, player_speed)) = player.iter().next() else {
        return;
    };

    query.for_each_mut(|(entity, targeting_type, transform)| match targeting_type {
        MoveTargetingType::Chase => {
            commands
                .entity(entity)
                .insert(Movement::PointMove(player_transform.translation.truncate()));
        }
        MoveTargetingType::Follow => {
            if transform.translation.distance(player_transform.translation) > 100. {
                commands
                    .entity(entity)
                    .insert(Movement::PointMove(player_transform.translation.truncate()));
            } else {
                commands.entity(entity).remove::<Movement>();
            }
        }
        MoveTargetingType::Outflank => {
            let target = player_transform.translation.truncate() + player_speed.0.normalize() * 1.5;
            commands.entity(entity).insert(Movement::PointMove(target));
        }
    });
}

#[derive(Component)]
pub(crate) enum AimTargetingType {
    AimCurrent,
    AimPredict,
}
pub fn aim_targeting_system(
    mut query: Query<(&AimTargetingType, &mut Aims), With<Enemy>>,
    player: Query<(&Transform, &LinearVelocity), (With<Player>, With<Life>)>,
) {
    let Some((player_transform, player_speed)) = player.iter().next() else {
        return;
    };

    query.for_each_mut(|(targeting_type, mut aims)| match targeting_type {
        AimTargetingType::AimCurrent => {
            aims.0 = player_transform.translation.truncate();
        }
        AimTargetingType::AimPredict => {
            aims.0 = player_transform.translation.truncate() + player_speed.0 * 0.5;
        }
    });
}

