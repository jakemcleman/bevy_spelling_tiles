use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use std::{fs};
use rand::seq::SliceRandom;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WordList { all_valid_words: Vec::new(), potential_pangrams: Vec::new() })
        .insert_resource(GameState { target_string: String::new(), target_bits: 0, required_letter: ' ', required_bit: 0, correct_words: Vec::new() })
        .add_event::<LetterAcceptedEvent>()
        .add_event::<WordAcceptedEvent>()
        .add_event::<WordRejectedEvent>()
        .add_plugins(DefaultPlugins)
        .add_plugin(ShapePlugin)
        .add_startup_system(setup_word_list)
        .add_startup_system(setup_goals.after(setup_word_list))
        .add_startup_system(setup_shapes.after(setup_goals))
        .add_system(chose_letter)
        .add_system(add_letter)
        .add_system(guess_word)
        .add_system(wrong_word_hint)
        .add_system(show_correct_words)
        .add_system(squish_effects)
        .run();
}

#[derive(Resource)]
struct WordList {
    all_valid_words: Vec<String>,
    potential_pangrams: Vec<String>, // Will be a subset of all words since these are also valid
}

#[derive(Resource)]
struct GameState {
    target_string: String,
    target_bits: u32,
    required_letter: char,
    required_bit: u32,
    correct_words: Vec<String>,
}

#[derive(Component)]
struct LetterTile {
    letter: char,
}

#[derive(Component)]
struct TriedWord {
    current: String,
}

#[derive(Component)]
struct CorrectWordsList {}

#[derive(Component)]
struct HintText {}

#[derive(Component)]
struct SquishEffect {
    attack_duration: f32,
    sustain_duration: f32,
    decay_duration: f32,
    elapsed: f32,
    total_time: f32,
    base_scale: Vec3,
    squish_multiplier: Vec3,
}

impl SquishEffect {
    fn new(base_scale: Vec3, squish_multiplier: Vec3, attack_duration: f32, sustain_duration: f32, decay_duration: f32) -> SquishEffect {
        SquishEffect { attack_duration, sustain_duration, decay_duration, elapsed: 0., total_time: attack_duration + sustain_duration + decay_duration, base_scale, squish_multiplier }
    }

    fn reset(&mut self) {
        self.elapsed = 0.0;
    }
}


struct LetterAcceptedEvent {
    letter: char,
}

struct WordAcceptedEvent {
    word: String,
    pangram: bool,
}

struct WordRejectedEvent {
    word: String,
    reason: String,
}

fn get_spacings(sides: usize, radius: f32, face_index: usize) -> (f32, f32) {
    let interval = (face_index as f32) * f32::to_radians(360.) / (sides as f32);
    (f32::sin(interval) * radius * 2., f32::cos(interval) * radius * 2.)
}

fn alphabet_index(letter: u8) -> usize {
    if letter >= 'A' as u8 && letter <= 'Z' as u8 {
        letter as usize - 'A' as usize
    }
    else if letter >= 'a' as u8 && letter <= 'z' as u8 {
        letter as usize - 'a' as usize   
    }
    else {
        panic!("not a letter");
    }
}

fn word_to_bits(word: &str) -> u32 {
    let mut val = 0;

    for c in word.as_bytes() {
        val |= 1 << alphabet_index(*c);
    }

    val
}

fn is_valid_word(word: &str) -> bool {
    if word.len() < 4 {
        return false;
    }
    
    for c in word.as_bytes() {
        if !c.is_ascii_alphabetic() {
            return false;
        }
    }
    return true;
}

fn bits_to_letters(bits: u32) -> String {
    let mut s = String::new();

    for c in 'A'..'Z' {
        if bits & 1 << alphabet_index(c as u8) != 0 {
            s.push(c);
        }
    }

    s
}

fn setup_word_list(mut wordlist: ResMut<WordList>) {
    let words_path = "assets/words/dict_words.txt";
    let file_contents = fs::read_to_string(words_path).expect("Was not able to read word list");

    for word in file_contents.split_whitespace() {
        if is_valid_word(word) {
            wordlist.all_valid_words.push(String::from(word));

            if word_to_bits(word).count_ones() == 7 {
                wordlist.potential_pangrams.push(String::from(word));
            }
        }
    }
}

fn setup_goals(wordlist: Res<WordList>, mut gamestate: ResMut<GameState>) {
    let target_pangram = wordlist.potential_pangrams.choose(&mut rand::thread_rng()).unwrap().to_uppercase();
    gamestate.target_bits = word_to_bits(target_pangram.as_str());
    gamestate.target_string = bits_to_letters(gamestate.target_bits);

    //println!("target pangram is {0}, target letters are {1}", target_pangram, gamestate.target_string);

    unsafe {
        let letters = gamestate.target_string.as_bytes_mut();
        letters.shuffle(&mut rand::thread_rng());
        gamestate.required_letter = letters[0] as char;
    }
    gamestate.required_bit = (1 as u32) << alphabet_index(gamestate.required_letter as u8);

    println!("target string is shuffled to {}", gamestate.target_string);
}

