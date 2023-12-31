use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioSource};
use bevy_rapier3d::prelude::*;

use crate::{
    animation::Animation, damage::Damage, level::LevelObject, GlobalState, COLLISION_GROUP_ENEMY,
    COLLISION_GROUP_LEVEL, COLLISION_GROUP_PLAYER, COLLISION_GROUP_PROJECTILES,
};

use self::floating::{FloatingObjectBundle, FloatingObjectInternal};

pub mod floating;

const DEFAULT_PROJECTILE_SIZE: f32 = 0.125;
const DEFAULT_CLIP_SIZE: f32 = 0.01;
const DEFAULT_CLIP_LENGTH: f32 = 0.02;

// Pistol
const PISTOL_AMMO: u32 = 20;
const PISTOL_DAMAGE: i32 = 10;
const PISTOL_ATTACK_SPEED: f32 = 1.0 / 4.0;
const PISTOL_PROJECTILE_VELOCITY: f32 = 500.0;
const PISTOL_PROJECTILE_OFFSET_SCALE: f32 = 2.0;

// Needs to be bigger that (1 / attack_speed) * 2
// because animatino played for 2 directions
const PISTOL_ANIMATION_SPEED: f32 = 10.0;
const PISTOL_ANIMATION_FORWARD: bool = true;
const PISTOL_ANIMATION_BACKWARD: bool = true;
const PISTOL_ANIMATION_TARGET_OFFSET: Vec3 = Vec3::new(0.2, 0.2, 0.0);
const PISTOL_ANIMATION_TARGET_ROTATION_X: f32 = std::f32::consts::FRAC_PI_8;
const PISTOL_ANIMATION_TARGET_ROTATION_Y: f32 = 0.0;
const PISTOL_SHELL_INITIAL_VELOCITY: f32 = 10.0;

// Shotgun
const SHOTGUN_AMMO: u32 = 10;
const SHOTGUN_DAMAGE: i32 = 5;
const SHOTGUN_ATTACK_SPEED: f32 = 1.0 / 1.2;
const SHOTGUN_PROJECTILE_VELOCITY: f32 = 500.0;
const SHOTGUN_PROJECTILE_OFFSET_SCALE: f32 = 2.2;

// Needs to be bigger that (1 / attack_speed) * 2
// because animatino played for 2 directions
const SHOTGUN_ANIMATION_SPEED: f32 = 5.0;
const SHOTGUN_ANIMATION_FORWARD: bool = true;
const SHOTGUN_ANIMATION_BACKWARD: bool = true;
const SHOTGUN_ANIMATION_TARGET_OFFSET: Vec3 = Vec3::new(0.2, 0.2, 0.0);
const SHOTGUN_ANIMATION_TARGET_ROTATION_X: f32 = std::f32::consts::FRAC_PI_8;
const SHOTGUN_ANIMATION_TARGET_ROTATION_Y: f32 = 0.0;
const SHOTGUN_SHELL_INITIAL_VELOCITY: f32 = 10.0;

// Minigun
const MINIGUN_AMMO: u32 = 50;
const MINIGUN_DAMAGE: i32 = 10;
const MINIGUN_ATTACK_SPEED: f32 = 1.0 / 8.0;
const MINIGUN_PROJECTILE_VELOCITY: f32 = 500.0;
const MINIGUN_PROJECTILE_OFFSET_SCALE: f32 = 3.0;

// Needs to be bigger that (1 / attack_speed)
const MINIGUN_ANIMATION_SPEED: f32 = 9.0;
const MINIGUN_ANIMATION_FORWARD: bool = true;
const MINIGUN_ANIMATION_BACKWARD: bool = false;
const MINIGUN_ANIMATION_TARGET_OFFSET: Vec3 = Vec3::ZERO;
const MINIGUN_ANIMATION_TARGET_ROTATION_X: f32 = 0.0;
const MINIGUN_ANIMATION_TARGET_ROTATION_Y: f32 = std::f32::consts::FRAC_PI_2;
const MINIGUN_SHELL_INITIAL_VELOCITY: f32 = 10.0;

