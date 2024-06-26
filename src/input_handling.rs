use bevy::prelude::*;
use bevy_cursor::CursorInfo;

use crate::Weapon;

use super::movements::YAxisMove;

use super::movements::XAxisMove;

use super::Player;

#[derive(Component)]
pub(crate) struct KeyboardControlled;

pub(crate) fn handle_input(
    mut commands: Commands,
    player: Query<Entity, (With<Player>, With<KeyboardControlled>)>,
    input: Res<Input<KeyCode>>,
) {
    if let Some(player) = player.iter().next() {
        if input.any_just_pressed([KeyCode::A, KeyCode::Left]) {
            commands.entity(player).insert(XAxisMove::Left);
        }
        if input.any_just_released([KeyCode::A, KeyCode::Left]) {
            if input.any_pressed([KeyCode::D, KeyCode::Right]) {
                commands.entity(player).insert(XAxisMove::Right);
            } else {
                commands.entity(player).remove::<XAxisMove>();
            }
        }

        if input.any_just_pressed([KeyCode::D, KeyCode::Right]) {
            commands.entity(player).insert(XAxisMove::Right);
        }
        if input.any_just_released([KeyCode::D, KeyCode::Right]) {
            if input.any_pressed([KeyCode::A, KeyCode::Left]) {
                commands.entity(player).insert(XAxisMove::Left);
            } else {
                commands.entity(player).remove::<XAxisMove>();
            }
        }

        if input.any_just_pressed([KeyCode::W, KeyCode::Up]) {
            commands.entity(player).insert(YAxisMove::Up);
        }
        if input.any_just_released([KeyCode::W, KeyCode::Up]) {
            if input.any_pressed([KeyCode::S, KeyCode::Down]) {
                commands.entity(player).insert(YAxisMove::Down);
            } else {
                commands.entity(player).remove::<YAxisMove>();
            }
        }

        if input.any_just_pressed([KeyCode::S, KeyCode::Down]) {
            commands.entity(player).insert(YAxisMove::Down);
        }
        if input.any_just_released([KeyCode::S, KeyCode::Down]) {
            if input.any_pressed([KeyCode::W, KeyCode::Up]) {
                commands.entity(player).insert(YAxisMove::Up);
            } else {
                commands.entity(player).remove::<YAxisMove>();
            }
        }
    }
}

use super::IsShooting;

use super::Aims;

pub(crate) fn handle_mouse(
    mut commands: Commands,
    cursor: Res<CursorInfo>,
    mouse_click: Res<Input<MouseButton>>,
    player: Query<Entity, (With<Player>, With<KeyboardControlled>)>,
    weapons: Query<(Entity, &Weapon), With<Player>>,
) {
    if let Some(player) = player.iter().next() {
        let Some(target) = cursor.position() else {
            return;
        };
        commands.entity(player).insert(Aims(target));

        if mouse_click.just_pressed(MouseButton::Left) {
            weapons.iter().for_each(|(e, _)| {
                commands.entity(e).insert(IsShooting);
            });
            // commands.entity(player).insert(IsShooting);
        }
        if mouse_click.just_released(MouseButton::Left) {
            weapons.iter().for_each(|(e, _)| {
                commands.entity(e).remove::<IsShooting>();
            });
            // commands.entity(player).remove::<IsShooting>();
        }
    }
}

