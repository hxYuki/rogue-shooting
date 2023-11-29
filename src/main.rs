#![feature(trait_upcasting)]
#![feature(trivial_bounds)]
use std::{f32::INFINITY, fs::File, io::Write};

use bevy::{
    prelude::*,
    render::{
        settings::{Backends, RenderCreation, WgpuSettings},
        RenderPlugin,
    },
    utils::HashMap,
};

use bevy_inspector_egui::quick::WorldInspectorPlugin;

use bevy_rand::prelude::*;
use bevy_xpbd_2d::prelude::*;
use enemy_targeting::{AimTargetingType, MoveTargetingType};
use rand::prelude::*;

mod enemy_targeting;
mod forced_move;
mod input_handling;
use bullets::*;
use input_handling::KeyboardControlled;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    backends: Some(Backends::VULKAN),
                    ..default()
                }),
            }),
            PhysicsPlugins::default(),
            EntropyPlugin::<WyRand>::default(),
            WorldInspectorPlugin::new(),
        ))
        .insert_resource(Gravity(Vec2::ZERO))
        .register_type::<bullets::lane_shot::LaneShot>()
        .register_type::<bullets::explode_shot::ExplodeShot>()
        .register_type::<bullets::splash_shot::SplashShot>()
        // register for Character entity components
        .register_type::<Player>()
        .register_type::<Enemy>()
        .register_type::<Character>()
        .register_type::<Life>()
        .register_type::<Movable>()
        .register_type::<Bullet>()
        .add_event::<bullets::BulletSpawnEvent>()
        .add_event::<BulletHitEvent>()
        .add_event::<bullets::BulletSucceedEvent>()
        // .add_plugins(space_editor::SpaceEditorPlugin::default())
        .add_systems(Startup, (setup_camera, spawn_player))
        .add_systems(
            Update,
            (
                input_handling::handle_input,
                input_handling::handle_mouse,
                life_dies_system,
                move_system.before(bullet_before_despawn),
                aim_system,
                player_enemy_shock_system,
                forced_move::forced_move_system,
                (forced_move::shock_system, forced_move::shock_timer_system).chain(),
            ),
        )
        .add_systems(Update, (cooldown_system, shoot_system).chain())
        .add_systems(Update, randomize_weapons)
        .add_systems(
            Update,
            (
                bullets::bullet_spawner,
                // bullet_hit_endurance_system.before(bullets::bullet_endurance),
                hit_damage_system,
                bullets::bullet_succeed,
                bullet_hit_endurance_system,
                // bullets::bullet_endurance.before(bullets::bullet_before_despawn),
                // bullets::bullet_before_despawn,
                bullets::lane_shot::lane_shot_move_system,
                bullets::lane_shot::lane_shot_bullet_initializer,
                bullets::explode_shot::explode_shot_move_system,
                bullets::explode_shot::explode_shot_bullet_initializer,
                bullets::splash_shot::splash_shot_move_system,
                bullets::splash_shot::splash_shot_bullet_initializer,
            ),
        )
        .add_systems(
            Update,
            (
                bullets::bullet_endurance,
                bullets::bullet_lifetime,
                bullets::bullet_before_despawn,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                enemy_targeting::move_targeting_system,
                enemy_targeting::aim_targeting_system,
            )
                .before(life_dies_system),
        )
        .insert_resource(Time::<Fixed>::from_hz(64.))
        .add_systems(FixedUpdate, bullet_hit_system)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    println!("hello world");

    // spawn_player(commands);
}

#[derive(Component, Reflect)]
struct Player;
#[derive(Component, Reflect)]
struct Enemy;
#[derive(Component, Reflect)]
struct Character;

#[derive(Component, Reflect)]
struct Life(i32);
fn life_dies_system(mut commands: Commands, mut query: Query<(Entity, &Life)>) {
    query.for_each_mut(|(entity, life)| {
        if life.0 <= 0 {
            commands.entity(entity).despawn();
        }
    });
}

#[derive(PhysicsLayer)]
enum Layer {
    Player,
    Enemy,
    PlayerBullet,
    EnemyBullet,
}

