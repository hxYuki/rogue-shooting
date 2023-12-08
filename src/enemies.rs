use bevy::ecs::query::With;

use crate::*;

pub trait EnemyType: Component {
    const TEXT: &'static str;

    fn to_component(self) -> impl Component;
}

pub fn type_dispatch(str: &str) -> impl EnemyType {
    match str {
        normal_enemy::NormalEnemy::TEXT => normal_enemy::NormalEnemy,
        _ => panic!("unknown enemy type"),
    }
}

pub mod normal_enemy {
    use super::*;

    #[derive(Component)]
    pub struct NormalEnemy;

    impl EnemyType for NormalEnemy {
        const TEXT: &'static str = "normal";
        fn to_component(self) -> impl Component {
            self
        }
    }

    pub fn normal_enemy_initializer(
        mut commands: Commands,
        to_initialize: Query<(Entity, &InitPosition), (With<Character>, With<NormalEnemy>)>,
    ) {
        to_initialize.for_each(|(entity, initial_pos)| {
            commands
                .entity(entity)
                .insert((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::ORANGE_RED,
                            rect: Some(Rect {
                                min: Vec2::new(0.0, 0.0),
                                max: Vec2::new(32.0, 32.0),
                            }),
                            ..Default::default()
                        },
                        transform: initial_pos.0,
                        ..Default::default()
                    },
                    Life(100),
                    Aims(Vec2::ZERO),
                    movements::Movable { speed: 150.0 },
                    Collider::ball(16.),
                    // Sensor,
                    CollisionLayers::new([Layer::Enemy], [Layer::Player, Layer::PlayerBullet]),
                    AimTargetingType::AimCurrent,
                    MoveTargetingType::Chase,
                ))
                .remove::<InitPosition>();
        });
    }
}

