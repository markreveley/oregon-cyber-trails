//! The terminal front end, plus `sim` (balance bots) and `check` (content).

use oregon_cyber_trails::bots::{Strategy, simulate};
use oregon_cyber_trails::content::Content;
use oregon_cyber_trails::{Company, Ctx, Game, Level, Tombstone};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

const USAGE: &str = "\
Oregon Cyber Trails

USAGE:
    oregon-cyber-trails [--seed N]
        Play. The seed replays the same run for the same choices.
    oregon-cyber-trails sim [--runs N] [--level L] [--company C]
        Run the balancing bots. L: beginner|intermediate|advanced|pro|all.
        C: bank|startup|garage|all.
    oregon-cyber-trails check
        Validate the content files.
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    match args.first().map(String::as_str) {
        Some("check") => check(),
        Some("sim") => sim(
            flag("--runs").and_then(|v| v.parse().ok()).unwrap_or(500),
            flag("--level").unwrap_or_else(|| "all".into()),
            flag("--company").unwrap_or_else(|| "startup".into()),
        ),
        Some("-h") | Some("--help") => print!("{USAGE}"),
        _ => {
            let seed = flag("--seed")
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(time_seed);
            play(seed);
        }
    }
}

fn time_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() % 100_000)
        .unwrap_or(1)
}

fn check() {
    match Content::parse() {
        Err(e) => {
            eprintln!("parse error: {e}");
            std::process::exit(1);
        }
        Ok(c) => {
            let errs = c.check();
            for e in &errs {
                eprintln!("{e}");
            }
            println!(
                "{} events, {} controls, {} arcs: {} problems",
                c.events.len(),
                c.controls.len(),
                c.arcs.len(),
                errs.len()
            );
            if !errs.is_empty() {
                std::process::exit(1);
            }
        }
    }
}

fn sim(runs: usize, level: String, company: String) {
    let levels: Vec<Level> = if level == "all" {
        Level::ALL.to_vec()
    } else {
        Level::parse(&level).into_iter().collect()
    };
    let companies: Vec<Company> = if company == "all" {
        Company::ALL.to_vec()
    } else {
        Company::parse(&company).into_iter().collect()
    };
    if levels.is_empty() || companies.is_empty() {
        eprint!("{USAGE}");
        std::process::exit(2);
    }
    println!(
        "{:<10} {:<13} {:<9} {:>6} {:>8} {:>6} {:>9} {:>7}  endings",
        "company", "level", "bot", "win%", "score", "trust", "low cash", "events"
    );
    for &c in &companies {
        for &l in &levels {
            for s in Strategy::ALL {
                let st = simulate(s, c, l, runs, 1);
                let endings: Vec<String> =
                    st.endings.iter().map(|(k, v)| format!("{k}:{v}")).collect();
                let n = st.runs.max(1) as i64;
                println!(
                    "{:<10} {:<13} {:<9} {:>5.1}% {:>8} {:>6} {:>9} {:>7.1}  {}",
                    c.key(),
                    l.name(),
                    s.name(),
                    st.win_rate() * 100.0,
                    st.mean_score(),
                    st.trust_sum / n,
                    st.min_budget_sum / n,
                    st.events_sum as f64 / n as f64,
                    endings.join(" ")
                );
            }
        }
    }
}

fn data_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    let dir = PathBuf::from(home).join(".oregon-cyber-trails");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn load_tombstones() -> Vec<Tombstone> {
    let Some(dir) = data_dir() else { return vec![] };
    let Ok(text) = std::fs::read_to_string(dir.join("tombstones.tsv")) else {
        return vec![];
    };
    let mut v: Vec<Tombstone> = text
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() == 4).then(|| Tombstone {
                name: f[0].into(),
                cause: f[1].into(),
                week: f[2].parse().unwrap_or(0),
                epitaph: f[3].into(),
            })
        })
        .collect();
    v.reverse();
    v.truncate(10);
    v
}

fn save_tombstone(t: &Tombstone) {
    let Some(dir) = data_dir() else { return };
    let clean = |s: &str| s.replace(['\t', '\n'], " ");
    let line = format!(
        "{}\t{}\t{}\t{}\n",
        clean(&t.name),
        clean(&t.cause),
        t.week,
        clean(&t.epitaph)
    );
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("tombstones.tsv"))
    {
        let _ = f.write_all(line.as_bytes());
    }
}

fn play(seed: u64) {
    let mut g = Game::new(seed, load_tombstones());
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let mut stdout = io::stdout();
    loop {
        for l in g.take_output() {
            if writeln!(stdout, "{l}").is_err() {
                return;
            }
        }
        if g.is_done() {
            break;
        }
        let p = g.prompt();
        print!("\n{} ", p.text);
        let _ = stdout.flush();
        let Some(Ok(line)) = lines.next() else {
            println!();
            return;
        };
        g.input(&line);
    }
    if let Some(e) = g.ending
        && !e.won()
        && g.phase == Ctx::Done
        && !g.epitaph.is_empty()
    {
        save_tombstone(&g.tombstone());
    }
    println!("\nSeed {seed}. Replay it with: oregon-cyber-trails --seed {seed}");
    print!("Save your trail journal? (y/n) ");
    let _ = stdout.flush();
    if let Some(Ok(a)) = lines.next()
        && a.trim().eq_ignore_ascii_case("y")
    {
        let path = format!("trail-journal-{seed}.md");
        match std::fs::write(&path, g.journal_markdown()) {
            Ok(()) => println!("Saved {path}"),
            Err(e) => println!("Could not save {path}: {e}"),
        }
    }
}