fn spawn_player(mut commands: Commands, type_registry: Res<AppTypeRegistry>) {
    commands
        .spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(1.0, 1.0, 1.0),
                    rect: Some(Rect {
                        min: Vec2::new(0.0, 0.0),
                        max: Vec2::new(32.0, 32.0),
                    }),
                    ..Default::default()
                },
                ..Default::default()
            },
            Life(100),
            Movable { speed: 300.0 },
            Player,
            Character,
            KeyboardControlled,
            Collider::ball(16.),
            // RigidBody::Kinematic,
            CollisionLayers::new([Layer::Player], [Layer::Enemy, Layer::EnemyBullet]),
        ))
        .with_children(|cb| {
            cb.spawn((
                Weapon {
                    accelerate: 1000.,
                    loads: vec![
                        (
                            Bullet {
                                life_time: 0.3,
                                endurance: 1.,
                                speed: 1000.,
                                cooldown: 0.6,
                                damage: 10.,
                                hit_limit: 1,
                            },
                            Box::new(bullets::splash_shot::SplashShot {
                                count: 1,
                                angle: 0.3,
                            }),
                        ),
                        (
                            Bullet {
                                life_time: 0.3,
                                endurance: INFINITY,
                                speed: 1000.,
                                cooldown: 1.,
                                damage: 80.,
                                hit_limit: 1,
                            },
                            Box::new(bullets::explode_shot::ExplodeShot),
                        ),
                    ],
                },
                Player,
            ));
        });

    spawn_enemy(commands, type_registry);
}

fn spawn_enemy(mut commands: Commands, type_registry: Res<AppTypeRegistry>) {
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(1.0, 1.0, 1.0),
                rect: Some(Rect {
                    min: Vec2::new(0.0, 0.0),
                    max: Vec2::new(32.0, 32.0),
                }),
                ..Default::default()
            },
            transform: Transform::from_translation(Vec3::new(0., 100., 0.)),
            ..Default::default()
        },
        Life(100),
        Enemy,
        Character,
        Aims(Vec2::ZERO),
        Movable { speed: 150.0 },
        Collider::ball(16.),
        // Sensor,
        CollisionLayers::new([Layer::Enemy], [Layer::Player, Layer::PlayerBullet]),
        AimTargetingType::AimCurrent,
        MoveTargetingType::Chase,
    ));
}

type WeaponEntropyComponent = EntropyComponent<WyRand>;
fn randomize_weapons(
    mut commands: Commands,
    mut global: ResMut<GlobalEntropy<WyRand>>,
    weapons: Query<(Entity, &Weapon), Without<WeaponEntropyComponent>>,
) {
    weapons.for_each(|(entity, _)| {
        commands.entity(entity).insert(global.fork_rng());
    })
}
#[derive(Component)]
enum XAxisMove {
    Left,
    Right,
}
#[derive(Component)]
enum YAxisMove {
    Up,
    Down,
}

#[derive(Component)]
enum Movement {
    // XYAxisMove(Option<XAxisMove>, Option<YAxisMove>),
    DirectionMove(Vec2),
    PointMove(Vec2),
}

#[derive(Component, Reflect)]
struct Movable {
    speed: f32,
}

#[derive(Component, Reflect)]
struct Aims(Vec2);

#[derive(Component, Reflect)]
struct IsShooting;

#[derive(Component)]
struct Weapon {
    accelerate: f32,
    loads: Vec<(Bullet, Box<dyn BulletType>)>,
}
impl Default for Weapon {
    fn default() -> Self {
        Self {
            accelerate: 100.,
            loads: vec![],
        }
    }
}

#[derive(Component)]
struct WeaponRef(Entity);
#[derive(Component)]
struct IsCoolingdown(Timer);

