use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContext, EguiPlugin};
use egui::plot::{Bar, BarChart, Line, Plot, Value, Values};
use egui::widgets::DragValue;
use rand::Rng;

const MOVEMENT_SPEED: f32 = 150.;
const DAY_LENGTH: f32 = 10.;
const NIGHT_LENGTH: f32 = 2.;

struct Sunset(bool);
struct DayTimer(Timer);
struct NightTimer(Timer);
#[derive(Default)]
struct Charts {
    population: Vec<Bar>,
    food_count: Vec<Bar>,
    avg_speed: Vec<Value>,
    avg_sense: Vec<Value>,
}
struct Started(bool);
struct Options {
    simulation_speed: f32,
    movement_speed: f32,
    day_length: f32,
    night_length: f32,
    base_energy_cost: f32,
    sense_cost: f32,
    base_energy: f32,
    trait_change_intensity: f32,
    person_count: i32,
    food_count: i32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            simulation_speed: 1.,
            movement_speed: MOVEMENT_SPEED,
            day_length: DAY_LENGTH,
            night_length: NIGHT_LENGTH,
            base_energy_cost: 1. / (NIGHT_LENGTH + DAY_LENGTH) / MOVEMENT_SPEED,
            sense_cost: 1. / (NIGHT_LENGTH + DAY_LENGTH),
            base_energy: 1.,
            trait_change_intensity: 0.1,
            person_count: 10,
            food_count: 100,
        }
    }
}

struct RandomizeDirections;
struct SpawnFood;
struct Reproduce(Transform, Traits);

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
struct Prey {
    x: f32,
    y: f32,
    distance: f32,
}
#[derive(Component, Copy, Clone)]
struct Traits {
    speed: f32,
    sense: f32,
}

impl Default for Traits {
    fn default() -> Self {
        Self {
            speed: 1.,
            sense: 100.,
        }
    }
}

impl Traits {
    fn variation(&self, change_intensity: f32) -> Traits {
        Traits {
            speed: self.speed * (1. + (rand::random::<f32>() * 2. - 1.) * change_intensity),
            sense: (self.sense * (1. + (rand::random::<f32>() * 2. - 1.) * change_intensity))
                .max(1e5),
        }
    }
}

#[derive(Component)]
struct Food;
#[derive(Component, Debug)]
struct Eaten(bool);

fn bar_options() -> Bar {
    Bar {
        argument: 0.,
        value: 0.,
        name: String::from(""),
        bar_width: 5.,
        base_offset: None,
        fill: egui::Color32::LIGHT_GRAY,
        orientation: egui::widgets::plot::Orientation::Vertical,
        stroke: egui::Stroke::none(),
    }
}

