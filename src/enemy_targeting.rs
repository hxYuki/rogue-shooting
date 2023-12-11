use crate::*;
// use bevy::prelude::*;

pub fn enemy_search_nearist_player(
    player: Query<(Entity, &Transform, &LinearVelocity), (With<Player>, With<Life>)>,
    wait_target: Query<
        (
            Entity,
            &Transform,
            Option<&MoveTargetingType>,
            Option<&AimTargetingType>,
        ),
        (With<Enemy>, Without<HostileTarget>),
    >,
    mut commands: Commands,
) {
    wait_target.for_each(|(entity, transform, move_targeting, aim_targeting)| {
        let Some(nearist) = player.iter().min_by_key(|(_, player_transform, _)| {
            transform.translation.distance(player_transform.translation) as i32
        }) else {
            return;
        };
        if move_targeting.is_some() || aim_targeting.is_some() {
            commands.entity(entity).insert(HostileTarget(nearist.0));
        }
    });
}

#[derive(Component)]
pub struct HostileTarget(Entity);

#[derive(Component)]
pub(crate) enum MoveTargetingType {
    Chase,
    Follow,
    Outflank,
}

pub fn move_targeting_system(
    mut commands: Commands,
    mut query: Query<(Entity, &MoveTargetingType, &Transform, &HostileTarget)>,
    living_entities: Query<(Entity, &Transform, &LinearVelocity), (With<Life>)>,
) {
    query.for_each_mut(
        |(entity, targeting_type, transform, HostileTarget(target_entity))| {
            let Some(target) = living_entities.get(*target_entity).ok() else {
                commands.entity(entity).remove::<HostileTarget>();
                return;
            };
            match targeting_type {
                MoveTargetingType::Chase => {
                    let (_, player_transform, player_speed) = target;
                    commands
                        .entity(entity)
                        .insert(movements::Movement::PointMove(
                            player_transform.translation.truncate(),
                        ));
                }
                MoveTargetingType::Follow => {
                    let (_, player_transform, player_speed) = target;
                    if transform.translation.distance(player_transform.translation) > 100. {
                        commands
                            .entity(entity)
                            .insert(movements::Movement::PointMove(
                                player_transform.translation.truncate(),
                            ));
                    } else {
                        commands.entity(entity).remove::<movements::Movement>();
                    }
                }
                MoveTargetingType::Outflank => {
                    let (_, player_transform, player_speed) = target;
                    let target =
                        player_transform.translation.truncate() + player_speed.0.normalize() * 1.5;
                    commands
                        .entity(entity)
                        .insert(movements::Movement::PointMove(target));
                }
            }
        },
    );
}

#[derive(Component)]
pub struct AimTarget(Entity);
#[derive(Component)]
pub(crate) enum AimTargetingType {
    AimCurrent,
    AimPredict,
}
pub fn aim_targeting_system(
    mut query: Query<(&AimTargetingType, &mut Aims, &HostileTarget), With<Enemy>>,
    player: Query<(&Transform, &LinearVelocity), (With<Life>)>,
) {
    query.for_each_mut(|(targeting_type, mut aims, HostileTarget(target_entity))| {
        let Some(target) = player.get(*target_entity).ok() else {
            return;
        };
        match targeting_type {
            AimTargetingType::AimCurrent => {
                let (player_transform, player_speed) = target;
                aims.0 = player_transform.translation.truncate();
            }
            AimTargetingType::AimPredict => {
                let (player_transform, player_speed) = target;
                aims.0 = player_transform.translation.truncate() + player_speed.0 * 0.5;
            }
        }
    });
}

