mod horse;
mod names;

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
    ExecutableCommand,
};
use std::{
    io::{stdout, Write},
    thread,
    time::{Duration, Instant},
};
use horse::Horse;

fn main() {
    let mut stdout = stdout();

    loop {
        let mut horses: Vec<Horse> = names::random_names()
            .into_iter()
            .map(|name| Horse::new(&name))
            .collect();

        stdout.execute(Clear(ClearType::All)).unwrap();
        println!("Welcome to 🍺 LAST CALL DERBY 🍺\n");
        for (i, h) in horses.iter().enumerate() {
            println!("{}. {}", i + 1, h.name);
        }

        print!("\nPlace your bet (1-{}): ", horses.len());
        stdout.flush().unwrap();
        let mut bet = String::new();
        std::io::stdin().read_line(&mut bet).unwrap();
        let bet_idx = bet.trim().parse::<usize>().unwrap_or(1) - 1;
        let bet_name = horses[bet_idx].name.clone();

        println!("\nGet ready...");
        thread::sleep(Duration::from_secs(1));
        for n in (1..=3).rev() {
            println!("{n}...");
            thread::sleep(Duration::from_secs(1));
        }
        println!("GO!!! 🏇");
        thread::sleep(Duration::from_millis(800));

        run_race(&mut stdout, horses, &bet_name);

        println!("\nPress ENTER to race again or 'q' to quit.");
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer).unwrap();
        if buffer.trim().eq_ignore_ascii_case("q") {
            println!("Thanks for racing! 🍻");
            break;
        }
    }
}

fn run_race(stdout: &mut std::io::Stdout, mut horses: Vec<Horse>, bet_name: &str) {
    let track_length: f32 = 70.0;
    enable_raw_mode().unwrap();
    stdout.execute(Hide).unwrap();
    stdout.execute(Clear(ClearType::All)).unwrap();

    println!("=== 🍀 LAST CALL DERBY 🍀 ===\n");
    for h in &horses {
        println!("{}:", h.name);
    }

    let start_time = Instant::now();
    loop {
        for horse in horses.iter_mut() {
            horse.advance();
            if horse.position > track_length {
                horse.position = track_length;
            }
        }

        for (i, horse) in horses.iter().enumerate() {
            let y = (i + 2) as u16;
            let pos = horse.position as u16;
            stdout.execute(MoveTo(0, y)).unwrap();

            let horse_icon = "🐴";
            let finish_line = "|🏁|";
            let spaces = " ".repeat(pos as usize);
            let after = " ".repeat((track_length as usize - pos as usize).saturating_sub(1));
            print!(
                "{:<18}|{}{}{}{}",
                horse.name, spaces, horse_icon, after, finish_line
            );
        }

        stdout.flush().unwrap();

        if let Some(winner) = horses.iter().find(|h| h.position >= track_length) {
            stdout.execute(MoveTo(0, horses.len() as u16 + 4)).unwrap();
            println!("\n🏁 WINNER: {}!", winner.name);
            if winner.name == bet_name {
                println!("🎉 You WIN! Everyone else drinks!");
            } else {
                println!("🍺 You lost! Take a drink, champ.");
            }
            break;
        }

        // smoother update (≈60 FPS)
        thread::sleep(Duration::from_millis(60));
    }

    stdout.execute(Show).unwrap();
    disable_raw_mode().unwrap();

    let elapsed = start_time.elapsed().as_secs_f32();
    println!("\nRace duration: {:.1}s", elapsed);
}
