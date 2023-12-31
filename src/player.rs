use bevy::{
    core_pipeline::Skybox, input::mouse::MouseMotion, prelude::*, render::view::ColorGrading,
};
use bevy_rapier3d::{prelude::*, rapier::geometry::CollisionEventFlags};

use crate::{
    animation::Animation,
    damage::{Damage, Health, KillEvent},
    ui::UiResources,
    weapons::{floating::FloatingObject, Ammo, ShootEvent, WeaponAttackTimer},
    GameSettings, GlobalState, COLLISION_GROUP_ENEMY, COLLISION_GROUP_LEVEL,
    COLLISION_GROUP_PICKUP, COLLISION_GROUP_PLAYER, COLLISION_GROUP_PROJECTILES,
};

const PLAYER_HEALTH: i32 = 300;

const PLAYER_WEAPON_DEFAULT_TRANSLATION: Vec3 = Vec3::new(0.0, -0.8, -1.7);
const PLAYER_THROW_OFFSET_SCALE: f32 = 10.0;
const PLAYER_THROW_STRENGTH: f32 = 80.0;
const PLAYER_THROW_DAMAGE: i32 = 50;

const PLAYER_HUD_ANIMATION_SPEED: f32 = 5.0;
const PLAYER_HUD_ON_TRANSLATION: Vec3 = Vec3::new(0.0, 0.0, -0.45);
const PLAYER_HUD_OFF_TRANSLATION: Vec3 = Vec3::new(-0.5, -0.3, -1.5);
const PLAYER_HUD_OFF_ROTATION_Y: f32 = std::f32::consts::FRAC_PI_4;
const PLAYER_HUD_OFF_ROTATION_X: f32 = -std::f32::consts::FRAC_PI_8;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnTransition {
                from: GlobalState::AssetLoading,
                to: GlobalState::MainMenu,
            },
            init_resources,
        );

        app.add_systems(OnEnter(GlobalState::InGame), player_toggle_hud_off);
        app.add_systems(OnEnter(GlobalState::Paused), player_toggle_hud_on);
        app.add_systems(OnEnter(GlobalState::GameOver), player_toggle_hud_on);
        app.add_systems(OnEnter(GlobalState::GameWon), player_toggle_hud_on);

        app.add_systems(
            Update,
            (
                player_kills_reading,
                player_trigger_pause,
                player_shoot,
                player_pick_up_weapon,
                player_throw_weapon,
                player_update,
                player_move,
                player_camera_update,
                player_weapon_update,
            )
                .run_if(in_state(GlobalState::InGame)),
        );
    }
}

#[derive(Resource)]
pub struct PlayerResources {
    pub hud_tablet_mesh: Handle<Mesh>,
    pub hud_tablet_material: Handle<StandardMaterial>,
    pub hud_tablet_arm_mesh: Handle<Mesh>,
    pub hud_tablet_arm_material: Handle<StandardMaterial>,
}

#[derive(Component)]
pub struct Player {
    pub acceleration: f32,
    pub slow_down_rade: f32,
    pub max_movement_speed_squared: f32,
}

#[derive(Component)]
pub struct PlayerVelocity {
    pub was_input: bool,
    pub velocity: Vec3,
}

#[derive(Component)]
pub struct PlayerCamera {
    pub default_translation: Vec3,

    pub bounce_continue: bool,
    pub bounce_progress: f32,
    pub bounce_speed: f32,

    pub bounce_amplitude: f32,
    pub bounce_amplitude_modifier: f32,
    pub bounce_amplitude_modifier_speed: f32,
    pub bounce_amplitude_modifier_max: f32,
}

#[derive(Component)]
struct PlayerHud;

#[derive(Component)]
pub struct PlayerWeapon {
    pub default_translation: Vec3,

    pub bounce_continue: bool,
    pub bounce_progress: f32,
    pub bounce_speed: f32,
    pub bounce_amplitude: f32,
}

#[derive(Bundle)]
struct PlayerThrownWeapon {
    transform: Transform,
    collider: Collider,
    collision_groups: CollisionGroups,
    active_events: ActiveEvents,
    rigid_body: RigidBody,
    velocity: Velocity,
    damage: Damage,
}

