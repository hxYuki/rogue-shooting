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

