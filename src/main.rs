use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::*;
use bevy::sprite::collide_aabb::collide;
use rand::Rng;

const SIMULATION_SPEED: f32 = 1.;
const MOVEMENT_SPEED: f32 = SIMULATION_SPEED * 150.;
const DAY_LENGTH: f32 = 10. / SIMULATION_SPEED;
const NIGHT_LENGTH: f32 = 2. / SIMULATION_SPEED;
const BASE_ENERGY_COST: f32 = 1. / (NIGHT_LENGTH + DAY_LENGTH) / MOVEMENT_SPEED * SIMULATION_SPEED;
const BASE_ENERGY: f32 = 1.;
const PERSON_COUNT: i32 = 10;
const FOOD_COUNT: i32 = 100;

struct Sunset(bool);
struct DayTimer(Timer);
struct NightTimer(Timer);

struct RandomizeDirections;
struct SpawnFood;
struct Reproduce(Transform);

#[derive(Component)]
struct Person;
#[derive(Component, Debug)]
struct Hungry;
#[derive(Component)]
struct Fertile;
#[derive(Component)]
struct Returning;
#[derive(Component)]
struct AtHome;
#[derive(Component)]
struct Dead;
#[derive(Component)]
struct Energy(f32);
#[derive(Component)]
struct Traits {
    speed: f32,
}

#[derive(Component)]
struct Food;
#[derive(Component, Debug)]
struct Eaten(bool);

fn get_random_location(window: &Window) -> Transform {
    let width = window.width();
    let height = window.height();

    let x = (rand::random::<f32>() - 0.5) * width;
    let y = (rand::random::<f32>() - 0.5) * height;

    Transform::from_xyz(x, y, 0.)
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    windows: Res<Windows>,
    mut ev_spawn_food: EventWriter<SpawnFood>,
    mut ev_randomize: EventWriter<RandomizeDirections>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    for _i in 0..PERSON_COUNT {
        commands
            .spawn_bundle(SpriteBundle {
                texture: asset_server.load("person1.png"),
                transform: get_random_location(windows.primary()),
                ..default()
            })
            .insert(Person)
            .insert(Hungry)
            .insert(Energy(BASE_ENERGY))
            .insert(Traits { speed: 1. });
    }
    ev_randomize.send(RandomizeDirections);

    ev_spawn_food.send(SpawnFood);
}

fn spawn_food(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    windows: Res<Windows>,
    mut events: EventReader<SpawnFood>,
) {
    for _event in events.iter() {
        for _i in 0..FOOD_COUNT {
            commands
                .spawn_bundle(SpriteBundle {
                    texture: asset_server.load("food1.png"),
                    transform: get_random_location(windows.primary()),
                    ..default()
                })
                .insert(Food)
                .insert(Eaten(false));
        }
    }
}

fn random_movement(
    time: Res<Time>,
    mut sprite: Query<
        (&mut Transform, &Traits, &mut Energy),
        (With<Person>, Without<Returning>, Without<Dead>),
    >,
    windows: Res<Windows>,
) {
    let mut rng = rand::thread_rng();

    for mut sprite in sprite.iter_mut() {
        let rotation_delta =
            Quat::from_rotation_z((rng.gen::<f32>() - 0.5) * 12. * time.delta_seconds());
        sprite.0.rotation *= rotation_delta;

        let rotation_rad = sprite.0.rotation.to_euler(EulerRot::ZYX).0;
        let distance = MOVEMENT_SPEED * sprite.1.speed * time.delta_seconds();
        let delta_x = distance * rotation_rad.cos();
        let delta_y = distance * rotation_rad.sin();
        let e = distance * sprite.1.speed * BASE_ENERGY_COST;

        sprite.2 .0 -= e;

        sprite.0.translation.x += delta_x;
        sprite.0.translation.y += delta_y;

        let window = windows.primary();
        let width = window.width() / 2.;
        let height = window.height() / 2.;

        if sprite.0.translation.x > width {
            sprite.0.translation.x = -width;
        }
        if sprite.0.translation.x < -width {
            sprite.0.translation.x = width;
        }
        if sprite.0.translation.y > height {
            sprite.0.translation.y = -height;
        }
        if sprite.0.translation.y < -height {
            sprite.0.translation.y = height;
        }
    }
}