impl PlayerThrownWeapon {
    fn new(
        weapon_global_transform: &GlobalTransform,
        camera_global_transform: &GlobalTransform,
    ) -> Self {
        Self {
            transform: Transform::from_translation(
                weapon_global_transform.translation()
                    + camera_global_transform.forward() * PLAYER_THROW_OFFSET_SCALE,
            ),
            collider: Collider::cuboid(0.6, 2.6, 0.3),
            collision_groups: CollisionGroups::new(
                COLLISION_GROUP_PROJECTILES,
                COLLISION_GROUP_LEVEL | COLLISION_GROUP_ENEMY,
            ),
            active_events: ActiveEvents::COLLISION_EVENTS,
            rigid_body: RigidBody::Dynamic,
            velocity: Velocity {
                linvel: camera_global_transform.forward() * PLAYER_THROW_STRENGTH,
                ..default()
            },
            damage: Damage {
                damage: PLAYER_THROW_DAMAGE,
            },
        }
    }
}

pub fn spawn_player(
    ui_resources: &UiResources,
    player_resources: &PlayerResources,
    skybox_image: Handle<Image>,
    commands: &mut Commands,
    mut transform: Transform,
) {
    transform.translation.z -= 0.5;
    let id = commands
        .spawn((
            TransformBundle::from_transform(transform),
            InheritedVisibility::VISIBLE,
            RigidBody::KinematicPositionBased,
            Collider::capsule(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 2.0), 1.0),
            CollisionGroups::new(
                COLLISION_GROUP_PLAYER,
                COLLISION_GROUP_LEVEL | COLLISION_GROUP_PROJECTILES | COLLISION_GROUP_PICKUP,
            ),
            ActiveCollisionTypes::KINEMATIC_STATIC | ActiveCollisionTypes::DYNAMIC_KINEMATIC,
            Player {
                acceleration: 50.0,
                slow_down_rade: 5.0,
                max_movement_speed_squared: 40.0,
            },
            PlayerVelocity {
                was_input: false,
                velocity: Vec3::default(),
            },
            Health {
                health: PLAYER_HEALTH,
            },
        ))
        .with_children(|builder| {
            builder
                .spawn((
                    Camera3dBundle {
                        transform: Transform::from_xyz(0.0, 0.0, 2.0)
                            .looking_at(Vec3::new(0.0, 1.0, 2.0), Vec3::Z),
                        color_grading: ColorGrading {
                            exposure: 0.0,
                            gamma: 1.0,
                            pre_saturation: 1.0,
                            post_saturation: 1.0,
                        },
                        ..default()
                    },
                    UiCameraConfig { show_ui: false },
                    Skybox(skybox_image),
                    PlayerCamera {
                        default_translation: Vec3::new(0.0, 0.0, 2.0),

                        bounce_continue: false,
                        bounce_progress: 0.0,
                        bounce_speed: 8.0,

                        bounce_amplitude: 0.2,
                        bounce_amplitude_modifier: 1.0,
                        bounce_amplitude_modifier_speed: 1.0,
                        bounce_amplitude_modifier_max: 2.0,
                    },
                ))
                .with_children(|builder| {
                    // Tablet
                    builder
                        .spawn((
                            PbrBundle {
                                mesh: player_resources.hud_tablet_mesh.clone(),
                                material: player_resources.hud_tablet_material.clone(),
                                transform: Transform::from_translation(PLAYER_HUD_ON_TRANSLATION),
                                ..default()
                            },
                            PlayerHud,
                        ))
                        .with_children(|builder| {
                            // UI window
                            builder.spawn((PbrBundle {
                                mesh: ui_resources.mesh.clone(),
                                material: ui_resources.material.clone(),
                                transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.06)),
                                ..default()
                            },));
                            // Tablet arm
                            builder.spawn((PbrBundle {
                                mesh: player_resources.hud_tablet_arm_mesh.clone(),
                                material: player_resources.hud_tablet_arm_material.clone(),
                                transform: Transform::from_translation(Vec3::new(-0.2, -0.3, -0.1))
                                    .with_rotation(Quat::from_rotation_z(
                                        -std::f32::consts::FRAC_PI_8,
                                    )),
                                ..default()
                            },));
                        });
                });

            // disabled camera for ui interaction
            builder.spawn((Camera3dBundle {
                transform: Transform::from_xyz(0.0, -20.0, 3.0)
                    .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Z),
                camera: Camera {
                    order: 99,
                    is_active: false,
                    ..default()
                },
                ..default()
            },));
        })
        .id();

    commands.entity(id).log_components();
}

