use bevy::{ecs::reflect::ReflectCommandExt, prelude::*, sprite::MaterialMesh2dBundle};
use dyn_clone::DynClone;

use crate::*;

#[derive(Component, Reflect, Clone, Copy)]
pub(crate) struct Bullet {
    pub life_time: f32,
    pub endurance: f32,
    pub hit_limit: f32,
    pub speed: f32,
    pub cooldown: f32,
    pub damage: f32,
}

#[derive(Event)]
pub(crate) struct BulletSpawnEvent {
    pub shooter: Transform,
    pub by: Entity,
    pub with: Entity,
    pub bullet: Bullet,
    pub bullet_type: Box<dyn BulletType>,
    pub generation: usize,
}
#[derive(Component)]
pub(crate) struct Shooter(Entity);
#[derive(Component)]
pub(crate) struct InitPosition(Transform);

#[derive(Component)]
pub(crate) struct BulletGeneration(pub usize);

pub(crate) fn bullet_spawner(
    mut spawn_event: EventReader<BulletSpawnEvent>,
    mut commands: Commands,
    player_entity: Query<Entity, With<Player>>,
) {
    spawn_event.read().for_each(|event| {
        let is_player_shoot = player_entity.get(event.by).is_ok();

        let repeats = if event
            .bullet_type
            .as_reflect()
            .is::<splash_shot::SplashShot>()
        {
            event
                .bullet_type
                .as_reflect()
                .downcast_ref::<splash_shot::SplashShot>()
                .unwrap()
                .count
        } else {
            1
        };
        for _ in 0..repeats {
            let mut bullet_ec = commands.spawn((
                InitPosition(event.shooter),
                Shooter(event.by),
                WeaponRef(event.with),
                BulletGeneration(event.generation + 1),
                event.bullet,
                HitCount::default(),
            ));
            bullet_ec.insert_reflect(event.bullet_type.clone());
            if is_player_shoot {
                bullet_ec.insert(Player).insert(CollisionLayers::new(
                    [Layer::PlayerBullet],
                    [Layer::Enemy, Layer::EnemyBullet],
                ));
            } else {
                bullet_ec.insert(CollisionLayers::new(
                    [Layer::EnemyBullet],
                    [Layer::Player, Layer::PlayerBullet],
                ));
            }
        }
    });
}

#[derive(Event, Debug)]
pub(crate) struct BulletSucceedEvent {
    pub weapon: Entity,
    pub generation: usize,
    pub bullet: Entity,
    pub transform: Transform,
}
pub(crate) fn bullet_succeed(
    mut reader: EventReader<BulletSucceedEvent>,
    mut writer: EventWriter<bullets::BulletSpawnEvent>,
    mut weapons: Query<(&Weapon, &Parent)>,
) {
    reader.read().for_each(|event| {
        let (weapon, shooter) = weapons.get_mut(event.weapon).unwrap();
        if let Some(next_bullet) = weapon.loads.get(event.generation) {
            writer.send(bullets::BulletSpawnEvent {
                shooter: event.transform,
                by: shooter.get(),
                with: event.weapon,
                bullet: next_bullet.0,
                bullet_type: next_bullet.1.clone(),
                generation: event.generation,
            });
        };
    })
}

#[derive(Component)]
pub(crate) struct BulletBeforeDespawn;
pub(crate) fn bullet_before_despawn(
    mut commands: Commands,
    bullets_to_despawned: Query<
        (
            Entity,
            &WeaponRef,
            &BulletGeneration,
            &Transform,
            Option<&BulletEndurance>,
        ),
        (With<Bullet>, With<BulletBeforeDespawn>),
    >,
    mut succeed_event_writer: EventWriter<BulletSucceedEvent>,
) {
    bullets_to_despawned.for_each(
        |(
            entity,
            WeaponRef(weapon_entity),
            BulletGeneration(next_generation),
            transform,
            endurance,
        )| {
            if endurance.is_some() && endurance.unwrap().0.is_finite() {
                succeed_event_writer.send(BulletSucceedEvent {
                    weapon: *weapon_entity,
                    generation: *next_generation,
                    bullet: entity,
                    transform: *transform,
                });
            }

            commands.entity(entity).despawn();
        },
    );
}