fn home_movement(
    time: Res<Time>,
    mut commands: Commands,
    mut sprites: Query<
        (&mut Transform, Entity, &Traits, &mut Energy),
        (With<Person>, With<Returning>, Without<Dead>),
    >,
    windows: Res<Windows>,
) {
    let window = windows.primary();
    let width = window.width() / 2.;
    let height = window.height() / 2.;

    for mut sprite in sprites.iter_mut() {
        let d = MOVEMENT_SPEED * time.delta_seconds() * sprite.2.speed;
        let e = d * sprite.2.speed * BASE_ENERGY_COST;

        let mut transform = sprite.0;
        let left = transform.translation.x + width;
        let right = width - transform.translation.x;
        let bottom = transform.translation.y + height;
        let top = height - transform.translation.y;

        let min = [left, right, bottom, top]
            .into_iter()
            .reduce(f32::min)
            .unwrap_or(0.);
        if min <= 0. {
            commands.entity(sprite.1).insert(AtHome);
        } else if min == left {
            transform.translation.x -= d;
            transform.rotation = Quat::from_rotation_z(f32::to_radians(180.));
            sprite.3 .0 -= e;
        } else if min == right {
            transform.translation.x += d;
            transform.rotation = Quat::from_rotation_z(f32::to_radians(0.));
            sprite.3 .0 -= e;
        } else if min == bottom {
            transform.translation.y -= d;
            transform.rotation = Quat::from_rotation_z(f32::to_radians(270.));
            sprite.3 .0 -= e;
        } else if min == top {
            transform.translation.y += d;
            transform.rotation = Quat::from_rotation_z(f32::to_radians(90.));
            sprite.3 .0 -= e;
        }
    }
}

fn fertile_return(mut commands: Commands, entities: Query<Entity, With<Fertile>>) {
    for entity in entities.iter() {
        commands.entity(entity).insert(Returning);
    }
}

fn non_hungry_return(
    mut commands: Commands,
    entities: Query<Entity, (With<Person>, Without<Hungry>)>,
) {
    for entity in entities.iter() {
        commands.entity(entity).insert(Returning);
    }
}

fn collision(
    mut commands: Commands,
    persons: Query<(Entity, &Transform), (With<Person>, Without<Fertile>)>,
    foods: Query<(Entity, &Transform), With<Food>>,
    mut eaten: Query<&mut Eaten>,
    hungry: Query<&Hungry>,
) {
    let person_size = Vec2::new(64., 64.);
    let food_size = Vec2::new(0., 0.);
    for person in persons.iter() {
        for food in foods.iter() {
            if collide(
                person.1.translation,
                person_size,
                food.1.translation,
                food_size,
            )
            .is_some()
            {
                if let Ok(mut is_eaten) = eaten.get_mut(food.0) {
                    if !is_eaten.0 {
                        commands.entity(food.0).despawn();
                        commands.entity(food.0).remove::<Eaten>();

                        if hungry.contains(person.0) {
                            commands.entity(person.0).remove::<Hungry>();
                        } else {
                            commands.entity(person.0).insert(Fertile);
                        }
                        is_eaten.0 = true;
                    }
                }
            }
        }
    }
}

fn day_timer(time: Res<Time>, mut timer: ResMut<DayTimer>, mut sunset: ResMut<Sunset>) {
    if !sunset.0 {
        timer.0.tick(time.delta());
        if timer.0.finished() {
            sunset.0 = true;
        }
    }
}

