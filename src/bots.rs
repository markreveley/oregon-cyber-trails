//! Scripted players for balancing. Each plays a whole run through the same
//! prompts a person sees.

use crate::game::{ChoiceInfo, Company, Ctx, Ending, Game, Level, Prompt};
use crate::rng::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Strategy {
    /// Picks at random.
    Random,
    /// Always the cheapest option; buys nothing.
    Miser,
    /// Always the most cautious option; buys everything it can.
    Paranoid,
    /// What the field notes recommend, within budget.
    Textbook,
}

impl Strategy {
    pub const ALL: [Strategy; 4] = [
        Strategy::Random,
        Strategy::Miser,
        Strategy::Paranoid,
        Strategy::Textbook,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Strategy::Random => "random",
            Strategy::Miser => "miser",
            Strategy::Paranoid => "paranoid",
            Strategy::Textbook => "textbook",
        }
    }
}

/// The order a sensible defender buys controls in.
const TEXTBOOK_BUYS: &[&str] = &[
    "password-manager",
    "passkeys",
    "backups",
    "logging",
    "secret-scanning",
    "ddos",
    "short-tokens",
];

pub struct Bot {
    pub strategy: Strategy,
    rng: Rng,
    company: Company,
    level: Level,
}

impl Bot {
    pub fn new(strategy: Strategy, seed: u64, company: Company, level: Level) -> Bot {
        Bot {
            strategy,
            rng: Rng::new(seed ^ 0xB07),
            company,
            level,
        }
    }

    fn pick(choices: &[ChoiceInfo], i: usize) -> String {
        debug_assert!(i < choices.len());
        (i + 1).to_string()
    }

    fn available(choices: &[ChoiceInfo]) -> Vec<usize> {
        (0..choices.len())
            .filter(|&i| choices[i].available)
            .collect()
    }

    fn by_key(choices: &[ChoiceInfo], key: &str) -> Option<usize> {
        choices.iter().position(|c| c.key == key && c.available)
    }

    pub fn answer(&mut self, g: &Game, p: &Prompt) -> String {
        let c = &p.choices;
        match p.ctx {
            Ctx::Company => (crate::game::Company::ALL
                .iter()
                .position(|&x| x == self.company)
                .unwrap()
                + 1)
            .to_string(),
            Ctx::Level => (self.level as usize + 1).to_string(),
            Ctx::Name | Ctx::Continue | Ctx::Epitaph | Ctx::Done => String::new(),
            Ctx::Pace | Ctx::OnCall => "2".into(),
            Ctx::Expo => self.expo(g, c),
            Ctx::Menu => self.menu(g, c),
            Ctx::Event => self.event(c),
            Ctx::Hunt => {
                let skill = match self.strategy {
                    Strategy::Textbook | Strategy::Paranoid => 0.6,
                    _ => 0.0,
                };
                let ans = g.hunt_answer().unwrap_or(0);
                if self.rng.chance(skill) {
                    Self::pick(c, ans)
                } else {
                    Self::pick(c, self.rng.index(c.len()))
                }
            }
            Ctx::Order => {
                if self.strategy == Strategy::Textbook && self.rng.chance(0.7) {
                    g.order_answer().unwrap_or_default()
                } else {
                    let n = g.order_answer().map_or(0, |s| s.split(' ').count());
                    let mut v: Vec<usize> = (1..=n).collect();
                    self.rng.shuffle(&mut v);
                    v.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                }
            }
        }
    }

    fn expo(&mut self, g: &Game, c: &[ChoiceInfo]) -> String {
        let done = c.len();
        let reserve = g.weekly_burn() * 6;
        let buy = match self.strategy {
            Strategy::Miser => None,
            Strategy::Random => {
                let av = Self::available(c);
                if self.rng.chance(0.4) {
                    av.get(self.rng.index(av.len()))
                        .copied()
                        .filter(|&i| i + 1 != done)
                } else {
                    None
                }
            }
            Strategy::Paranoid => (0..c.len())
                .find(|&i| c[i].available && c[i].key != "done" && g.budget - c[i].cost >= reserve),
            Strategy::Textbook => TEXTBOOK_BUYS.iter().find_map(|k| {
                let i = Self::by_key(c, &format!("buy:{k}"))?;
                // Passkeys make one-time codes redundant; skip MFA.
                (g.budget - c[i].cost >= reserve).then_some(i)
            }),
        };
        match buy {
            Some(i) => Self::pick(c, i),
            None => done.to_string(),
        }
    }