pub struct WeaponsPlugin;

impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        app.add_collection_to_loading_state::<_, WeaponAssets>(GlobalState::AssetLoading);

        app.add_event::<ShootEvent>();

        app.add_plugins(floating::FloatingPlugin);

        app.add_systems(
            Update,
            (update_attack_timers, weapon_shoot).run_if(in_state(GlobalState::InGame)),
        );
    }
}

#[derive(AssetCollection, Resource)]
pub struct WeaponAssets {
    #[asset(path = "pistol/pistol.glb#Scene0")]
    pub pistol_scene: Handle<Scene>,
    #[asset(path = "pistol/pistol_shell.glb#Scene0")]
    pub pistol_shell_scene: Handle<Scene>,
    #[asset(path = "pistol/pistol.wav")]
    pub pistol_sound: Handle<AudioSource>,

    #[asset(path = "shotgun/shotgun.glb#Scene0")]
    pub shotgun_scene: Handle<Scene>,
    #[asset(path = "shotgun/shotgun_shell.glb#Scene0")]
    pub shotgun_shell_scene: Handle<Scene>,
    #[asset(path = "shotgun/shotgun.wav")]
    pub shotgun_sound: Handle<AudioSource>,

    #[asset(path = "minigun/minigun.glb#Scene0")]
    pub minigun_scene: Handle<Scene>,
    #[asset(path = "minigun/minigun_shell.glb#Scene0")]
    pub minigun_shell_scene: Handle<Scene>,
    #[asset(path = "minigun/minigun.wav")]
    pub minigun_sound: Handle<AudioSource>,

    #[asset(path = "round.glb#Scene0")]
    pub round_scene: Handle<Scene>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum WeaponType {
    #[default]
    Pistol,
    Shotgun,
    Minigun,
}

#[derive(Default, Component)]
pub struct Weapon {
    weapon_type: WeaponType,
}

#[derive(Component)]
pub struct WeaponModel;

#[derive(Default, Component)]
pub struct Ammo {
    pub ammo: u32,
}

#[derive(Event)]
pub struct ShootEvent {
    pub weapon_entity: Entity,
    pub weapon_translation: Vec3,
    pub direction: Vec3,
}

#[derive(Component)]
pub struct WeaponAttackTimer {
    pub attack_timer: Timer,
    pub ready: bool,
}

impl WeaponAttackTimer {
    pub fn new(seconds: f32) -> Self {
        Self {
            attack_timer: Timer::new(
                std::time::Duration::from_secs_f32(seconds),
                TimerMode::Repeating,
            ),
            ready: false,
        }
    }
}

#[derive(Bundle)]
pub struct WeaponBundle {
    pub transform_bundle: TransformBundle,
    pub inherited_visibility: InheritedVisibility,
    pub ammo: Ammo,
    pub weapon_attack_timer: WeaponAttackTimer,
    pub weapon: Weapon,
}

impl WeaponBundle {
    pub fn pistol(transform: Transform) -> Self {
        Self {
            transform_bundle: TransformBundle::from_transform(transform),
            inherited_visibility: InheritedVisibility::VISIBLE,
            ammo: Ammo { ammo: PISTOL_AMMO },
            weapon_attack_timer: WeaponAttackTimer::new(PISTOL_ATTACK_SPEED),
            weapon: Weapon {
                weapon_type: WeaponType::Pistol,
            },
        }
    }

    pub fn shotgun(transform: Transform) -> Self {
        Self {
            transform_bundle: TransformBundle::from_transform(transform),
            inherited_visibility: InheritedVisibility::VISIBLE,
            ammo: Ammo { ammo: SHOTGUN_AMMO },
            weapon_attack_timer: WeaponAttackTimer::new(SHOTGUN_ATTACK_SPEED),
            weapon: Weapon {
                weapon_type: WeaponType::Shotgun,
            },
        }
    }

