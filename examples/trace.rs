//! Watch a bot play one run, for balancing and playtesting.
//!
//!     cargo run --release --example trace -- textbook pro 21          # weekly numbers
//!     cargo run --release --example trace -- textbook pro 21 --full   # full transcript

use oregon_cyber_trails::bots::{Bot, Strategy};
use oregon_cyber_trails::{Company, Game, Level};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let strategy = Strategy::ALL
        .into_iter()
        .find(|s| Some(s.name()) == args.first().map(String::as_str))
        .unwrap_or(Strategy::Textbook);
    let level = args
        .get(1)
        .and_then(|l| Level::parse(l))
        .unwrap_or(Level::Intermediate);
    let seed = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let full = args.iter().any(|a| a == "--full");

    let mut g = Game::new(seed, vec![]);
    let mut bot = Bot::new(strategy, seed, Company::Startup, level);
    let mut week = 0;
    while !g.is_done() {
        let out = g.take_output();
        if full {
            out.iter().for_each(|l| println!("{l}"));
        }
        if g.week != week && !full {
            week = g.week;
            println!(
                "week {:>2}  budget {:>8}  revenue {:>6}  costs {:>6}  trust {:>3}",
                g.week,
                g.budget,
                g.weekly_revenue(),
                g.weekly_burn(),
                g.trust
            );
        }
        let p = g.prompt();
        let answer = bot.answer(&g, &p);
        if full {
            println!(">> {answer}");
        }
        g.input(&answer);
    }
    if full {
        g.take_output().iter().for_each(|l| println!("{l}"));
    }
    println!("{:?}, score {}", g.ending.unwrap(), g.score());
}