fn init_resources(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let aspect_ration = 1280.0 / 720.0;
    let holder_width = 0.6;
    let holder_hight = holder_width / aspect_ration;
    let hud_tablet_mesh = meshes.add(shape::Box::new(holder_width, holder_hight, 0.1).into());
    let hud_tablet_material = materials.add(StandardMaterial {
        base_color: Color::GOLD,
        perceptual_roughness: 0.9,
        ..default()
    });

    let hud_tablet_arm_mesh = meshes.add(shape::Box::new(0.2, 1.0, 0.1).into());
    let hud_tablet_arm_material = materials.add(StandardMaterial {
        base_color: Color::GOLD,
        perceptual_roughness: 0.9,
        ..default()
    });

    commands.insert_resource(PlayerResources {
        hud_tablet_mesh,
        hud_tablet_material,
        hud_tablet_arm_mesh,
        hud_tablet_arm_material,
    })
}

fn player_trigger_pause(
    keys: Res<Input<KeyCode>>,
    mut global_state: ResMut<NextState<GlobalState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        global_state.set(GlobalState::Paused);
    }
}

fn player_toggle_hud_on(hud: Query<Entity, With<PlayerHud>>, mut commands: Commands) {
    let Ok(hud) = hud.get_single() else {
        return;
    };

    let target_transform = Transform::from_translation(PLAYER_HUD_ON_TRANSLATION);
    let initial_transform = Transform::from_translation(PLAYER_HUD_OFF_TRANSLATION).with_rotation(
        Quat::from_rotation_y(PLAYER_HUD_OFF_ROTATION_Y)
            * Quat::from_rotation_x(PLAYER_HUD_OFF_ROTATION_X),
    );

    let Some(mut e) = commands.get_entity(hud) else {
        return;
    };

    e.insert(Animation {
        animate_forward: true,
        animate_backward: false,
        animation_speed: PLAYER_HUD_ANIMATION_SPEED,
        progress: 0.0,
        initial_transform,
        target_transform,
    });
}

fn player_toggle_hud_off(hud: Query<Entity, With<PlayerHud>>, mut commands: Commands) {
    let Ok(hud) = hud.get_single() else {
        return;
    };

    let initial_transform = Transform::from_translation(PLAYER_HUD_ON_TRANSLATION);
    let target_transform = Transform::from_translation(PLAYER_HUD_OFF_TRANSLATION).with_rotation(
        Quat::from_rotation_y(PLAYER_HUD_OFF_ROTATION_Y)
            * Quat::from_rotation_x(PLAYER_HUD_OFF_ROTATION_X),
    );

    let Some(mut e) = commands.get_entity(hud) else {
        return;
    };

    e.insert(Animation {
        animate_forward: true,
        animate_backward: false,
        animation_speed: PLAYER_HUD_ANIMATION_SPEED,
        progress: 0.0,
        initial_transform,
        target_transform,
    });
}

fn player_kills_reading(
    mut player: Query<Entity, With<Player>>,
    mut kill_events: EventReader<KillEvent>,
    mut global_state: ResMut<NextState<GlobalState>>,
) {
    let Ok(player) = player.get_single_mut() else {
        return;
    };

    for kill_event in kill_events.read() {
        if kill_event.entity == player {
            global_state.set(GlobalState::GameOver);
        }
    }
}

fn player_pick_up_weapon(
    player: Query<Entity, With<Player>>,
    player_camera: Query<Entity, With<PlayerCamera>>,
    player_weapon: Query<Entity, With<PlayerWeapon>>,
    floating_objects: Query<(Entity, &Children), With<FloatingObject>>,
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
) {
    // if there is already a weapon, do nothing
    if player_weapon.get_single().is_ok() {
        return;
    }

    let Ok(player) = player.get_single() else {
        return;
    };

    let Ok(camera) = player_camera.get_single() else {
        return;
    };

    for collision_event in collision_events.read() {
        let (collider_1, collider_2, flags) = match collision_event {
            CollisionEvent::Started(c1, c2, f) => (c1, c2, f),
            CollisionEvent::Stopped(c1, c2, f) => (c1, c2, f),
        };

        if flags.contains(CollisionEventFlags::REMOVED)
            || !flags.contains(CollisionEventFlags::SENSOR)
        {
            return;
        }
        let (floating_object_entity, floating_object_children) = if collider_1 == &player {
            if let Ok(w) = floating_objects.get(*collider_2) {
                w
            } else {
                continue;
            }
        } else if collider_2 == &player {
            if let Ok(w) = floating_objects.get(*collider_1) {
                w
            } else {
                continue;
            }
        } else {
            continue;
        };

        let Some(mut floating_object_commands) = commands.get_entity(floating_object_entity) else {
            continue;
        };
        let weapon_entity = floating_object_children[0];

        floating_object_commands.remove_children(&[weapon_entity]);
        floating_object_commands.despawn();

        let Some(mut weapon_commands) = commands.get_entity(weapon_entity) else {
            continue;
        };
        weapon_commands.insert((
            PlayerWeapon {
                default_translation: PLAYER_WEAPON_DEFAULT_TRANSLATION,
                bounce_continue: false,
                bounce_progress: 0.0,
                bounce_speed: 4.0,
                bounce_amplitude: 0.08,
            },
            Transform::default().with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ));

        commands.entity(camera).add_child(weapon_entity);
    }
}

