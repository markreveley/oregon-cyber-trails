//! Whole-run tests: every bot finishes every kind of run, runs replay
//! exactly from a seed, and the balance stays in its bands.

use oregon_cyber_trails::bots::{Strategy, play, simulate};
use oregon_cyber_trails::{Company, Ctx, Game, Level};

#[test]
fn every_bot_finishes_every_kind_of_run() {
    for c in Company::ALL {
        for l in Level::ALL {
            for s in Strategy::ALL {
                for seed in 0..15 {
                    play(s, seed, c, l);
                }
            }
        }
    }
}

#[test]
fn a_seed_replays_exactly() {
    for seed in [1, 7, 99] {
        let a = play(Strategy::Random, seed, Company::Startup, Level::Advanced);
        let b = play(Strategy::Random, seed, Company::Startup, Level::Advanced);
        assert_eq!((a.score, a.week, a.steps), (b.score, b.week, b.steps));
    }
}

#[test]
fn transcript_replays_exactly() {
    let inputs = ["2", "2", "", "", "", "", "", "", "1", "3", "1"];
    let run = || {
        let mut g = Game::new(5, vec![]);
        let mut out = Vec::new();
        for i in inputs {
            out.extend(g.take_output());
            g.input(i);
        }
        // Then keep continuing until the run ends.
        for _ in 0..2000 {
            if g.is_done() {
                break;
            }
            out.extend(g.take_output());
            let answer = match g.prompt().ctx {
                Ctx::Order => g.order_answer().unwrap(),
                Ctx::Hunt => "1".to_string(),
                Ctx::Continue | Ctx::Epitaph => String::new(),
                _ => {
                    let p = g.prompt();
                    let i = p.choices.iter().position(|c| c.available).unwrap();
                    (i + 1).to_string()
                }
            };
            g.input(&answer);
        }
        out.extend(g.take_output());
        out
    };
    assert_eq!(run(), run());
}

/// Bands for the funded startup over a fixed set of seeds. The textbook bot
/// knows which option the field notes recommend, so its win rate is an
/// upper bound on a careful player's.
#[test]
fn balance_bands() {
    let runs = 200;
    let rate = |s, l| simulate(s, Company::Startup, l, runs, 1000).win_rate();
    for l in Level::ALL {
        let textbook = rate(Strategy::Textbook, l);
        let random = rate(Strategy::Random, l);
        let miser = rate(Strategy::Miser, l);
        assert!(
            textbook > random,
            "{l:?}: textbook {textbook} <= random {random}"
        );
        assert!(
            textbook > miser,
            "{l:?}: textbook {textbook} <= miser {miser}"
        );
    }
    let t = |l| rate(Strategy::Textbook, l);
    assert!(t(Level::Beginner) >= 0.95);
    assert!(t(Level::Intermediate) >= 0.85);
    assert!((0.6..=0.97).contains(&t(Level::Advanced)));
    assert!((0.3..=0.7).contains(&t(Level::Pro)));
    assert!(
        rate(Strategy::Random, Level::Beginner) >= 0.6,
        "Beginner should forgive a casual player"
    );
    assert!(
        rate(Strategy::Paranoid, Level::Pro) < t(Level::Pro),
        "overspending on security should not pay at Pro"
    );
}
