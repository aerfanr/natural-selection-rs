use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::*;
use bevy::sprite::collide_aabb::collide;
use rand::Rng;

const MOVEMENT_SPEED: f32 = 500.;
const PERSON_COUNT: i32 = 1000;
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
            .insert(Hungry);
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
    mut sprite_position: Query<&mut Transform, (With<Person>, Without<Returning>)>,
    windows: Res<Windows>,
) {
    let mut rng = rand::thread_rng();

    for mut transform in sprite_position.iter_mut() {
        let rotation_delta =
            Quat::from_rotation_z((rng.gen::<f32>() - 0.5) * 12. * time.delta_seconds());
        transform.rotation *= rotation_delta;

        let rotation_rad = transform.rotation.to_euler(EulerRot::ZYX).0;
        let distance = MOVEMENT_SPEED * time.delta_seconds();
        let delta_x = distance * rotation_rad.cos();
        let delta_y = distance * rotation_rad.sin();

        transform.translation.x += delta_x;
        transform.translation.y += delta_y;

        let window = windows.primary();
        let width = window.width() / 2.;
        let height = window.height() / 2.;

        if transform.translation.x > width {
            transform.translation.x = -width;
        }
        if transform.translation.x < -width {
            transform.translation.x = width;
        }
        if transform.translation.y > height {
            transform.translation.y = -height;
        }
        if transform.translation.y < -height {
            transform.translation.y = height;
        }
    }
}

fn home_movement(
    time: Res<Time>,
    mut commands: Commands,
    mut sprites: Query<(&mut Transform, Entity), (With<Person>, With<Returning>)>,
    windows: Res<Windows>,
) {
    let window = windows.primary();
    let width = window.width() / 2.;
    let height = window.height() / 2.;

    for sprite in sprites.iter_mut() {
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
            transform.translation.x -= MOVEMENT_SPEED * time.delta_seconds();
            transform.rotation = Quat::from_rotation_z(f32::to_radians(180.));
        } else if min == right {
            transform.translation.x += MOVEMENT_SPEED * time.delta_seconds();
            transform.rotation = Quat::from_rotation_z(f32::to_radians(0.));
        } else if min == bottom {
            transform.translation.y -= MOVEMENT_SPEED * time.delta_seconds();
            transform.rotation = Quat::from_rotation_z(f32::to_radians(270.));
        } else if min == top {
            transform.translation.y += MOVEMENT_SPEED * time.delta_seconds();
            transform.rotation = Quat::from_rotation_z(f32::to_radians(90.));
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
    to_live: Query<Entity, (With<Person>, With<AtHome>)>,
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
        for person in to_live.iter() {
            commands.entity(person).insert(Hungry);
            commands
                .entity(person)
                .remove_bundle::<(Fertile, Returning, AtHome)>();
        }
        for person in to_reproduce.iter() {
            ev_reproduce.send(Reproduce(*person));
        }
        ev_randomize.send(RandomizeDirections);
        ev_spawn_food.send(SpawnFood);
    }
}

fn reproduce(mut commands: Commands, mut events: EventReader<Reproduce>, asset_server: Res<AssetServer>) {
    for event in events.iter() {
        commands.spawn_bundle(SpriteBundle {
                    texture: asset_server.load("person1.png"),
                    transform: event.0,
                    ..default()
                })
                .insert(Person)
                .insert(Hungry);
    }
}

fn count_stuff(mut events: EventReader<RandomizeDirections>, persons: Query<&Person>, foods: Query<&Food>) {
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

//fn debug1(query: Query<&Hungry, Changed<Hungry>>) {
//    for item in query.iter() {
//        println!("CHANGE");
//    }
//}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.9, 0.8, 0.8)))
        .insert_resource(Sunset(false))
        .insert_resource(DayTimer {
            0: Timer::from_seconds(2., true),
        })
        .insert_resource(NightTimer {
            0: Timer::from_seconds(3., true),
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
        .add_system_to_stage(CoreStage::PreUpdate, night_timer)
        .add_event::<RandomizeDirections>()
        .add_event::<SpawnFood>()
        .add_event::<Reproduce>()
        .run();
}