fn get_random_location(window: &Window) -> Transform {
    let width = window.width();
    let height = window.height();

    let x = (rand::random::<f32>() - 0.5) * width;
    let y = (rand::random::<f32>() - 0.5) * height;

    Transform::from_xyz(x, y, 0.)
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn start_simulation(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    windows: Res<Windows>,
    mut ev_spawn_food: EventWriter<SpawnFood>,
    mut ev_randomize: EventWriter<RandomizeDirections>,
    started: Res<Started>,
    mut day_timer: ResMut<DayTimer>,
    mut night_timer: ResMut<NightTimer>,
    options: Res<Options>,
) {
    if !started.is_changed() || !started.0 {
        return;
    };
    for _i in 0..options.person_count {
        commands
            .spawn_bundle(SpriteBundle {
                texture: asset_server.load("person1.png"),
                transform: get_random_location(windows.primary()),
                ..default()
            })
            .insert(Person)
            .insert(Hungry)
            .insert(Energy(options.base_energy))
            .insert(Traits::default());
    }
    ev_randomize.send(RandomizeDirections);

    ev_spawn_food.send(SpawnFood);

    day_timer.0 = Timer::from_seconds(options.day_length / options.simulation_speed, true);
    night_timer.0 = Timer::from_seconds(
        (options.day_length + options.night_length) / options.simulation_speed,
        true,
    );
}

fn spawn_food(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    windows: Res<Windows>,
    mut events: EventReader<SpawnFood>,
    options: Res<Options>,
) {
    for _event in events.iter() {
        for _i in 0..options.food_count {
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

fn normal_rotation(
    mut sprites: Query<
        (&mut Transform, Option<&Prey>),
        (With<Person>, Without<Returning>, Without<Dead>),
    >,
    time: Res<Time>,
    options: Res<Options>,
) {
    let mut rng = rand::thread_rng();
    for (mut transform, prey) in sprites.iter_mut() {
        if prey.is_some() {
            transform.rotation = Quat::from_rotation_z(
                (prey.unwrap().y - transform.translation.y)
                    .atan2(prey.unwrap().x - transform.translation.x),
            )
        } else {
            let rotation_delta = Quat::from_rotation_z(
                (rng.gen::<f32>() - 0.5) * 12. * time.delta_seconds() * options.simulation_speed,
            );
            transform.rotation *= rotation_delta;
        }
    }
}

fn normal_movement(
    time: Res<Time>,
    mut sprites: Query<
        (&mut Transform, &Traits, &mut Energy),
        (With<Person>, Without<Returning>, Without<Dead>),
    >,
    windows: Res<Windows>,
    options: Res<Options>,
) {
    for mut sprite in sprites.iter_mut() {
        let rotation_rad = sprite.0.rotation.to_euler(EulerRot::ZYX).0;
        let distance = options.movement_speed
            * sprite.1.speed
            * time.delta_seconds()
            * options.simulation_speed;
        let delta_x = distance * rotation_rad.cos();
        let delta_y = distance * rotation_rad.sin();
        let e = distance * sprite.1.speed * options.base_energy_cost
            + options.sense_cost * time.delta_seconds() * options.simulation_speed;

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
    options: Res<Options>,
) {
    let window = windows.primary();
    let width = window.width() / 2.;
    let height = window.height() / 2.;

    for mut sprite in sprites.iter_mut() {
        let d = options.movement_speed
            * time.delta_seconds()
            * options.simulation_speed
            * sprite.2.speed;
        let e = d * sprite.2.speed * options.base_energy_cost;

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

fn get_distance(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    ((x1 - x2).powi(2) + (y1 - y2).powi(2)).sqrt()
}

fn radar(
    mut commands: Commands,
    persons: Query<(Entity, &Transform, Option<&Prey>, &Traits), (With<Person>, Without<Fertile>)>,
    foods: Query<(Entity, &Transform), With<Food>>,
    mut eaten: Query<&mut Eaten>,
    hungry: Query<&Hungry>,
) {
    for person in persons.iter() {
        for food in foods.iter() {
            let distance = get_distance(
                person.1.translation.x,
                person.1.translation.y,
                food.1.translation.x,
                food.1.translation.y,
            );
            if distance <= 45. {
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
            } else if distance <= person.3.sense {
                if !person.2.is_some() || distance >= person.2.unwrap().distance {
                    commands.entity(person.0).insert(Prey {
                        x: food.1.translation.x,
                        y: food.1.translation.y,
                        distance: distance,
                    });
                }
            } else {
                if person.2.is_some() {
                    commands.entity(person.0).remove::<Prey>();
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
    to_reproduce: Query<(&Transform, &Traits), (With<Person>, With<AtHome>, With<Fertile>)>,
    mut ev_randomize: EventWriter<RandomizeDirections>,
    mut ev_spawn_food: EventWriter<SpawnFood>,
    mut ev_reproduce: EventWriter<Reproduce>,
    options: Res<Options>,
) {
    timer.0.tick(time.delta());
    if timer.0.finished() {
        sunset.0 = false;

        for person in to_die.iter() {
            commands.entity(person).despawn();
        }
        for mut person in to_live.iter_mut() {
            person.1 .0 = options.base_energy;
            commands.entity(person.0).insert(Hungry);
            commands
                .entity(person.0)
                .remove_bundle::<(Fertile, Returning, AtHome)>();
        }
        for person in to_reproduce.iter() {
            ev_reproduce.send(Reproduce(
                *person.0,
                person.1.variation(options.trait_change_intensity),
            ));
        }
        ev_randomize.send(RandomizeDirections);
        ev_spawn_food.send(SpawnFood);
    }
}

fn reproduce(
    mut commands: Commands,
    mut events: EventReader<Reproduce>,
    asset_server: Res<AssetServer>,
    options: Res<Options>,
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
            .insert(Energy(options.base_energy))
            .insert(event.1);
    }
}

fn count_stuff(
    mut events: EventReader<RandomizeDirections>,
    persons: Query<&Traits>,
    foods: Query<&Food>,
    time: Res<Time>,
    mut charts: ResMut<Charts>,
    mut exit: EventWriter<bevy::app::AppExit>,
    options: Res<Options>,
) {
    for _event in events.iter() {
        let mut speed_avg = 0.;
        let mut sense_avg = 0.;
        let mut people_count = 0.;
        for person in persons.iter() {
            speed_avg += person.speed;
            sense_avg += person.sense;
            people_count += 1.;
        }
        let food_count = foods.iter().count();
        if people_count <= 0. {
            exit.send(bevy::app::AppExit);
            break;
        }
        speed_avg = speed_avg / people_count;
        sense_avg = sense_avg / people_count;

        println!("{};\t{};\t{};", people_count, food_count, speed_avg);

        charts.population.push(Bar {
            argument: time.seconds_since_startup() * options.simulation_speed as f64,
            value: people_count as f64,
            name: String::from("Population"),
            ..bar_options()
        });
        charts.food_count.push(Bar {
            argument: time.seconds_since_startup() * options.simulation_speed as f64,
            value: food_count as f64,
            name: String::from("Food Count"),
            fill: egui::Color32::RED,
            bar_width: 1.,
            ..bar_options()
        });
        charts.avg_speed.push(Value {
            x: time.seconds_since_startup() * options.simulation_speed as f64,
            y: speed_avg as f64,
        });
        charts.avg_sense.push(Value {
            x: time.seconds_since_startup() * options.simulation_speed as f64,
            y: sense_avg as f64,
        });
    }
}

fn plot_stuff(mut context: ResMut<EguiContext>, charts: Res<Charts>) {
    egui::Window::new("Stats").show(context.ctx_mut(), |ui| {
        let population_chart = BarChart::new(charts.population.clone());
        let food_chart = BarChart::new(charts.food_count.clone());
        let avg_speed_line = Line::new(Values::from_values(charts.avg_speed.clone()));
        let avg_sense_line = Line::new(Values::from_values(charts.avg_sense.clone()));
        Plot::new("Stats_1").height(200.).show(ui, |plot_ui| {
            plot_ui.bar_chart(population_chart);
            plot_ui.bar_chart(food_chart);
        });
        Plot::new("Stats_2")
            .height(200.)
            .show(ui, |plot_ui| plot_ui.line(avg_speed_line));
        Plot::new("Stats_3")
            .height(200.)
            .show(ui, |plot_ui| plot_ui.line(avg_sense_line));
    });
}

fn options_window(
    mut context: ResMut<EguiContext>,
    mut started: ResMut<Started>,
    mut options: ResMut<Options>,
) {
    egui::Window::new("Options")
        .enabled(!started.0)
        .show(context.ctx_mut(), |ui| {
            ui.label("Simulation speed:");
            ui.add(
                DragValue::new(&mut options.simulation_speed)
                    .clamp_range(0.1..=100.)
                    .speed(0.2),
            );

            ui.label("Movement speed:");
            ui.add(DragValue::new(&mut options.movement_speed).speed(5));

            ui.label("Day length:");
            ui.add(DragValue::new(&mut options.day_length).clamp_range(0.1..=f64::MAX));

            ui.label("Night length:");
            ui.add(DragValue::new(&mut options.night_length).clamp_range(0.1..=f64::MAX));

            ui.label("Energy cost:");
            ui.add(DragValue::new(&mut options.base_energy_cost).speed(0.0001));

            ui.label("Base energy:");
            ui.add(DragValue::new(&mut options.base_energy));

            ui.label("Trait change intensity:");
            ui.add(DragValue::new(&mut options.trait_change_intensity).speed(0.05));

            ui.label("Base population:");
            ui.add(DragValue::new(&mut options.person_count).speed(10));

            ui.label("Food count:");
            ui.add(DragValue::new(&mut options.food_count).speed(10));

            if ui.button("Start Simulation").clicked() {
                started.0 = true;
            }
        });
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

fn run_if_started(started: Res<Started>) -> ShouldRun {
    if started.0 && !started.is_changed() {
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
    let options = Options::default();

    App::new()
        .insert_resource(ClearColor(Color::rgb(0.9, 0.8, 0.8)))
        .insert_resource(Sunset(false))
        .insert_resource(Charts::default())
        .insert_resource(Started(false))
        .insert_resource(Options::default())
        .insert_resource(DayTimer {
            0: Timer::from_seconds(options.day_length / options.simulation_speed, true),
        })
        .insert_resource(NightTimer {
            0: Timer::from_seconds(
                (options.night_length + options.day_length) / options.simulation_speed,
                true,
            ),
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_startup_system(setup)
        .add_system(start_simulation)
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
                .with_system(normal_movement)
                .with_system(normal_rotation)
                .with_system(radar),
        )
        .add_system_set_to_stage(
            CoreStage::PreUpdate,
            SystemSet::new()
                .with_run_criteria(run_if_started)
                .with_system(night_timer),
        )
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(run_if_started)
                .with_system(day_timer)
                .with_system(count_stuff),
        )
        .add_system(fertile_return)
        .add_system(randomize_directions)
        .add_system(spawn_food)
        .add_system(reproduce)
        .add_system(energy)
        .add_system(plot_stuff)
        .add_system(options_window)
        .add_event::<RandomizeDirections>()
        .add_event::<SpawnFood>()
        .add_event::<Reproduce>()
        .run();
}