    fn menu(&mut self, g: &Game, c: &[ChoiceInfo]) -> String {
        let tired = g.team.iter().filter(|m| !m.gone && m.tired()).count();
        let key = match self.strategy {
            Strategy::Random => {
                let opts = ["continue", "continue", "continue", "hunt", "rest"];
                opts[self.rng.index(opts.len())]
            }
            Strategy::Miser => "continue",
            Strategy::Paranoid => {
                if tired >= 2 {
                    "rest"
                } else {
                    "hunt"
                }
            }
            Strategy::Textbook => {
                if tired >= 2 {
                    "rest"
                } else if g.week % 3 == 2 {
                    "hunt"
                } else {
                    "continue"
                }
            }
        };
        let i = Self::by_key(c, key)
            .or_else(|| Self::by_key(c, "continue"))
            .unwrap();
        Self::pick(c, i)
    }

    fn event(&mut self, c: &[ChoiceInfo]) -> String {
        let av = Self::available(c);
        let tagged = |t: &str| {
            av.iter()
                .copied()
                .find(|&i| c[i].tags.iter().any(|x| x == t))
        };
        let cheapest = || {
            *av.iter()
                .min_by_key(|&&i| c[i].cost + c[i].hours as i64 * 50)
                .unwrap()
        };
        let i = match self.strategy {
            Strategy::Random => av[self.rng.index(av.len())],
            Strategy::Miser => cheapest(),
            Strategy::Paranoid => tagged("secure").unwrap_or_else(|| {
                *av.iter()
                    .max_by_key(|&&i| c[i].cost + c[i].hours as i64 * 50)
                    .unwrap()
            }),
            Strategy::Textbook => tagged("textbook")
                .or_else(|| tagged("secure"))
                .unwrap_or_else(cheapest),
        };
        Self::pick(c, i)
    }
}

#[derive(Clone, Debug)]
pub struct RunResult {
    pub ending: Ending,
    pub score: i64,
    pub week: u32,
    pub steps: usize,
    pub trust: i32,
    pub budget: i64,
    pub min_budget: i64,
    pub events: usize,
}

/// Play one whole run with a bot. Panics if the run stalls, which would be
/// an engine bug.
pub fn play(strategy: Strategy, seed: u64, company: Company, level: Level) -> RunResult {
    let mut g = Game::new(seed, vec![]);
    let mut bot = Bot::new(strategy, seed, company, level);
    let mut steps = 0;
    let mut min_budget = i64::MAX;
    while !g.is_done() {
        if g.week > 0 {
            min_budget = min_budget.min(g.budget);
        }
        g.take_output();
        let p = g.prompt();
        let a = bot.answer(&g, &p);
        g.input(&a);
        steps += 1;
        assert!(
            steps < 5000,
            "run stalled at {:?} (seed {seed}, {strategy:?})",
            g.phase
        );
    }
    RunResult {
        ending: g.ending.unwrap(),
        score: g.score(),
        week: g.week,
        steps,
        trust: g.trust,
        budget: g.budget,
        min_budget,
        events: g.journal.len(),
    }
}

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub runs: usize,
    pub wins: usize,
    pub score_sum: i64,
    pub trust_sum: i64,
    pub min_budget_sum: i64,
    pub events_sum: usize,
    pub endings: std::collections::BTreeMap<String, usize>,
}

impl Stats {
    pub fn win_rate(&self) -> f64 {
        self.wins as f64 / self.runs.max(1) as f64
    }
    pub fn mean_score(&self) -> i64 {
        self.score_sum / self.runs.max(1) as i64
    }
}

pub fn simulate(
    strategy: Strategy,
    company: Company,
    level: Level,
    runs: usize,
    seed0: u64,
) -> Stats {
    let mut s = Stats::default();
    for i in 0..runs {
        let r = play(strategy, seed0 + i as u64, company, level);
        s.runs += 1;
        if r.ending.won() {
            s.wins += 1;
        }
        s.score_sum += r.score;
        s.trust_sum += r.trust as i64;
        s.min_budget_sum += r.min_budget.min(r.budget);
        s.events_sum += r.events;
        *s.endings.entry(format!("{:?}", r.ending)).or_default() += 1;
    }
    s
}
