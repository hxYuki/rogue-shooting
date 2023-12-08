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

use bevy_cursor::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rand::prelude::*;
use bevy_xpbd_2d::prelude::*;
use enemies::{normal_enemy, EnemyType};
use enemy_targeting::{AimTargetingType, MoveTargetingType};
use levels::*;
use rand::prelude::*;

pub(crate) mod bullets;
mod constants;
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
            CursorInfoPlugin,
        ))
        .insert_resource(Gravity(Vec2::ZERO))
        .register_type::<bullets::lane_shot::LaneShot>()
        .register_type::<bullets::explode_shot::ExplodeShot>()
        .register_type::<bullets::splash_shot::SplashShot>()
        .register_type::<bullets::lazer_shot::LazerShot>()
        // register for Character entity components
        .register_type::<Player>()
        .register_type::<Enemy>()
        .register_type::<Character>()
        .register_type::<Life>()
        .register_type::<Movable>()
        .register_type::<Bullet>()
        .add_event::<bullets::BulletSpawnEvent>()
        .add_event::<bullets::BulletSucceedEvent>()
        .add_event::<BulletHitEvent>()
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
                bullets::lazer_shot::lazer_shot_move_system,
                bullets::lazer_shot::lazer_shot_bullet_initializer,
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
            (levels::level_enemy_spawner, levels::level_boss_spawner),
        )
        .add_systems(Update, (enemies::normal_enemy::normal_enemy_initializer))
        .add_systems(
            Update,
            (
                enemy_targeting::move_targeting_system,
                enemy_targeting::aim_targeting_system,
            )
                .before(life_dies_system),
        )
        .insert_resource(Time::<Fixed>::from_hz(
            constants::GAME_FIXED_TICK_PER_SECOND,
        ))
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

fn spawn_player(mut commands: Commands, mut global_entropy: ResMut<GlobalEntropy<WyRand>>) {
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
                    loads: vec![(
                        Bullet {
                            life_time: 0.03,
                            endurance: INFINITY,
                            speed: 1000.,
                            cooldown: 0.,
                            damage: 60. / constants::GAME_FIXED_TICK_PER_SECOND as f32,
                            hit_limit: INFINITY,
                        },
                        Box::new(bullets::lazer_shot::LazerShot {
                            width: 4.,
                            length: 900.,
                        }),
                    )],
                },
                // Weapon {
                //     accelerate: 1000.,
                //     loads: vec![
                //         (
                //             Bullet {
                //                 life_time: 0.3,
                //                 endurance: 1.,
                //                 speed: 1000.,
                //                 cooldown: 0.6,
                //                 damage: 10.,
                //                 hit_limit: 1,.
                //             },
                //             Box::new(bullets::splash_shot::SplashShot {
                //                 count: 1,
                //                 angle: 0.3,
                //             }),
                //         ),
                //         (
                //             Bullet {
                //                 life_time: 0.3,
                //                 endurance: INFINITY,
                //                 speed: 1000.,
                //                 cooldown: 1.,
                //                 damage: 80.,
                //                 hit_limit: 1,.
                //             },
                //             Box::new(bullets::explode_shot::ExplodeShot),
                //         ),
                //     ],
                // },
                Player,
            ));
        });

    spawn_level(commands, global_entropy);
}

fn spawn_level(mut commands: Commands, mut global_entropy: ResMut<GlobalEntropy<WyRand>>) {
    commands.spawn((
        LevelInfo {
            id: 0,
            enemy_to_spawn: vec![EnemyDescriptor {
                enemy: normal_enemy::NormalEnemy::TEXT.into(),
                class: EnemyClass::Normal(1),
                amount: 10,
            }],
            is_spawning: true,
            wave_enemy_limit: 3,
        },
        NextSpawnTimer(Timer::from_seconds(1., TimerMode::Repeating)),
        global_entropy.fork_rng(),
    ));
    // commands.spawn((
    //     SpriteBundle {
    //         sprite: Sprite {
    //             color: Color::rgb(1.0, 1.0, 1.0),
    //             rect: Some(Rect {
    //                 min: Vec2::new(0.0, 0.0),
    //                 max: Vec2::new(32.0, 32.0),
    //             }),
    //             ..Default::default()
    //         },
    //         transform: Transform::from_translation(Vec3::new(0., 100., 0.)),
    //         ..Default::default()
    //     },
    //     Life(100),
    //     Enemy,
    //     Character,
    //     Aims(Vec2::ZERO),
    //     Movable { speed: 150.0 },
    //     Collider::ball(16.),
    //     // Sensor,
    //     CollisionLayers::new([Layer::Enemy], [Layer::Player, Layer::PlayerBullet]),
    //     AimTargetingType::AimCurrent,
    //     MoveTargetingType::Chase,
    // ));
}

mod enemies {
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
                        Movable { speed: 150.0 },
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
}

mod levels {
    use super::*;
    use bevy::prelude::*;
    use enemies::EnemyType;
    use std::{
        hash::{Hash, Hasher},
        time::Duration,
    };