#[derive(Component)]
pub(crate) struct LifeTime(Timer);
pub(crate) fn bullet_lifetime(
    mut commands: Commands,
    mut query: Query<(Entity, &mut LifeTime)>,
    time: Res<Time>,
) {
    query.for_each_mut(|(entity, mut life_time)| {
        if life_time.0.tick(time.delta()).just_finished() {
            commands.entity(entity).insert(BulletBeforeDespawn);
        }
    });
}

#[derive(Component)]
pub(crate) struct BulletEndurance(pub f32);
pub(crate) fn bullet_endurance(mut commands: Commands, query: Query<(Entity, &BulletEndurance)>) {
    query.for_each(|(entity, endurance)| {
        if endurance.0 < 0. {
            commands
                .entity(entity)
                .insert(BulletBeforeDespawn)
                .remove::<BulletEndurance>();
        }
    });
}

pub(crate) trait BulletType: Reflect + DynClone {}
dyn_clone::clone_trait_object!(BulletType);

type BoxedBulletType = Box<dyn BulletType>;

pub(crate) trait BulletTypeExt {
    fn to_dyn(self) -> Box<dyn Reflect>;
}
impl<T: BulletType> BulletTypeExt for T {
    fn to_dyn(self) -> Box<dyn Reflect> {
        Box::new(self)
    }
}

pub(crate) mod lane_shot {
    use super::*;

    #[derive(Component, Reflect, Default, Clone, Copy)]
    #[reflect(Component)]
    pub(crate) struct LaneShot;

    impl BulletType for LaneShot {}

    pub(crate) fn lane_shot_move_system(
        mut commands: Commands,
        mut query: Query<
            (Entity, &mut Bullet, &mut Transform),
            (With<LaneShot>, Without<Movement>),
        >,
    ) {
        query.for_each_mut(|(entity, bullet, transform)| {
            commands.entity(entity).insert((
                Movement::DirectionMove(transform.rotation.mul_vec3(Vec3::Y).truncate()),
                Movable {
                    speed: bullet.speed,
                },
            ));
        });
    }

    pub(crate) fn lane_shot_bullet_initializer(
        mut commands: Commands,
        query: Query<(Entity, &Bullet, &InitPosition), With<LaneShot>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<ColorMaterial>>,
    ) {
        query.for_each(|(entity, bullet, InitPosition(transform))| {
            commands
                .entity(entity)
                .insert((
                    MaterialMesh2dBundle {
                        mesh: meshes.add(shape::Circle::new(3.).into()).into(),
                        material: materials.add(ColorMaterial::from(Color::ALICE_BLUE)),
                        transform: *transform,
                        ..default()
                    },
                    LifeTime(Timer::from_seconds(bullet.life_time, TimerMode::Once)),
                    BulletEndurance(bullet.endurance),
                    Collider::ball(3.),
                    RigidBody::Dynamic,
                ))
                .remove::<InitPosition>();
        });
    }
}

pub mod explode_shot {
    use super::*;

    #[derive(Component, Reflect, Default, Clone, Copy)]
    #[reflect(Component)]
    pub(crate) struct ExplodeShot;

    impl BulletType for ExplodeShot {}

    pub(crate) fn explode_shot_move_system(
        mut commands: Commands,
        mut query: Query<
            (Entity, &mut Bullet, &mut Transform),
            (With<ExplodeShot>, Without<Movement>),
        >,
    ) {
        query.for_each_mut(|(entity, bullet, transform)| {
            // commands.entity(entity).insert((
            //     // Movement::DirectionMove(transform.rotation.mul_vec3(Vec3::Y).truncate()),
            //     // Movable {
            //     //     speed: bullet.speed,
            //     // },
            // ));
        });
    }

    pub(crate) fn explode_shot_bullet_initializer(
        mut commands: Commands,
        query: Query<(Entity, &Bullet, &InitPosition), With<ExplodeShot>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<ColorMaterial>>,
    ) {
        query.for_each(|(entity, bullet, InitPosition(transform))| {
            commands
                .entity(entity)
                .insert((
                    MaterialMesh2dBundle {
                        mesh: meshes.add(shape::Circle::new(50.).into()).into(),
                        material: materials.add(ColorMaterial::from(Color::CRIMSON)),
                        transform: *transform,
                        ..default()
                    },
                    LifeTime(Timer::from_seconds(bullet.life_time, TimerMode::Once)),
                    BulletEndurance(bullet.endurance),
                    Collider::ball(50.),
                    // RigidBody::Dynamic,
                ))
                .remove::<InitPosition>();
        });
    }
}