fn check_word(word: &str, gamestate: &GameState, wordlist: &WordList) -> (bool, String, bool) {
    if word.len() < 4 {
        (false, String::from("is too short!"), false)
    }
    else {
        let word_bits = word_to_bits(word);
        if (word_bits & gamestate.required_bit != 0) && ((word_bits ^ gamestate.target_bits) & word_bits == 0) {
            if wordlist.all_valid_words.contains(&String::from(word.to_ascii_lowercase())) {
                if gamestate.correct_words.contains(&String::from(word.to_ascii_lowercase())) {
                    (false, String::from("was already found"), false)
                }
                else {
                    (true, String::from("hap :)"), (word_bits.count_ones() == 7))
                }
            }
            else {
                (false, String::from("is not in word list"), false)
            }
        }
        else {
            (false, String::from("does not use required letters"), false)
        }
    }
}

fn setup_shapes(mut commands: Commands, asset_server: Res<AssetServer>, gamestate: Res<GameState>) {
    let center = Vec3::new(-80., -40., 0.);
    let sides = 6;
    let spacing = 0.;
    let radius = 80.;
    let center_color = Color::CYAN;
    let petal_color = Color::ALICE_BLUE;
    let line_width = 8.0;

    let shape = shapes::RegularPolygon {
        sides,
        feature: shapes::RegularPolygonFeature::Radius(radius),
        ..shapes::RegularPolygon::default()
    };

    let bold_font = asset_server.load("fonts/BarlowCondensed-Bold.ttf");
    let narrow_font = asset_server.load("fonts/BarlowCondensed-Regular.ttf");
    let tiles_text_style = TextStyle {
        font: bold_font.clone(),
        font_size: radius,
        color: Color::BLACK,
    };
    let word_text_style = TextStyle {
        font: bold_font.clone(),
        font_size: radius,
        color: Color::WHITE,
    };
    let info_text_style = TextStyle {
        font: narrow_font.clone(),
        font_size: radius / 2.,
        color: Color::WHITE,
    };
    let text_alignment = TextAlignment::CENTER;

    let letters = gamestate.target_string.as_bytes();

    commands.spawn(Camera2dBundle::default());
    commands.spawn(GeometryBuilder::build_as(
        &shape,
        DrawMode::Outlined {
            fill_mode: FillMode::color(center_color),
            outline_mode: StrokeMode::new(Color::BLACK, line_width),
        },
        Transform::from_translation(center),
    )).insert(LetterTile {
        letter: letters[0] as char
    }).insert(SquishEffect::new(Vec3::ONE, Vec3::splat(0.8), 0.0, 0.0, 0.1));
    commands.spawn(Text2dBundle{
        text: Text::from_section(letters[0] as char, tiles_text_style.clone()).with_alignment(text_alignment),
        transform: Transform::from_translation(center + Vec3::new(0., 0., 1.)),
        ..default()
    });

    
    commands.spawn(Text2dBundle{
        text: Text::from_section("_", word_text_style.clone()).with_alignment(TextAlignment::CENTER),
        transform: Transform::from_translation(center + Vec3::new(0., 4.2 * radius, 1.)),
        ..default()
    }).insert(TriedWord {
        current: String::new(),
    });

    commands.spawn(Text2dBundle{
        text: Text::from_section("", info_text_style.clone()).with_alignment(TextAlignment::CENTER),
        transform: Transform::from_translation(center + Vec3::new(0., 3.6 * radius, 1.)),
        ..default()
    }).insert(HintText {});

    commands.spawn(Text2dBundle{
        text: Text::from_section("Found Words: 0", info_text_style.clone()).with_alignment(TextAlignment::TOP_CENTER),
        transform: Transform::from_translation(center + Vec3::new(6. * radius, 4.2 * radius, 1.)),
        ..default()
    }).insert(CorrectWordsList { });

    for i in 0..sides {
        let (x_space, y_space) = get_spacings(sides, radius + spacing, i);
        commands.spawn(GeometryBuilder::build_as(
            &shape,
            DrawMode::Outlined {
                fill_mode: FillMode::color(petal_color),
                outline_mode: StrokeMode::new(Color::BLACK, line_width),
            },
            Transform::from_translation(center + Vec3::new(x_space, y_space, 0.0)),
        )).insert(LetterTile {
            letter: letters[i + 1] as char
        }).insert(SquishEffect::new(Vec3::ONE, Vec3::splat(0.8), 0.0, 0.0, 0.1));
        commands.spawn(Text2dBundle{
            text: Text::from_section(letters[i + 1] as char, tiles_text_style.clone()).with_alignment(text_alignment),
            transform: Transform::from_translation(center + Vec3::new(x_space, y_space, 1.)),
            ..default()
        });
    }
    
}