fn player_throw_weapon(
    keys: Res<Input<KeyCode>>,
    player_camera: Query<(Entity, &GlobalTransform), With<PlayerCamera>>,
    player_weapon_components: Query<(Entity, &GlobalTransform), With<PlayerWeapon>>,
    mut commands: Commands,
) {
    let Ok((camera, camera_global_transform)) = player_camera.get_single() else {
        return;
    };

    let Ok((weapon, weapon_global_transform)) = player_weapon_components.get_single() else {
        return;
    };

    if keys.just_pressed(KeyCode::F) {
        commands
            .get_entity(camera)
            .unwrap()
            .remove_children(&[weapon]);

        commands
            .get_entity(weapon)
            .unwrap()
            .remove::<PlayerWeapon>()
            .insert(PlayerThrownWeapon::new(
                weapon_global_transform,
                camera_global_transform,
            ));
    }
}

fn player_shoot(
    keys: Res<Input<KeyCode>>,
    player_camera: Query<&GlobalTransform, With<PlayerCamera>>,
    mut player_weapon_components: Query<
        (Entity, &GlobalTransform, &mut WeaponAttackTimer, &mut Ammo),
        With<PlayerWeapon>,
    >,
    mut shoot_event: EventWriter<ShootEvent>,
) {
    let Ok(camera_global_transform) = player_camera.get_single() else {
        return;
    };

    let Ok((weapon_entity, weapon_global_transform, mut weapon_attack_timer, mut ammo)) =
        player_weapon_components.get_single_mut()
    else {
        return;
    };

    if keys.pressed(KeyCode::Space) && weapon_attack_timer.ready && ammo.ammo != 0 {
        weapon_attack_timer.attack_timer.reset();
        weapon_attack_timer.ready = false;
        ammo.ammo -= 1;
        shoot_event.send(ShootEvent {
            weapon_entity,
            weapon_translation: weapon_global_transform.translation(),
            direction: camera_global_transform.forward(),
        });
    }
}

fn player_update(
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
    player_camera_components: Query<&Transform, With<PlayerCamera>>,
    mut player_components: Query<(&Player, &mut PlayerVelocity)>,
) {
    let Ok((player, mut velocity)) = player_components.get_single_mut() else {
        return;
    };

    let Ok(camera_transform) = player_camera_components.get_single() else {
        return;
    };

    // slow down
    let velocity_copy = velocity.velocity;
    velocity.velocity -= velocity_copy * player.slow_down_rade * time.delta_seconds();

    let forward = camera_transform.forward();
    let right = forward.cross(Vec3::Z);

    let mut movement = Vec3::ZERO;
    if keys.pressed(KeyCode::W) {
        movement += forward;
    }
    if keys.pressed(KeyCode::S) {
        movement -= forward;
    }
    if keys.pressed(KeyCode::A) {
        movement -= right;
    }
    if keys.pressed(KeyCode::D) {
        movement += right;
    }

    movement.z = 0.0;
    if movement == Vec3::ZERO {
        velocity.was_input = false;
        return;
    }

    movement = movement.normalize();
    velocity.velocity = movement * player.acceleration * time.delta_seconds();
    let velocity_length = velocity
        .velocity
        .length_squared()
        .max(player.max_movement_speed_squared);
    velocity.velocity = velocity.velocity.normalize() * velocity_length;
    velocity.was_input = true;
}