    pub fn minigun(transform: Transform) -> Self {
        Self {
            transform_bundle: TransformBundle::from_transform(transform),
            inherited_visibility: InheritedVisibility::VISIBLE,
            ammo: Ammo { ammo: MINIGUN_AMMO },
            weapon_attack_timer: WeaponAttackTimer::new(MINIGUN_ATTACK_SPEED),
            weapon: Weapon {
                weapon_type: WeaponType::Minigun,
            },
        }
    }
}

impl Default for WeaponBundle {
    fn default() -> Self {
        Self {
            transform_bundle: TransformBundle::default(),
            inherited_visibility: InheritedVisibility::VISIBLE,
            ammo: Ammo::default(),
            weapon_attack_timer: WeaponAttackTimer::new(0.0),
            weapon: Weapon::default(),
        }
    }
}

#[derive(Default, Component)]
pub struct Projectile {
    pub direction: Vec3,
}

#[derive(Bundle)]
pub struct ProjectileBundle {
    pub scene_bundle: SceneBundle,
    pub rigid_body: RigidBody,
    pub collider: Collider,
    pub collision_groups: CollisionGroups,
    pub active_events: ActiveEvents,
    pub velocity: Velocity,
    pub projectile: Projectile,
    pub damage: Damage,

    pub level_object: LevelObject,
}

impl Default for ProjectileBundle {
    fn default() -> Self {
        Self {
            scene_bundle: SceneBundle::default(),
            rigid_body: RigidBody::Dynamic,
            collider: Collider::default(),
            collision_groups: CollisionGroups::new(
                COLLISION_GROUP_PROJECTILES,
                COLLISION_GROUP_LEVEL | COLLISION_GROUP_PLAYER | COLLISION_GROUP_ENEMY,
            ),
            active_events: ActiveEvents::COLLISION_EVENTS,
            velocity: Velocity::default(),
            projectile: Projectile::default(),
            damage: Damage::default(),

            level_object: LevelObject,
        }
    }
}

#[derive(Bundle)]
pub struct ShellBundle {
    pub scene_bundle: SceneBundle,
    pub rigid_body: RigidBody,
    pub collider: Collider,
    pub velocity: Velocity,
    pub friction: Friction,