    #[derive(Component)]
    pub struct LevelInfo {
        pub id: i32,
        pub is_spawning: bool,
        pub enemy_to_spawn: Vec<EnemyDescriptor>,
        pub wave_enemy_limit: usize,
    }

    #[derive(Component)]
    pub struct CurrentWave(usize);

    #[derive(Component)]
    pub struct NextSpawnTimer(pub Timer);

    #[derive(Component)]
    pub struct SpawnedCounter(HashMap<EnemyDescriptor, u32>);

    #[derive(PartialEq)]
    pub enum EnemyClass {
        /// Normal enemies will be spawned with pace according to existing enemies
        /// the parameter implies its strength
        Normal(u32),
        /// Bosses in level will be spawned together at the end
        Boss,
    }
    #[derive(Component)]
    pub struct BossClass;
    #[derive(Component)]
    pub struct NormalClass(u32);

    pub struct EnemyDescriptor {
        pub enemy: String,
        pub class: EnemyClass,
        pub amount: u32,
    }
    impl Hash for EnemyDescriptor {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.enemy.hash(state);
        }
    }
    #[derive(Component)]
    pub struct LevelRef(Entity);

    const LEVEL_TIME_BASE: f32 = 60.;
    pub fn level_enemy_spawner(
        mut commands: Commands,
        mut levels: Query<(
            Entity,
            &mut LevelInfo,
            &mut NextSpawnTimer,
            &mut EntropyComponent<WyRand>,
        )>,
        level_enemies: Query<(&LevelRef), With<Enemy>>,
        time: Res<Time>,
    ) {
        levels.for_each_mut(|(entity, mut level, mut spawn_timer, mut entropy)| {
            // FIXME: remove this check, is_spawning shuould be a component
            if !level.is_spawning {
                return;
            }
            if spawn_timer.0.tick(time.delta()).just_finished() {
                // let remains = counter.0.iter().map(|e| e.1).sum::<u32>() as i64;
                let remains = level
                    .enemy_to_spawn
                    .iter()
                    .filter(|e| e.class != EnemyClass::Boss)
                    .map(|e| e.amount)
                    .sum::<u32>() as i64;
                if remains == 0 {
                    return;
                }

                let mut next = entropy.gen_range(0..remains);
                for desc in level.enemy_to_spawn.iter_mut() {
                    next -= desc.amount as i64;
                    if next < 0 {
                        desc.amount -= 1;

                        let EnemyClass::Normal(class) = desc.class else {
                            unreachable!()
                        };
                        // TODO: scene range
                        let rand_transform = Transform::from_xyz(
                            entropy.gen_range(-300. ..=300.),
                            entropy.gen_range(-300. ..=300.),
                            0.,
                        );

                        commands.spawn((
                            enemies::type_dispatch(&desc.enemy).to_component(),
                            NormalClass(class),
                            Enemy,
                            Character,
                            InitPosition(rand_transform),
                            LevelRef(entity),
                        ));
                        break;
                    }
                }
            }

            let level_enemy_count = level_enemies
                .iter()
                .filter(|LevelRef(level_et)| *level_et == entity)
                .count();
            match level_enemy_count {
                c if c < level.wave_enemy_limit => {
                    if spawn_timer.0.duration() != Duration::from_secs_f32(0.6) {
                        spawn_timer.0.reset();
                        spawn_timer.0.set_duration(Duration::from_secs_f32(0.6));
                    }
                }
                c if c >= level.wave_enemy_limit && c < 2 * level.wave_enemy_limit => {
                    if spawn_timer.0.duration() != Duration::from_secs_f32(2.) {
                        spawn_timer.0.reset();
                        spawn_timer.0.set_duration(Duration::from_secs_f32(2.));
                    }
                }
                _ => {
                    if spawn_timer.0.duration() != Duration::from_secs_f32(6.) {
                        spawn_timer.0.reset();
                        spawn_timer.0.set_duration(Duration::from_secs_f32(6.));
                    }
                }
            };
        });
    }

    #[derive(Component)]
    pub struct BossSpawnTimer(Timer);
    pub fn level_boss_spawner(
        mut commands: Commands,
        mut levels: Query<(Entity, &mut LevelInfo, Option<&mut BossSpawnTimer>)>,
        time: Res<Time>,
    ) {
        levels.for_each_mut(|(entity, mut level, boss_timer)| {
            if let Some(mut boss_timer) = boss_timer {
                if boss_timer.0.tick(time.delta()).just_finished() {
                    // TODO: spawn all bosses
                }
            } else if level
                .enemy_to_spawn
                .iter()
                .any(|e| e.class == EnemyClass::Boss)
                && level.enemy_to_spawn.iter().map(|e| e.amount).sum::<u32>() == 0
            {
                commands
                    .entity(entity)
                    .insert(BossSpawnTimer(Timer::from_seconds(10., TimerMode::Once)));
            }
        });
    }
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

#[derive(Component, Reflect, Clone, Copy)]
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

                        if count as f32 <= bullet.hit_limit {
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