fn night_timer(
    time: Res<Time>,
    mut commands: Commands,
    mut timer: ResMut<NightTimer>,
    mut sunset: ResMut<Sunset>,
    to_die: Query<Entity, (With<Person>, Without<AtHome>)>,
    mut to_live: Query<(Entity, &mut Energy), (With<Person>, With<AtHome>)>,
    to_reproduce: Query<&Transform, (With<Person>, With<AtHome>, With<Fertile>)>,
    mut ev_randomize: EventWriter<RandomizeDirections>,
    mut ev_spawn_food: EventWriter<SpawnFood>,
    mut ev_reproduce: EventWriter<Reproduce>,
) {
    timer.0.tick(time.delta());
    if timer.0.finished() {
        sunset.0 = false;

        for person in to_die.iter() {
            commands.entity(person).despawn();
        }
        for mut person in to_live.iter_mut() {
            person.1 .0 = BASE_ENERGY;
            commands.entity(person.0).insert(Hungry);
            commands
                .entity(person.0)
                .remove_bundle::<(Fertile, Returning, AtHome)>();
        }
        for person in to_reproduce.iter() {
            ev_reproduce.send(Reproduce(*person));
        }
        ev_randomize.send(RandomizeDirections);
        ev_spawn_food.send(SpawnFood);
    }
}

fn reproduce(
    mut commands: Commands,
    mut events: EventReader<Reproduce>,
    asset_server: Res<AssetServer>,
) {
    for event in events.iter() {
        commands
            .spawn_bundle(SpriteBundle {
                texture: asset_server.load("person1.png"),
                transform: event.0,
                ..default()
            })
            .insert(Person)
            .insert(Hungry)
            .insert(Energy(BASE_ENERGY))
            .insert(Traits { speed: 1. });
    }
}

fn count_stuff(
    mut events: EventReader<RandomizeDirections>,
    persons: Query<&Person>,
    foods: Query<&Food>,
) {
    for _event in events.iter() {
        println!("{}\t{}", persons.iter().count(), foods.iter().count());
    }
}

fn background_color(sunset: Res<Sunset>, mut clear_color: ResMut<ClearColor>) {
    if sunset.0 {
        clear_color.0 = Color::rgb(0.5, 0.4, 0.4);
    } else {
        clear_color.0 = Color::rgb(0.9, 0.8, 0.8);
    }
}

fn randomize_directions(
    mut events: EventReader<RandomizeDirections>,
    mut persons: Query<&mut Transform, With<Person>>,
) {
    let mut rng = rand::thread_rng();
    for _event in events.iter() {
        for mut person in persons.iter_mut() {
            person.rotation = Quat::from_rotation_z(f32::to_radians(rng.gen::<f32>() * 360.))
        }
    }
}

fn energy(mut commands: Commands, people: Query<(Entity, &Energy), Without<Dead>>) {
    for person in people.iter() {
        if person.1 .0 <= 0. {
            commands.entity(person.0).insert(Dead);
        }
    }
}

fn run_if_sunset(sunset: Res<Sunset>) -> ShouldRun {
    if sunset.0 {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}

fn run_if_day(sunset: Res<Sunset>) -> ShouldRun {
    if !sunset.0 {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}

//fn debug1(query: Query<Entity, Without<Dead>>) {
//    for item in query.iter() {
//        println!("Dead");
//    }
//}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.9, 0.8, 0.8)))
        .insert_resource(Sunset(false))
        .insert_resource(DayTimer {
            0: Timer::from_seconds(DAY_LENGTH, true),
        })
        .insert_resource(NightTimer {
            0: Timer::from_seconds(NIGHT_LENGTH + DAY_LENGTH, true),
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(background_color)
        .add_system(home_movement)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(run_if_sunset)
                .with_system(non_hungry_return),
        )
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(run_if_day)
                .with_system(random_movement)
                .with_system(collision),
        )
        .add_system(fertile_return)
        .add_system(day_timer)
        .add_system(randomize_directions)
        .add_system(spawn_food)
        .add_system(reproduce)
        .add_system(count_stuff)
        .add_system(energy)
        .add_system_to_stage(CoreStage::PreUpdate, night_timer)
        .add_event::<RandomizeDirections>()
        .add_event::<SpawnFood>()
        .add_event::<Reproduce>()
        .run();
}