    pub level_object: LevelObject,
}

impl Default for ShellBundle {
    fn default() -> Self {
        Self {
            scene_bundle: SceneBundle::default(),
            rigid_body: RigidBody::Dynamic,
            collider: Collider::cuboid(DEFAULT_CLIP_LENGTH, DEFAULT_CLIP_SIZE, DEFAULT_CLIP_SIZE),
            velocity: Velocity::default(),
            friction: Friction {
                coefficient: 100.0,
                ..default()
            },

            level_object: LevelObject,
        }
    }
}

macro_rules! attach_weapon {
    ($commands:ident, $weapon_assets:ident, $transform:ident, $bundle_fn:ident, $asset:ident) => {
        $commands
            .spawn(WeaponBundle::$bundle_fn($transform))
            .with_children(|builder| {
                builder.spawn((
                    SceneBundle {
                        scene: $weapon_assets.$asset.clone(),
                        ..default()
                    },
                    WeaponModel,
                ));
            })
    };
}
pub(crate) use attach_weapon;

pub fn spawn_weapon(
    weapon_assets: &WeaponAssets,
    weapon_type: WeaponType,
    commands: &mut Commands,
    transform: Transform,
) {
    match weapon_type {
        WeaponType::Pistol => {
            commands
                .spawn((FloatingObjectBundle::new(transform.translation),))
                .with_children(|builder| {
                    let transform = Transform::default();
                    _ = attach_weapon!(builder, weapon_assets, transform, pistol, pistol_scene)
                        .insert(FloatingObjectInternal);
                });
        }

        WeaponType::Shotgun => {
            commands
                .spawn((FloatingObjectBundle::new(transform.translation),))
                .with_children(|builder| {
                    let transform = Transform::default();
                    _ = attach_weapon!(builder, weapon_assets, transform, shotgun, shotgun_scene)
                        .insert(FloatingObjectInternal);
                });
        }
        WeaponType::Minigun => {
            commands
                .spawn((FloatingObjectBundle::new(transform.translation),))
                .with_children(|builder| {
                    let transform = Transform::default();
                    _ = attach_weapon!(builder, weapon_assets, transform, minigun, minigun_scene)
                        .insert(FloatingObjectInternal);
                });
        }
    }
}

fn update_attack_timers(time: Res<Time>, mut timers: Query<&mut WeaponAttackTimer>) {
    for mut timer in timers.iter_mut() {
        if !timer.ready {
            timer.attack_timer.tick(time.delta());
            if timer.attack_timer.finished() {
                timer.ready = true;
            }
        }
    }
}

fn weapon_shoot(
    audio: Res<Audio>,
    weapon_assets: Res<WeaponAssets>,
    weapons: Query<(&Weapon, &Children)>,
    weapon_models: Query<&Transform, With<WeaponModel>>,
    mut commands: Commands,
    mut shoot_event: EventReader<ShootEvent>,
) {
    for e in shoot_event.read() {
        if let Ok((weapon, weapon_children)) = weapons.get(e.weapon_entity) {
            match weapon.weapon_type {
                WeaponType::Pistol => pistol_shoot(
                    audio.as_ref(),
                    weapon_assets.as_ref(),
                    &weapon_models,
                    weapon_children,
                    e,
                    &mut commands,
                ),
                WeaponType::Shotgun => shotgun_shoot(
                    audio.as_ref(),
                    weapon_assets.as_ref(),
                    &weapon_models,
                    weapon_children,
                    e,
                    &mut commands,
                ),
                WeaponType::Minigun => minigun_shoot(
                    audio.as_ref(),
                    weapon_assets.as_ref(),
                    &weapon_models,
                    weapon_children,
                    e,
                    &mut commands,
                ),
            }
        }
    }
}

fn pistol_shoot(
    audio: &Audio,
    weapon_assets: &WeaponAssets,
    weapon_models: &Query<&Transform, With<WeaponModel>>,
    weapon_children: &Children,
    event: &ShootEvent,
    commands: &mut Commands,
) {
    let right = event.direction.cross(Vec3::Z);

    // spawn projectiles
    let mut projectile_angle = event.direction.angle_between(Vec3::Y);
    if event.direction.cross(Vec3::Y).z >= 0.0 {
        projectile_angle *= -1.0;
    }
    let projectile_rotation = Quat::from_rotation_z(projectile_angle);
    let projectile_translation =
        event.weapon_translation + event.direction * PISTOL_PROJECTILE_OFFSET_SCALE;
    commands.spawn(ProjectileBundle {
        scene_bundle: SceneBundle {
            scene: weapon_assets.round_scene.clone(),
            transform: Transform::from_translation(projectile_translation)
                .with_rotation(projectile_rotation)
                .with_scale(Vec3::new(10.0, 10.0, 10.0)),
            ..default()
        },
        collider: Collider::ball(DEFAULT_PROJECTILE_SIZE),
        velocity: Velocity {
            linvel: event.direction * PISTOL_PROJECTILE_VELOCITY,
            ..default()
        },
        damage: Damage {
            damage: PISTOL_DAMAGE,
        },
        projectile: Projectile {
            direction: event.direction,
        },
        ..default()
    });

    // spawn shell
    let shell_direction = right + Vec3::Z;
    let mut shell_translation = event.weapon_translation;
    shell_translation += event.direction * 2.0;
    commands.spawn(ShellBundle {
        scene_bundle: SceneBundle {
            scene: weapon_assets.pistol_shell_scene.clone(),
            transform: Transform::from_translation(shell_translation)
                .with_scale(Vec3::new(2.0, 2.0, 2.0)),
            ..default()
        },
        velocity: Velocity {
            linvel: shell_direction * PISTOL_SHELL_INITIAL_VELOCITY,
            ..default()
        },
        ..default()
    });

    // start shooting animation
    let weapon_model = weapon_children[0];
    let Ok(weapon_model_transform) = weapon_models.get(weapon_model) else {
        return;
    };
    let initial_transform = *weapon_model_transform;
    let mut target_transform = initial_transform;
    target_transform.translation += PISTOL_ANIMATION_TARGET_OFFSET;
    target_transform.rotation *= Quat::from_rotation_x(PISTOL_ANIMATION_TARGET_ROTATION_X)
        * Quat::from_rotation_y(PISTOL_ANIMATION_TARGET_ROTATION_Y);
    let Some(mut e) = commands.get_entity(weapon_model) else {
        return;
    };
    e.insert(Animation {
        animate_forward: PISTOL_ANIMATION_FORWARD,
        animate_backward: PISTOL_ANIMATION_BACKWARD,
        animation_speed: PISTOL_ANIMATION_SPEED,
        progress: 0.0,
        initial_transform,
        target_transform,
    });

    // play sound
    audio.play(weapon_assets.pistol_sound.clone());
}

fn shotgun_shoot(
    audio: &Audio,
    weapon_assets: &WeaponAssets,
    weapon_models: &Query<&Transform, With<WeaponModel>>,
    weapon_children: &Children,
    event: &ShootEvent,
    commands: &mut Commands,
) {
    let right = event.direction.cross(Vec3::Z);

    // spawn projectiles
    let mut projectile_angle = event.direction.angle_between(Vec3::Y);
    if event.direction.cross(Vec3::Y).z >= 0.0 {
        projectile_angle *= -1.0;
    }
    let projectile_rotation = Quat::from_rotation_z(projectile_angle);
    let projectile_translation =
        event.weapon_translation + event.direction * SHOTGUN_PROJECTILE_OFFSET_SCALE;

    let left_barrel = projectile_translation - right / 2.0;
    let right_barrel = projectile_translation + right / 2.0;
    let offsets = [
        right / 3.0 + Vec3::Z / 3.0,
        -right / 3.0 + Vec3::Z / 3.0,
        right / 3.0 - Vec3::Z / 3.0,
        -right / 3.0 - Vec3::Z / 3.0,
    ];

    for barrel in [left_barrel, right_barrel] {
        for offset in offsets {
            let projectile_translation = barrel + offset;
            commands.spawn(ProjectileBundle {
                scene_bundle: SceneBundle {
                    scene: weapon_assets.round_scene.clone(),
                    transform: Transform::from_translation(projectile_translation)
                        .with_rotation(projectile_rotation)
                        .with_scale(Vec3::new(10.0, 10.0, 10.0)),
                    ..default()
                },
                collider: Collider::ball(DEFAULT_PROJECTILE_SIZE),
                velocity: Velocity {
                    linvel: event.direction * SHOTGUN_PROJECTILE_VELOCITY,
                    ..default()
                },
                damage: Damage {
                    damage: SHOTGUN_DAMAGE,
                },
                projectile: Projectile {
                    direction: event.direction,
                },
                ..default()
            });
        }
    }

    // spawn shell
    let shell_direction = right + Vec3::Z;
    let mut shell_translation = event.weapon_translation;
    shell_translation += event.direction * 2.0;

    let offsets = [-right / 2.0, right / 2.0];
    for offset in offsets {
        commands.spawn(ShellBundle {
            scene_bundle: SceneBundle {
                scene: weapon_assets.shotgun_shell_scene.clone(),
                transform: Transform::from_translation(shell_translation + offset)
                    .with_scale(Vec3::new(2.0, 2.0, 2.0)),
                ..default()
            },
            velocity: Velocity {
                linvel: shell_direction * SHOTGUN_SHELL_INITIAL_VELOCITY,
                ..default()
            },
            ..default()
        });
    }

    // start shooting animation
    let weapon_model = weapon_children[0];
    let Ok(weapon_model_transform) = weapon_models.get(weapon_model) else {
        return;
    };
    let initial_transform = *weapon_model_transform;
    let mut target_transform = initial_transform;
    target_transform.translation += SHOTGUN_ANIMATION_TARGET_OFFSET;
    target_transform.rotation *= Quat::from_rotation_x(SHOTGUN_ANIMATION_TARGET_ROTATION_X)
        * Quat::from_rotation_y(SHOTGUN_ANIMATION_TARGET_ROTATION_Y);
    let Some(mut e) = commands.get_entity(weapon_model) else {
        return;
    };
    e.insert(Animation {
        animate_forward: SHOTGUN_ANIMATION_FORWARD,
        animate_backward: SHOTGUN_ANIMATION_BACKWARD,
        animation_speed: SHOTGUN_ANIMATION_SPEED,
        progress: 0.0,
        initial_transform,
        target_transform,
    });

    // play sound
    audio.play(weapon_assets.shotgun_sound.clone());
}

fn minigun_shoot(
    audio: &Audio,
    weapon_assets: &WeaponAssets,
    weapon_models: &Query<&Transform, With<WeaponModel>>,
    weapon_children: &Children,
    event: &ShootEvent,
    commands: &mut Commands,
) {
    let right = event.direction.cross(Vec3::Z);

    // spawn projectiles
    let mut projectile_angle = event.direction.angle_between(Vec3::Y);
    if event.direction.cross(Vec3::Y).z >= 0.0 {
        projectile_angle *= -1.0;
    }
    let projectile_rotation = Quat::from_rotation_z(projectile_angle);
    let projectile_translation =
        event.weapon_translation + event.direction * MINIGUN_PROJECTILE_OFFSET_SCALE;

    let left_barrel = projectile_translation - right / 2.0;
    let right_barrel = projectile_translation + right / 2.0;

    for barrel in [left_barrel, right_barrel] {
        commands.spawn(ProjectileBundle {
            scene_bundle: SceneBundle {
                scene: weapon_assets.minigun_shell_scene.clone(),
                transform: Transform::from_translation(barrel)
                    .with_rotation(projectile_rotation)
                    .with_scale(Vec3::new(10.0, 10.0, 10.0)),
                ..default()
            },
            collider: Collider::ball(DEFAULT_PROJECTILE_SIZE),
            velocity: Velocity {
                linvel: event.direction * MINIGUN_PROJECTILE_VELOCITY,
                ..default()
            },
            damage: Damage {
                damage: MINIGUN_DAMAGE,
            },
            projectile: Projectile {
                direction: event.direction,
            },
            ..default()
        });
    }

    // spawn shell
    let shell_direction = right + Vec3::Z;
    let mut shell_translation = event.weapon_translation;
    shell_translation += event.direction * 2.0;

    let offsets = [-right / 2.0, right / 2.0];
    for offset in offsets {
        commands.spawn(ShellBundle {
            scene_bundle: SceneBundle {
                scene: weapon_assets.minigun_shell_scene.clone(),
                transform: Transform::from_translation(shell_translation + offset)
                    .with_scale(Vec3::new(2.0, 2.0, 2.0)),
                ..default()
            },
            velocity: Velocity {
                linvel: shell_direction * MINIGUN_SHELL_INITIAL_VELOCITY,
                ..default()
            },
            ..default()
        });
    }

    // start shooting animation
    let weapon_model = weapon_children[0];
    let Ok(weapon_model_transform) = weapon_models.get(weapon_model) else {
        return;
    };
    let initial_transform = *weapon_model_transform;
    let mut target_transform = initial_transform;
    target_transform.translation += MINIGUN_ANIMATION_TARGET_OFFSET;
    target_transform.rotation *= Quat::from_rotation_x(MINIGUN_ANIMATION_TARGET_ROTATION_X)
        * Quat::from_rotation_y(MINIGUN_ANIMATION_TARGET_ROTATION_Y);
    let Some(mut e) = commands.get_entity(weapon_model) else {
        return;
    };
    e.insert(Animation {
        animate_forward: MINIGUN_ANIMATION_FORWARD,
        animate_backward: MINIGUN_ANIMATION_BACKWARD,
        animation_speed: MINIGUN_ANIMATION_SPEED,
        progress: 0.0,
        initial_transform,
        target_transform,
    });

    // play sound
    audio.play(weapon_assets.minigun_sound.clone());
}