fn move_system(
    time: Res<Time>,
    mut query: Query<
        (
            Entity,
            &Movable,
            &mut Transform,
            Option<&Movement>,
            Option<&XAxisMove>,
            Option<&YAxisMove>,
            Option<&mut LinearVelocity>,
            Option<&RigidBody>,
        ),
        Without<forced_move::ForcedMove>,
    >,
    mut commands: Commands,
) {
    query.for_each_mut(
        |(entity, movable, mut transform, movement, x, y, _, rigidbody)| {
            if let Some(movement) = movement {
                match movement {
                    Movement::DirectionMove(dir) => {
                        if rigidbody.is_none() {
                            transform.translation +=
                                dir.extend(0.0) * time.delta_seconds() * movable.speed;
                        }
                        commands
                            .entity(entity)
                            .insert(LinearVelocity(*dir * movable.speed));
                    }
                    Movement::PointMove(point) => {
                        let dir = *point - transform.translation.truncate();
                        if rigidbody.is_none() {
                            transform.translation += dir
                                .extend(0.0)
                                .clamp_length_max(movable.speed * time.delta_seconds());
                        }

                        if dir.length() > movable.speed * time.delta_seconds() {
                            commands
                                .entity(entity)
                                .insert(LinearVelocity(dir.normalize() * movable.speed));
                        } else {
                            commands.entity(entity).remove::<Movement>();
                        }
                    }
                }
            } else {
                let mut direction = Vec2::ZERO;
                if let Some(x) = x {
                    match x {
                        XAxisMove::Left => direction.x -= 1.0,
                        XAxisMove::Right => direction.x += 1.0,
                    }
                }
                if let Some(y) = y {
                    match y {
                        YAxisMove::Up => direction.y += 1.0,
                        YAxisMove::Down => direction.y -= 1.0,
                    }
                }
                if rigidbody.is_none() && direction != Vec2::ZERO {
                    transform.translation +=
                        (direction.normalize() * movable.speed * time.delta_seconds()).extend(0.0);
                }
                commands.entity(entity).insert(LinearVelocity(
                    direction.normalize_or_zero() * movable.speed,
                ));
            };
        },
    );
}

pub(crate) fn player_enemy_shock_system(
    mut commands: Commands,
    colliding: Query<(Entity, &CollidingEntities, &Transform), With<Character>>,
) {
    colliding.for_each(|(entity, colliding_entities, transform)| {
        let Some(c_entity) = colliding_entities
            .iter()
            .filter(|e| colliding.contains(**e))
            .next()
        else {
            return;
        };
        let Ok(target) = colliding.get(*c_entity) else {
            return;
        };
        let self_translation = transform.translation;
        let target_translation = target.2.translation;
        let direction = (self_translation - target_translation).normalize();
        commands.entity(entity).insert(forced_move::Shocked {
            impact: 1.,
            direction: direction.truncate(),
        });
    })
}

fn aim_system(mut query: Query<(&Aims, &mut Transform)>) {
    query.for_each_mut(|(aim, mut transform)| {
        transform.rotation = Quat::from_rotation_z(
            -(aim.0 - transform.translation.truncate()).angle_between(Vec2::Y),
        );
    });
}

fn shoot_system(
    weapons: Query<(Entity, &Weapon, &Parent), (With<IsShooting>, Without<IsCoolingdown>)>,
    weapon_holder: Query<(Entity, &Transform, &Children)>,
    mut commands: Commands,
    mut writer: EventWriter<bullets::BulletSpawnEvent>,
) {
    weapons.for_each(|(entity, weapon, owner)| {
        if let Some((bullet, bullet_type)) = weapon.loads.first() {
            let owner_transform = weapon_holder.get(owner.get()).unwrap().1;
            writer.send(bullets::BulletSpawnEvent {
                shooter: *owner_transform,
                by: owner.get(),
                with: entity,
                bullet: *bullet,
                bullet_type: bullet_type.clone(),
                generation: 0,
            });

            commands
                .entity(entity)
                .insert(IsCoolingdown(Timer::from_seconds(
                    bullet.cooldown * 100. / weapon.accelerate,
                    TimerMode::Once,
                )));
        }
    });
}