fn chose_letter(mut char_evr: EventReader<ReceivedCharacter>,
                mut ev_letter_accepted: EventWriter<LetterAcceptedEvent>,
                mut letter_tiles: Query<(&LetterTile, &mut SquishEffect)>) {
    for ev in char_evr.iter() {
        for (tile, mut squish) in letter_tiles.iter_mut() {
            if ev.char.to_ascii_uppercase() == tile.letter.to_ascii_uppercase() {
                ev_letter_accepted.send(LetterAcceptedEvent { letter: ev.char });
                squish.reset();
                break;
            }
        }
    }
}

fn add_letter(mut word_guess: Query<(&mut Text, &mut TriedWord)>, 
              mut ev_letter_accepted: EventReader<LetterAcceptedEvent>,
              game_state: Res<GameState>,) {
    for ev in ev_letter_accepted.iter() {
        let (mut text, mut tried_word) = word_guess.get_single_mut().unwrap();
        tried_word.current.push(ev.letter.to_ascii_uppercase());
        let mut style = text.sections[0].style.clone();
        if ev.letter.to_ascii_uppercase() == game_state.required_letter {
            style.color = Color::CYAN;
        }
        else {
            style.color = Color::WHITE;
        }
        if text.sections[0].value == "_" {
            text.sections.clear();
        }
        text.sections.push(TextSection { value: ev.letter.to_ascii_uppercase().to_string(), style });
        //println!("got a letter! {0} Word so far is {1}", ev.letter, tried_word.current);
    }
}

fn guess_word(mut word_guess: Query<(&mut Text, &mut TriedWord)>,
              gamestate: Res<GameState>,
              wordlist: Res<WordList>, 
              keys: Res<Input<KeyCode>>,
              mut ev_word_accepted: EventWriter<WordAcceptedEvent>,
              mut ev_word_rejected: EventWriter<WordRejectedEvent>,
            ) {
    let (mut text, mut tried_word) = word_guess.get_single_mut().unwrap();

    if keys.just_pressed(KeyCode::Return) {
        let (correct, reason, pangram) = check_word(&tried_word.current.as_str(), &gamestate, &wordlist);
        
        if correct {
            ev_word_accepted.send(WordAcceptedEvent{ word: tried_word.current.clone(), pangram });
        }
        else {
            ev_word_rejected.send(WordRejectedEvent { word: tried_word.current.clone(), reason: reason });
        }

        text.sections[0].value = String::from("_");
        text.sections[0].style.color = Color::WHITE;
        text.sections.resize(1, TextSection::default());
        tried_word.current = String::new();
    }
    
    if keys.just_pressed(KeyCode::Back) {
        tried_word.current.pop();
        if text.sections.len() == 1 {
            text.sections[0].value = String::from("_");
            text.sections[0].style.color = Color::WHITE;
        } else {
            text.sections.pop();
        }
    }
}

fn wrong_word_hint(mut ev_word_rejected: EventReader<WordRejectedEvent>,
                   mut hint_text: Query<(&mut Text, &mut HintText)>,) {
    for ev in ev_word_rejected.iter() {
        for (mut text, _hint) in hint_text.iter_mut() {
            text.sections[0].value = ev.word.clone() + " " + ev.reason.as_str();
        }
    }
}

fn show_correct_words(mut ev_word_accepted: EventReader<WordAcceptedEvent>,
                      mut word_list: Query<(&mut Text, &CorrectWordsList)>,
                      mut gamestate: ResMut<GameState>,
                    ) {
    for ev in ev_word_accepted.iter() {
        for (mut text, _word_list) in word_list.iter_mut() {
            let style = text.sections[0].style.clone();
            let word = String::from("\n") + ev.word.as_str() + if ev.pangram { " *" } else { "" };
            text.sections.push(TextSection::new(word, style));
            gamestate.correct_words.push(ev.word.clone().to_ascii_lowercase());

            text.sections[0].value = format!("Found Words: {}", gamestate.correct_words.len());
        }
    }
}

fn squish_effects(mut squishees: Query<(&mut Transform, &mut SquishEffect)>, time: Res<Time>) {
    for (mut transform, mut squish) in squishees.iter_mut() {
        if squish.elapsed < squish.total_time {
            squish.elapsed += time.delta_seconds();
            let mut adj_elapsed = squish.elapsed;
            if adj_elapsed > squish.attack_duration {
                adj_elapsed -= squish.attack_duration;

                if adj_elapsed > squish.sustain_duration {
                    adj_elapsed -= squish.sustain_duration;

                    if adj_elapsed > squish.decay_duration {
                        // animation finished, reset scale
                        transform.scale = squish.base_scale;
                    }
                    else {
                        // in decay phase, do lerp out
                        let t = adj_elapsed / squish.decay_duration;
                        transform.scale = squish.base_scale.lerp(squish.squish_multiplier, 1.0 - t);
                    }
                }
                else {
                    // in sustain phase, hold
                    transform.scale = squish.base_scale * squish.squish_multiplier;
                }
            }
            else {
                // in attack phase, do lerp in
                let t = adj_elapsed / squish.attack_duration;
                transform.scale = squish.base_scale.lerp(squish.squish_multiplier, t);
            }
            
        }
    }
} 