pub mod splash_shot {
    use super::*;

    #[derive(Component, Reflect, Default, Clone, Copy)]
    #[reflect(Component)]
    pub(crate) struct SplashShot {
        pub count: usize,
        pub angle: f32,
    }

    impl BulletType for SplashShot {}

    pub(crate) fn splash_shot_move_system(
        mut commands: Commands,
        mut query: Query<
            (Entity, &WeaponRef, &mut Bullet, &mut Transform, &SplashShot),
            Without<Movement>,
        >,
        mut weapons: Query<(&mut WeaponEntropyComponent, &Weapon)>,
    ) {
        query.for_each_mut(|(entity, WeaponRef(weapon), bullet, transform, splash)| {
            let (mut entropy, _) = weapons.get_mut(*weapon).unwrap();
            let angle = entropy.gen_range(-splash.angle..splash.angle);

            let dir = transform.rotation * Quat::from_rotation_z(angle) * Vec3::Y;
            commands.entity(entity).insert((
                Movement::DirectionMove(dir.truncate()),
                Movable {
                    speed: bullet.speed,
                },
            ));
        });
    }

    pub(crate) fn splash_shot_bullet_initializer(
        mut commands: Commands,
        query: Query<(Entity, &Bullet, &InitPosition, &SplashShot)>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<ColorMaterial>>,
    ) {
        query.for_each(|(entity, bullet, InitPosition(transform), _)| {
            commands
                .entity(entity)
                .insert((
                    MaterialMesh2dBundle {
                        mesh: meshes.add(shape::Circle::new(3.).into()).into(),
                        material: materials.add(ColorMaterial::from(Color::LIME_GREEN)),
                        transform: *transform,
                        ..default()
                    },
                    LifeTime(Timer::from_seconds(bullet.life_time, TimerMode::Once)),
                    BulletEndurance(bullet.endurance),
                    Collider::ball(3.),
                    RigidBody::Dynamic,
                ))
                .remove::<InitPosition>();
        });
    }
}

pub mod lazer_shot {
    use bevy::sprite::Anchor;

    use super::*;
    #[derive(Component, Reflect, Default, Clone, Copy)]
    #[reflect(Component)]
    pub(crate) struct LazerShot {
        pub length: f32,
        pub width: f32,
    }
    impl BulletType for LazerShot {}
    pub(crate) fn lazer_shot_move_system(
        mut commands: Commands,
        mut query: Query<
            (Entity, &mut Bullet, &mut Transform, &Shooter),
            (With<LazerShot>, Without<Aims>, Without<InitPosition>),
        >,
        shooters: Query<(Entity, &Aims)>,
    ) {
        query.for_each_mut(|(entity, bullet, transform, Shooter(shooter_entity))| {
            let aims = shooters.get(*shooter_entity).unwrap().1;
            commands.entity(entity).insert(*aims);
        });
    }
    pub(crate) fn lazer_shot_bullet_initializer(
        mut commands: Commands,
        query: Query<(Entity, &Bullet, &InitPosition, &Shooter, &LazerShot)>,
        spawned_bullet: Query<(Entity, &Shooter), (With<LazerShot>, Without<InitPosition>)>,
    ) {
        query.for_each(
            |(
                entity,
                bullet,
                InitPosition(transform),
                Shooter(shooter_entity),
                LazerShot { length, width },
            )| {
                if spawned_bullet
                    .iter()
                    .any(|(_, Shooter(existed_shooter))| existed_shooter == shooter_entity)
                {
                    // if there exists a lazer shoot by same shooter, won't spawn new lazer
                    commands.entity(entity).despawn();
                } else {
                    commands
                        .entity(entity)
                        .insert((
                            SpriteBundle {
                                sprite: Sprite {
                                    color: Color::LIME_GREEN,
                                    custom_size: Some(Vec2::new(*width, *length)),
                                    anchor: Anchor::BottomCenter,
                                    ..default()
                                },
                                transform: *transform,
                                ..default()
                            },
                            LifeTime(Timer::from_seconds(bullet.life_time, TimerMode::Once)),
                            BulletEndurance(bullet.endurance),
                            Collider::cuboid(*width, *length),
                            RigidBody::Dynamic,
                        ))
                        .remove::<InitPosition>();
                }
            },
        );
    }
}