fn cooldown_system(
    mut cooling_weapons: Query<(Entity, &mut IsCoolingdown), With<Weapon>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    cooling_weapons.for_each_mut(|(entity, mut cooldown)| {
        if cooldown.0.tick(time.delta()).just_finished() {
            commands.entity(entity).remove::<IsCoolingdown>();
        }
    });
}

#[derive(Component, Default)]
struct HitCount(HashMap<Entity, usize>);

fn bullet_hit_system(
    mut bullets: Query<(
        Entity,
        &CollidingEntities,
        &Bullet,
        &mut HitCount,
        &Transform,
    )>,
    hitable: Query<(Entity, &mut Life)>,
    mut writer: EventWriter<BulletHitEvent>,
) {
    bullets.for_each_mut(
        |(bullet_entity, colliding_entities, bullet, mut hit_count, transform)| {
            if !colliding_entities.is_empty() {
                hitable
                    .iter_many(colliding_entities.iter())
                    .for_each(|(entity, _)| {
                        let count = *hit_count
                            .0
                            .entry(entity)
                            .and_modify(|c| *c += 1)
                            .or_insert(1);

                        if count <= bullet.hit_limit {
                            writer.send(BulletHitEvent {
                                bullet_entity,
                                target: entity,
                                bullet_transform: *transform,
                                first_hit: count == 1,
                            });
                        }
                    });
            }
        },
    );
}

#[derive(Event, Debug)]
struct BulletHitEvent {
    bullet_entity: Entity,
    target: Entity,
    bullet_transform: Transform,
    first_hit: bool,
}

fn bullet_hit_endurance_system(
    mut reader: EventReader<BulletHitEvent>,
    mut bullets: Query<
        (
            &WeaponRef,
            &mut BulletEndurance,
            &Transform,
            &BulletGeneration,
        ),
        With<Bullet>,
    >,
    mut weapons: Query<(&mut WeaponEntropyComponent, &Weapon)>,
    mut succeed_event_writer: EventWriter<bullets::BulletSucceedEvent>,
) {
    reader.read().for_each(|event| {
        if !event.first_hit {
            return;
        }
        let (WeaponRef(weapon_entity), mut endurance, transform, BulletGeneration(generation)) =
            bullets.get_mut(event.bullet_entity).unwrap();

        let (mut entropy, _) = weapons.get_mut(*weapon_entity).unwrap();

        succeed_event_writer.send(bullets::BulletSucceedEvent {
            weapon: *weapon_entity,
            generation: *generation,
            bullet: event.bullet_entity,
            transform: *transform,
        });

        if endurance.0 < 1. {
            if !entropy.gen_bool(endurance.0.into()) {
                endurance.0 = -1.;
            } else {
            }
        } else {
            endurance.0 -= 1.;
        }
    });
}

fn hit_damage_system(
    mut reader: EventReader<BulletHitEvent>,
    mut life: Query<(Entity, &mut Life)>,
    bullets: Query<(Entity, &Bullet)>,
) {
    reader.read().for_each(|event| {
        if let Ok((_, mut life)) = life.get_mut(event.target) {
            let dmg = bullets.get(event.bullet_entity).unwrap().1.damage;
            life.0 -= dmg.ceil() as i32;
        }
    });
}

pub(crate) mod bullets {
    use std::marker::PhantomData;

    use bevy::{ecs::reflect::ReflectCommandExt, prelude::*, sprite::MaterialMesh2dBundle};
    use dyn_clone::DynClone;

    use crate::*;

    #[derive(Component, Reflect, Clone, Copy)]
    pub(crate) struct Bullet {
        pub life_time: f32,
        pub endurance: f32,
        pub hit_limit: usize,
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
    pub(crate) fn bullet_endurance(
        mut commands: Commands,
        query: Query<(Entity, &BulletEndurance)>,
    ) {
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
                commands.entity(entity).insert((
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
                ));

                commands.entity(entity).remove::<InitPosition>();
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
                commands.entity(entity).insert((
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
                ));

                commands.entity(entity).remove::<InitPosition>();
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
                commands.entity(entity).insert((
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
                ));

                commands.entity(entity).remove::<InitPosition>();
            });
        }
    }
}