fn player_move(
    time: Res<Time>,
    rapier_context: Res<RapierContext>,
    mut player_components: Query<
        (
            Entity,
            &Collider,
            &CollisionGroups,
            &PlayerVelocity,
            &mut Transform,
        ),
        With<Player>,
    >,
) {
    let Ok((player, collider, collision_groups, velocity, mut transform)) =
        player_components.get_single_mut()
    else {
        return;
    };

    let mut movement = velocity.velocity * time.delta_seconds();

    for i in 0..4 {
        let shape = collider;
        let shape_pos = transform.translation + movement;
        let shape_rot = transform.rotation;
        let shape_vel = movement;
        let max_toi = 2.0;
        let filter = QueryFilter {
            flags: QueryFilterFlags::EXCLUDE_SENSORS | QueryFilterFlags::EXCLUDE_DYNAMIC,
            groups: Some(*collision_groups),
            exclude_collider: Some(player),
            ..default()
        };

        if let Some((_, hit)) = rapier_context.cast_shape(
            shape_pos, shape_rot, shape_vel, shape, max_toi, true, filter,
        ) {
            match hit.status {
                TOIStatus::Converged => {
                    if i == 3 {
                        movement = Vec3::ZERO;
                        break;
                    }
                    // hit.normal1: indicates the normal at the contact point hit.witness1,
                    // expressed in the local-space of the collider hit by the shape.
                    let wall_parrallel = hit.details.unwrap().normal1.cross(Vec3::Z);
                    movement = wall_parrallel * wall_parrallel.dot(movement);
                }
                TOIStatus::Penetrating => {
                    return;
                }
                _ => {}
            }
        }
    }

    transform.translation += movement;
}

// TODO make better
fn player_camera_update(
    time: Res<Time>,
    game_settings: Res<GameSettings>,
    player_components: Query<&PlayerVelocity>,
    mut ev_motion: EventReader<MouseMotion>,
    mut player_camera_components: Query<(&mut PlayerCamera, &mut Transform)>,
) {
    let Ok(velocity) = player_components.get_single() else {
        return;
    };

    let Ok((mut camera, mut transform)) = player_camera_components.get_single_mut() else {
        return;
    };

    let rotation: f32 = ev_motion.read().map(|e| -e.delta.x).sum();
    transform.rotate_z(rotation * time.delta_seconds() * game_settings.camera_sensitivity);

    transform.translation = camera.default_translation
        + Vec3::NEG_Z
            * camera.bounce_amplitude
            * camera.bounce_amplitude_modifier
            * (camera.bounce_progress).sin();

    if velocity.was_input {
        // if there was input, continue bouncing
        camera.bounce_continue = true;
        camera.bounce_progress += camera.bounce_speed * time.delta_seconds();
        camera.bounce_amplitude_modifier = (camera.bounce_amplitude_modifier
            + camera.bounce_amplitude_modifier_speed * time.delta_seconds())
        .min(camera.bounce_amplitude_modifier_max);
    } else if camera.bounce_continue {
        // if there was no input, continue until next PI
        camera.bounce_progress += camera.bounce_speed * time.delta_seconds();
        let next_pi = (camera.bounce_progress / std::f32::consts::PI).ceil() * std::f32::consts::PI;
        if next_pi <= camera.bounce_progress + 0.1 {
            camera.bounce_progress = 0.0;
            camera.bounce_continue = false;
            camera.bounce_amplitude_modifier = 1.0;
        }
    }
}

// TODO make better
fn player_weapon_update(
    time: Res<Time>,
    player_velocity: Query<&PlayerVelocity>,
    mut weapon: Query<(&mut Transform, &mut PlayerWeapon)>,
) {
    let Ok(velocity) = player_velocity.get_single() else {
        return;
    };

    let Ok((mut weapon_transform, mut player_weapon)) = weapon.get_single_mut() else {
        return;
    };
    // weapon_transform.rotation = Quat::IDENTITY;

    let bounce = player_weapon.bounce_progress.sin();
    let offset = Vec3::new(
        player_weapon.bounce_amplitude * bounce,
        (player_weapon.bounce_amplitude * bounce).abs(),
        0.0,
    );

    weapon_transform.translation = player_weapon.default_translation + offset;

    if velocity.was_input {
        // if there was input, continue bouncing
        player_weapon.bounce_continue = true;
        player_weapon.bounce_progress += player_weapon.bounce_speed * time.delta_seconds();
    } else if player_weapon.bounce_continue {
        // if there was no input, continue until next PI
        player_weapon.bounce_progress += player_weapon.bounce_speed * time.delta_seconds();
        let next_pi =
            (player_weapon.bounce_progress / std::f32::consts::PI).ceil() * std::f32::consts::PI;
        if next_pi <= player_weapon.bounce_progress + 0.1 {
            player_weapon.bounce_progress = 0.0;
            player_weapon.bounce_continue = false;
        }
    }
}
