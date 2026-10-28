//! The engine: a pure state machine. Front ends read `take_output()` and
//! `prompt()`, and send the player's answer to `input()`. Nothing in here
//! does I/O, so the same engine runs in a terminal, a browser or a bot.

use crate::content::{Content, Effect, EventDef, OptionDef, STAGE_IDS};
use crate::logs;
use crate::rng::Rng;
use crate::text::{WIDTH, money, round_sig, tombstone, wrap};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Company {
    Bank,
    Startup,
    Garage,
}

impl Company {
    pub const ALL: [Company; 3] = [Company::Bank, Company::Startup, Company::Garage];
    /// Money scale relative to the funded startup the content is priced for.
    pub fn scale(self) -> f64 {
        match self {
            Company::Bank => 2.0,
            Company::Startup => 1.0,
            Company::Garage => 0.25,
        }
    }
    pub fn start_budget(self) -> i64 {
        match self {
            Company::Bank => 200_000,
            Company::Startup => 80_000,
            Company::Garage => 16_000,
        }
    }
    pub fn multiplier(self) -> i64 {
        match self {
            Company::Bank => 1,
            Company::Startup => 2,
            Company::Garage => 3,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Company::Bank => "a bank spin-off",
            Company::Startup => "a funded startup",
            Company::Garage => "two founders in a garage",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Company::Bank => "bank",
            Company::Startup => "startup",
            Company::Garage => "garage",
        }
    }
    pub fn parse(s: &str) -> Option<Company> {
        Company::ALL.into_iter().find(|c| c.key() == s)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Level {
    Beginner = 0,
    Intermediate = 1,
    Advanced = 2,
    Pro = 3,
}

impl Level {
    pub const ALL: [Level; 4] = [
        Level::Beginner,
        Level::Intermediate,
        Level::Advanced,
        Level::Pro,
    ];
    pub fn idx(self) -> u8 {
        self as u8
    }
    pub fn multiplier(self) -> i64 {
        [1, 2, 3, 5][self as usize]
    }
    pub fn name(self) -> &'static str {
        ["Beginner", "Intermediate", "Advanced", "Pro"][self as usize]
    }
    pub fn parse(s: &str) -> Option<Level> {
        Level::ALL
            .into_iter()
            .find(|l| l.name().eq_ignore_ascii_case(s))
    }
    /// Share of the company's usual starting money.
    fn budget_share(self) -> f64 {
        [1.0, 0.95, 0.65, 0.5][self as usize]
    }
    /// Multiplier on trust lost.
    fn harm(self) -> f64 {
        [1.3, 1.25, 1.6, 2.0][self as usize]
    }
    /// Extra weekly chance of a random event.
    fn event_pressure(self) -> f64 {
        [0.1, 0.15, 0.2, 0.25][self as usize]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pace {
    Careful,
    Steady,
    Crunch,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OnCall {
    Generous,
    Normal,
    Skeleton,
}

#[derive(Clone, Debug)]
pub struct Member {
    pub name: String,
    pub role: &'static str,
    pub fatigue: i32,
    pub gone: bool,
    /// Hidden from the player.
    pub compromised: bool,
}

impl Member {
    pub fn tired(&self) -> bool {
        self.fatigue >= 4
    }
}

const ROLES: [(&str, &str); 5] = [
    ("founder", "Ana"),
    ("engineer", "Ben"),
    ("engineer", "Priya"),
    ("ops", "Lee"),
    ("intern", "Sam"),
];

const ROLE_TITLES: [&str; 5] = ["Founder", "Engineer", "Engineer", "Ops", "Intern"];

struct StageInfo {
    name: &'static str,
    milestone: &'static str,
}

fn stage_info(id: &str) -> StageInfo {
    match id {
        "launch" => StageInfo {
            name: "Launch",
            milestone: "your first paying customer",
        },
        "signin" => StageInfo {
            name: "Sign-in",
            milestone: "the Series A",
        },
        "identifier" => StageInfo {
            name: "The identifier",
            milestone: "a clean SOC 2 report",
        },
        "keys" => StageInfo {
            name: "Holding keys for others",
            milestone: "the enterprise contract",
        },
        _ => StageInfo {
            name: "Agents",
            milestone: "the IPO",
        },
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArcStatus {
    Dormant,
    Active,
    Stopped,
    Succeeded,
}

#[derive(Clone, Debug)]
pub struct TimelineEntry {
    pub week: u32,
    pub beat: usize,
    pub seen: bool,
    pub choice: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ArcState {
    pub def: usize,
    pub start_week: u32,
    pub progress: usize,
    pub status: ArcStatus,
    pub timeline: Vec<TimelineEntry>,
}

#[derive(Clone, Debug)]
struct Pending {
    due: u32,
    text: String,
    effect: Effect,
    event: Option<String>,
    cause: String,
}

#[derive(Clone, Debug)]
pub struct JournalEntry {
    pub week: u32,
    pub title: String,
    pub choice: String,
    pub outcome: String,
    pub note: Option<String>,
    pub source: Option<String>,
    pub textbook: bool,
    pub alternative: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Tombstone {
    pub name: String,
    pub cause: String,
    pub week: u32,
    pub epitaph: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ending {
    Ipo,
    Bankrupt,
    LostCustomers,
    Offline,
    EveryoneLeft,
    Acquired,
}

impl Ending {
    pub fn won(self) -> bool {
        matches!(self, Ending::Ipo)
    }
}

/// What the game is waiting for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ctx {
    Company,
    Level,
    Name,
    Expo,
    Menu,
    Pace,
    OnCall,
    Event,
    Hunt,
    Order,
    /// Press Enter.
    Continue,
    Epitaph,
    Done,
}

#[derive(Clone, Debug)]
pub struct ChoiceInfo {
    pub label: String,
    /// Dollars, scaled to the company.
    pub cost: i64,
    pub hours: i32,
    pub tags: Vec<String>,
    pub available: bool,
    /// What the choice does, for bots: "continue", "hunt", "buy:mfa"...
    pub key: String,
}

#[derive(Clone, Debug)]
pub struct Prompt {
    pub ctx: Ctx,
    pub choices: Vec<ChoiceInfo>,
    /// The text shown before the cursor.
    pub text: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum After {
    Queue,
    Menu,
}

#[derive(Clone, Debug)]
struct Cur {
    ev: usize,
    target: Option<usize>,
    down: i32,
    arc: Option<usize>,
    opts: Vec<usize>,
}

#[derive(Clone, Debug)]
struct OrderState {
    arc: usize,
    /// Timeline entry indices, in the shuffled order shown.
    shown: Vec<usize>,
}

#[derive(Clone)]
pub struct Game {
    content: &'static Content,
    pub seed: u64,
    rng: Rng,
    pub company: Company,
    pub level: Level,
    pub team: Vec<Member>,
    pub budget: i64,
    pub trust: i32,
    pub servers: i32,
    offline_weeks: u32,
    pub week: u32,
    trail: Vec<&'static str>,
    stage_len: Vec<u32>,
    pub stage: usize,
    stage_week: u32,
    pub controls: Vec<String>,
    pub flags: BTreeSet<String>,
    pub pace: Pace,
    pub oncall: OnCall,
    pub hours_left: i32,
    used: BTreeSet<String>,
    cooldown: BTreeMap<String, u32>,
    pub arcs: Vec<ArcState>,
    pending: Vec<Pending>,
    queue: VecDeque<(String, Option<usize>)>,
    cur: Option<Cur>,
    pub journal: Vec<JournalEntry>,
    out: Vec<String>,
    pub phase: Ctx,
    after: After,
    pub ending: Option<Ending>,
    pub milestones: u32,
    pub surfaced: u32,
    pub timeline_points: i64,
    pub arc_bonus: i64,
    past: Vec<Tombstone>,
    shown_graves: usize,
    last_cause: String,
    pub mentor_left: u32,
    snapshot: Option<Box<Game>>,
    pub hunted_week: u32,
    hunt: Option<(Vec<String>, usize)>,
    hunt_boost: bool,
    hours_debt: i32,
    /// Share of last week's team hours left for shipping, 0 to 1.
    spare_hours: f64,
    order: Option<OrderState>,
    pending_orders: VecDeque<usize>,
    pending_gate: bool,
    gate_fired: bool,
    name_idx: usize,
    pub epitaph: String,
    pub company_name: String,
}

impl Game {
    pub fn new(seed: u64, past: Vec<Tombstone>) -> Game {
        let mut g = Game {
            content: Content::get(),
            seed,
            rng: Rng::new(seed),
            company: Company::Startup,
            level: Level::Beginner,
            team: ROLES
                .iter()
                .map(|(r, n)| Member {
                    name: n.to_string(),
                    role: r,
                    fatigue: 0,
                    gone: false,
                    compromised: false,
                })
                .collect(),
            budget: 0,
            trust: 60,
            servers: 5,
            offline_weeks: 0,
            week: 0,
            trail: Vec::new(),
            stage_len: Vec::new(),
            stage: 0,
            stage_week: 0,
            controls: Vec::new(),
            flags: BTreeSet::new(),
            pace: Pace::Steady,
            oncall: OnCall::Normal,
            hours_left: 0,
            used: BTreeSet::new(),
            cooldown: BTreeMap::new(),
            arcs: Vec::new(),
            pending: Vec::new(),
            queue: VecDeque::new(),
            cur: None,
            journal: Vec::new(),
            out: Vec::new(),
            phase: Ctx::Company,
            after: After::Menu,
            ending: None,
            milestones: 0,
            surfaced: 0,
            timeline_points: 0,
            arc_bonus: 0,
            past,
            shown_graves: 0,
            last_cause: "bad luck".into(),
            mentor_left: 0,
            snapshot: None,
            hunted_week: 0,
            hunt: None,
            hunt_boost: false,
            hours_debt: 0,
            spare_hours: 1.0,
            order: None,
            pending_orders: VecDeque::new(),
            pending_gate: false,
            gate_fired: false,
            name_idx: 0,
            epitaph: String::new(),
            company_name: "Acme".into(),
        };
        g.title();
        g
    }

    // ----- front-end API -------------------------------------------------

    pub fn take_output(&mut self) -> Vec<String> {
        std::mem::take(&mut self.out)
    }

    pub fn is_done(&self) -> bool {
        self.phase == Ctx::Done
    }

    pub fn prompt(&self) -> Prompt {
        let (choices, text) = match self.phase {
            Ctx::Company => (
                self.simple(&["bank", "startup", "garage", "explain"]),
                "What is your choice?".into(),
            ),
            Ctx::Level => (
                self.simple(&["beginner", "intermediate", "advanced", "pro"]),
                "What is your choice?".into(),
            ),
            Ctx::Name => {
                if self.name_idx == 0 {
                    (vec![], "Company name [Acme]:".to_string())
                } else {
                    let i = self.name_idx - 1;
                    (vec![], format!("  {} [{}]:", ROLE_TITLES[i], ROLES[i].1))
                }
            }
            Ctx::Expo => (self.expo_choices(), "Which item?".into()),
            Ctx::Menu => (self.menu_choices(), "What is your choice?".into()),
            Ctx::Pace => (
                self.simple(&["careful", "steady", "crunch"]),
                "What is your choice?".into(),
            ),
            Ctx::OnCall => (
                self.simple(&["generous", "normal", "skeleton"]),
                "What is your choice?".into(),
            ),
            Ctx::Event => (self.event_choices(), "What is your choice?".into()),
            Ctx::Hunt => {
                let n = self.hunt.as_ref().map_or(0, |h| h.0.len());
                let keys: Vec<String> = (1..=n).map(|i| i.to_string()).collect();
                let keys: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
                (self.simple(&keys), "Which line is suspicious?".into())
            }
            Ctx::Order => (vec![], "Order (e.g. 3 1 2):".into()),
            Ctx::Continue => (vec![], "Press ENTER to continue.".into()),
            Ctx::Epitaph => (vec![], "Write an epitaph:".into()),
            Ctx::Done => (vec![], String::new()),
        };
        Prompt {
            ctx: self.phase,
            choices,
            text,
        }
    }

    fn simple(&self, keys: &[&str]) -> Vec<ChoiceInfo> {
        keys.iter()
            .map(|k| ChoiceInfo {
                label: k.to_string(),
                cost: 0,
                hours: 0,
                tags: vec![],
                available: true,
                key: k.to_string(),
            })
            .collect()
    }

    pub fn input(&mut self, raw: &str) {
        let s = raw.trim();
        match self.phase {
            Ctx::Company => match s {
                "1" | "2" | "3" => {
                    self.company = Company::ALL[s.parse::<usize>().unwrap() - 1];
                    self.level_menu();
                }
                "4" => {
                    self.say("");
                    self.say_wrap("A bank spin-off starts with $200,000 and big bills. Two founders in a garage start with $16,000 and small ones. Everything costs more at a bigger company, and the garage has the least room for mistakes. The harder the start, the bigger the score: a bank scores x1, a startup x2, a garage x3.");
                    self.company_menu();
                }
                _ => self.bad_number(4),
            },
            Ctx::Level => match s.parse::<usize>() {
                Ok(n @ 1..=4) => {
                    self.level = Level::ALL[n - 1];
                    self.budget = (self.company.start_budget() as f64 * self.level.budget_share())
                        .round() as i64;
                    self.phase = Ctx::Name;
                    self.say("");
                    self.say("What is your company called, and who is on the team?");
                    self.say("(Press ENTER to keep a name.)");
                }
                _ => self.bad_number(4),
            },
            Ctx::Name => {
                if !s.is_empty() {
                    let name: String = s.chars().take(20).collect();
                    if self.name_idx == 0 {
                        self.company_name = name;
                    } else {
                        self.team[self.name_idx - 1].name = name;
                    }
                }
                self.name_idx += 1;
                if self.name_idx > self.team.len() {
                    self.setup();
                }
            }
            Ctx::Expo => self.expo_input(s),
            Ctx::Menu => self.menu_input(s),
            Ctx::Pace => match s {
                "1" | "2" | "3" => {
                    self.pace = [Pace::Careful, Pace::Steady, Pace::Crunch]
                        [s.parse::<usize>().unwrap() - 1];
                    self.show_menu();
                }
                _ => self.bad_number(3),
            },
            Ctx::OnCall => match s {
                "1" | "2" | "3" => {
                    self.oncall = [OnCall::Generous, OnCall::Normal, OnCall::Skeleton]
                        [s.parse::<usize>().unwrap() - 1];
                    self.show_menu();
                }
                _ => self.bad_number(3),
            },
            Ctx::Event => self.event_input(s),
            Ctx::Hunt => self.hunt_input(s),
            Ctx::Order => self.order_input(s),
            Ctx::Continue => match self.after {
                After::Queue => self.process_queue(),
                After::Menu => self.show_menu(),
            },
            Ctx::Epitaph => {
                self.epitaph = if s.is_empty() {
                    "We were going to fix it next sprint.".into()
                } else {
                    s.chars().take(60).collect()
                };
                self.finish();
            }
            Ctx::Done => {}
        }
    }

    // ----- small helpers -------------------------------------------------

    fn say(&mut self, s: &str) {
        self.out.push(s.to_string());
    }

    fn say_wrap(&mut self, s: &str) {
        for l in wrap(s, WIDTH) {
            self.out.push(l);
        }
    }

    fn bad_number(&mut self, n: usize) {
        self.say(&format!("Please enter a number from 1 to {n}."));
    }

    fn scaled(&self, v: i64) -> i64 {
        (v as f64 * self.company.scale()).round() as i64
    }

    /// An event's dollar cost: company scale, and dearer at higher levels.
    fn event_cost(&self, v: i64) -> i64 {
        (v as f64 * self.company.scale() * [0.8, 1.0, 1.2, 1.4][self.level as usize]).round() as i64
    }

    pub fn has(&self, c: &str) -> bool {
        self.controls.iter().any(|x| x == c) || self.flags.contains(c)
    }

    fn member_with(&self, role: &str) -> Option<usize> {
        self.team.iter().position(|m| !m.gone && m.role == role)
    }

    fn role_name(&self, role: &str) -> String {
        self.team
            .iter()
            .find(|m| m.role == role)
            .map(|m| m.name.clone())
            .unwrap_or_default()
    }

    fn subst(&self, s: &str, target: Option<usize>, down: i32) -> String {
        let who = target
            .map(|t| self.team[t].name.clone())
            .unwrap_or_default();
        s.replace("{who}", &who)
            .replace("{down}", &down.to_string())
            .replace("{founder}", &self.role_name("founder"))
            .replace("{ops}", &self.role_name("ops"))
            .replace("{intern}", &self.role_name("intern"))
            .replace("{company}", &self.company_name)
    }

    pub fn active_members(&self) -> usize {
        self.team.iter().filter(|m| !m.gone).count()
    }

    pub fn stage_id(&self) -> &'static str {
        self.trail[self.stage.min(self.trail.len() - 1)]
    }

    pub fn weekly_revenue(&self) -> i64 {
        let pace = match self.pace {
            Pace::Careful => 0.8,
            Pace::Steady => 1.0,
            Pace::Crunch => 1.25,
        };
        let growth = 1.3f64.powi(self.milestones as i32);
        let trust = (self.trust as f64 / 60.0).clamp(0.3, 1.3);
        let up = self.servers.max(0) as f64 / 5.0;
        // Hours spent on security are hours not spent shipping.
        let shipping = 0.7 + 0.3 * self.spare_hours;
        (5200.0 * self.company.scale() * pace * growth * trust * up * shipping).round() as i64
    }

    pub fn weekly_burn(&self) -> i64 {
        let people = self.active_members() as f64;
        let upkeep: i64 = self
            .controls
            .iter()
            .filter_map(|c| self.content.control(c))
            .map(|c| c.upkeep)
            .sum();
        let hiring = 1.15f64.powi(self.milestones as i32);
        ((2400.0 * hiring + 700.0 * people * hiring.sqrt() + upkeep as f64) * self.company.scale())
            .round() as i64
    }

    fn weekly_hours(&self) -> i32 {
        let per = match self.pace {
            Pace::Crunch => 50,
            _ => 40,
        };
        let total: i32 = self
            .team
            .iter()
            .filter(|m| !m.gone)
            .map(|m| if m.tired() { per - 10 } else { per })
            .sum();
        let oncall = match self.oncall {
            OnCall::Generous => 12,
            OnCall::Normal => 6,
            OnCall::Skeleton => 0,
        };
        // Most of the team's time goes to the product. This is the share
        // you can pull away for security work.
        (total * 3 / 10 - oncall).max(0)
    }

    pub fn total_weeks(&self) -> u32 {
        self.stage_len.iter().sum()
    }

    /// For bots: the answer to the current hunt.
    pub fn hunt_answer(&self) -> Option<usize> {
        self.hunt.as_ref().map(|h| h.1)
    }

    /// For bots: the correct order for the current timeline prompt.
    pub fn order_answer(&self) -> Option<String> {
        let o = self.order.as_ref()?;
        let mut pos: Vec<(usize, usize)> = o
            .shown
            .iter()
            .enumerate()
            .map(|(i, &e)| (e, i + 1))
            .collect();
        pos.sort();
        Some(
            pos.iter()
                .map(|(_, i)| i.to_string())
                .collect::<Vec<_>>()
                .join(" "),
        )
    }

    // ----- title and setup -------------------------------------------------

    fn title(&mut self) {
        for l in [
            "",
            "   ___                              ____      _               ",
            "  / _ \\ _ __ ___  __ _  ___  _ __   / ___|   _| |__   ___ _ __ ",
            " | | | | '__/ _ \\/ _` |/ _ \\| '_ \\ | |  | | | | '_ \\ / _ \\ '__|",
            " | |_| | | |  __/ (_| | (_) | | | || |__| |_| | |_) |  __/ |   ",
            "  \\___/|_|  \\___|\\__, |\\___/|_| |_| \\____\\__, |_.__/ \\___|_|   ",
            "                 |___/   T R A I L S    |___/                 ",
            "",
        ] {
            self.say(l);
        }
        self.say_wrap("You are the first security hire at a young company. The trail runs from launch to the IPO. Along the way: floods of junk traffic, phished engineers, leaked keys, and AI agents with too much power. Every choice costs something.");
        self.company_menu();
    }

    fn company_menu(&mut self) {
        self.phase = Ctx::Company;
        self.say("");
        self.say("Many kinds of companies set out on the trail. You may:");
        self.say("  1. Be a bank spin-off ($200,000)");
        self.say("  2. Be a funded startup ($80,000)");
        self.say("  3. Be two founders in a garage ($16,000)");
        self.say("  4. Find out the differences between these choices");
    }

    fn level_menu(&mut self) {
        self.phase = Ctx::Level;
        self.say("");
        self.say("Choose your level:");
        self.say("  1. Beginner      (first time thinking about security)");
        self.say("  2. Intermediate  (you know what MFA and OAuth are)");
        self.say("  3. Advanced      (you have been on call for an incident)");
        self.say("  4. Pro           (you run incident response)");
    }

    fn setup(&mut self) {
        if self.level == Level::Beginner {
            self.trail = vec!["launch", "signin", "keys"];
            self.stage_len = vec![4, 4, 4];
            self.mentor_left = 1;
        } else {
            self.trail = STAGE_IDS.to_vec();
            self.stage_len = vec![5, 5, 5, 5, 6];
            self.mentor_left = if self.level == Level::Intermediate {
                1
            } else {
                0
            };
        }
        // Pick the incident arcs for this run.
        let total = self.total_weeks();
        let mut eligible: Vec<usize> = (0..self.content.arcs.len())
            .filter(|&i| {
                let a = &self.content.arcs[i];
                a.min_level <= self.level.idx() && a.start[1] + 3 <= total
            })
            .collect();
        self.rng.shuffle(&mut eligible);
        let n = [1, 2, 3, 4][self.level as usize].min(eligible.len());
        eligible.truncate(n);
        eligible.sort();
        for def in eligible {
            let [lo, hi] = self.content.arcs[def].start;
            let start_week = self.rng.range(lo as i64, hi as i64) as u32;
            self.arcs.push(ArcState {
                def,
                start_week,
                progress: 0,
                status: ArcStatus::Dormant,
                timeline: vec![],
            });
        }
        self.say("");
        self.say_wrap(&format!(
            "{} sets out as {}. Before you launch, you visit the Vendor Expo.",
            self.company_name,
            self.company.name()
        ));
        self.expo();
    }

    // ----- the Vendor Expo -------------------------------------------------

    fn expo(&mut self) {
        self.expo_list(true);
    }

    fn expo_list(&mut self, full: bool) {
        self.phase = Ctx::Expo;
        self.say("");
        self.say(&format!("You have {} to spend.", money(self.budget)));
        let items = self.expo_choices();
        let beginner = self.level == Level::Beginner;
        for (i, c) in items.iter().enumerate() {
            if c.key == "done" {
                self.say(&format!("  {}. Done shopping", i + 1));
                continue;
            }
            let def = self.content.control(&c.key[4..]).unwrap();
            let upkeep = if def.upkeep > 0 {
                format!(" + {}/week", money(self.scaled(def.upkeep)))
            } else {
                String::new()
            };
            self.say(&format!(
                "  {}. {} ... {}{}",
                i + 1,
                def.name,
                money(c.cost),
                upkeep
            ));
            if !full {
                continue;
            }
            let blurb = if beginner {
                def.gloss.clone().unwrap_or(def.blurb.clone())
            } else {
                def.blurb.clone()
            };
            for l in wrap(&blurb, WIDTH - 7) {
                self.say(&format!("       {l}"));
            }
        }
    }

    fn expo_choices(&self) -> Vec<ChoiceInfo> {
        let mut v: Vec<ChoiceInfo> = self
            .content
            .controls
            .iter()
            .filter(|c| !self.has(&c.id))
            .map(|c| {
                let cost = self.scaled(c.cost);
                ChoiceInfo {
                    label: c.name.clone(),
                    cost,
                    hours: 0,
                    tags: vec![],
                    available: cost <= self.budget,
                    key: format!("buy:{}", c.id),
                }
            })
            .collect();
        v.push(ChoiceInfo {
            label: "Done shopping".into(),
            cost: 0,
            hours: 0,
            tags: vec![],
            available: true,
            key: "done".into(),
        });
        v
    }

    fn expo_input(&mut self, s: &str) {
        let items = self.expo_choices();
        let Some(c) = s
            .parse::<usize>()
            .ok()
            .and_then(|n| n.checked_sub(1))
            .and_then(|i| items.get(i))
        else {
            return self.bad_number(items.len());
        };
        if c.key == "done" {
            if self.week == 0 {
                self.say("");
                self.say_wrap("You set out on the trail.");
            }
            return self.show_menu();
        }
        if !c.available {
            self.say("You can't afford that.");
            return;
        }
        self.budget -= c.cost;
        self.controls.push(c.key[4..].to_string());
        self.say(&format!("Bought: {}.", c.label));
        self.expo_list(false);
    }

    // ----- the weekly menu -------------------------------------------------

    fn show_menu(&mut self) {
        self.phase = Ctx::Menu;
        self.status();
        let items = self.menu_choices();
        for (i, c) in items.iter().enumerate() {
            self.say(&format!("  {}. {}", i + 1, c.label));
        }
    }

    fn status(&mut self) {
        let st = stage_info(self.stage_id());
        self.say("");
        self.say(&format!(
            "WEEK {}   Stage: {}   Budget: {}   Servers: {}/5   Trust: {}",
            self.week + 1,
            st.name,
            money(self.budget),
            self.servers,
            self.trust
        ));
        let team: Vec<String> = self
            .team
            .iter()
            .map(|m| {
                let st = if m.gone {
                    "gone"
                } else if m.tired() {
                    "tired"
                } else {
                    "fine"
                };
                format!("{} ({st})", m.name)
            })
            .collect();
        self.say_wrap(&format!("Team: {}", team.join(" ")));
        let pace = format!("{:?}", self.pace).to_lowercase();
        let oncall = format!("{:?}", self.oncall).to_lowercase();
        self.say(&format!(
            "Pace: {pace}   On-call: {oncall}   Security hours: {} a week",
            self.weekly_hours()
        ));
    }

    fn menu_choices(&self) -> Vec<ChoiceInfo> {
        let mut v = vec![
            ("continue", "Continue on the trail".to_string()),
            ("supplies", "Check supplies".to_string()),
            ("map", "Look at the map".to_string()),
            ("pace", "Change shipping pace".to_string()),
            ("oncall", "Change on-call rotation".to_string()),
            (
                "hunt",
                "Hunt for threats (8 hours of next week's security time)".to_string(),
            ),
            ("rest", "Rest the team for a week".to_string()),
        ];
        // The mentor can only rewind a decision made this week.
        if self.mentor_left > 0 && self.snapshot.as_ref().is_some_and(|s| s.week == self.week) {
            v.push((
                "mentor",
                format!(
                    "Ask the mentor to rewind your last decision ({} left)",
                    self.mentor_left
                ),
            ));
        }
        v.into_iter()
            .map(|(k, l)| ChoiceInfo {
                label: l,
                cost: 0,
                hours: if k == "hunt" { 8 } else { 0 },
                tags: vec![],
                available: k != "hunt" || self.hunted_week != self.week + 1,
                key: k.into(),
            })
            .collect()
    }

    fn menu_input(&mut self, s: &str) {
        let items = self.menu_choices();
        let Some(c) = s
            .parse::<usize>()
            .ok()
            .and_then(|n| n.checked_sub(1))
            .and_then(|i| items.get(i))
            .cloned()
        else {
            return self.bad_number(items.len());
        };
        match c.key.as_str() {
            "continue" => self.advance_week(false),
            "rest" => {
                self.say("");
                self.say("The team rests. Nothing ships this week.");
                self.advance_week(true)
            }
            "supplies" => {
                self.supplies();
                self.after = After::Menu;
                self.phase = Ctx::Continue;
            }
            "map" => {
                self.map();
                self.after = After::Menu;
                self.phase = Ctx::Continue;
            }
            "pace" => {
                self.phase = Ctx::Pace;
                self.say("");
                self.say("Change shipping pace:");
                self.say("  1. Careful (less revenue, fewer incidents, rested team)");
                self.say("  2. Steady");
                self.say("  3. Crunch (more revenue and hours, more incidents, tired team)");
            }
            "oncall" => {
                self.phase = Ctx::OnCall;
                self.say("");
                self.say("Change on-call rotation:");
                self.say("  1. Generous (30 hours a week watching; attackers move slower)");
                self.say("  2. Normal (15 hours a week)");
                self.say("  3. Skeleton (nobody watching; attackers move faster)");
            }
            "hunt" => {
                if !c.available {
                    self.say("You already hunted this week.");
                    return;
                }
                self.start_hunt();
            }
            "mentor" => self.use_mentor(),
            _ => {}
        }
    }

    fn supplies(&mut self) {
        self.say("");
        self.say(&format!("Budget: {}", money(self.budget)));
        self.say(&format!(
            "Each week: about {} in, {} out (salaries, servers, upkeep)",
            money(self.weekly_revenue()),
            money(self.weekly_burn())
        ));
        self.say(&format!(
            "Servers up: {}/5   Trust: {}/100",
            self.servers, self.trust
        ));
        if self.controls.is_empty() {
            self.say("Controls: none");
        } else {
            self.say("Controls:");
            let names: Vec<String> = self
                .controls
                .iter()
                .filter_map(|c| self.content.control(c))
                .map(|c| c.name.clone())
                .collect();
            for n in names {
                self.say(&format!("  - {n}"));
            }
        }
    }

    fn map(&mut self) {
        self.say("");
        self.say("THE TRAIL");
        let mut start = 1;
        for i in 0..self.trail.len() {
            let st = stage_info(self.trail[i]);
            let end = start + self.stage_len[i] - 1;
            let mark = if i < self.stage {
                "done"
            } else if i == self.stage {
                "you are here"
            } else {
                ""
            };
            let milestone = if i + 1 == self.trail.len() {
                "the IPO"
            } else {
                st.milestone
            };
            self.say(&format!(
                "  Weeks {start:>2}-{end:<2}  {:<26} -> {}  {}",
                st.name, milestone, mark
            ));
            start = end + 1;
        }
    }

    // ----- the week ----------------------------------------------------------

    fn advance_week(&mut self, rest: bool) {
        if self.week > 0 {
            self.spare_hours =
                (self.hours_left.max(0) as f64 / self.weekly_hours().max(1) as f64).clamp(0.0, 1.0);
        }
        self.week += 1;
        self.stage_week += 1;
        self.hours_left = self.weekly_hours() - std::mem::take(&mut self.hours_debt);
        self.say("");
        self.say(&format!("--- Week {} ---", self.week));

        // Money.
        let mut revenue = self.weekly_revenue();
        if rest {
            revenue /= 2;
        }
        let burn = self.weekly_burn();
        self.budget += revenue - burn;
        self.say(&format!(
            "Revenue {} in, costs {} out.",
            money(revenue),
            money(burn)
        ));

        // People.
        let delta = if rest {
            -3
        } else {
            let pace = match self.pace {
                Pace::Careful => -1,
                Pace::Steady => 0,
                Pace::Crunch => 2,
            };
            let oncall = match self.oncall {
                OnCall::Generous => -1,
                OnCall::Normal => 0,
                OnCall::Skeleton => 1,
            };
            pace + oncall
        };
        for i in 0..self.team.len() {
            if self.team[i].gone {
                continue;
            }
            let was_tired = self.team[i].tired();
            self.team[i].fatigue = (self.team[i].fatigue + delta).max(0);
            if self.team[i].fatigue >= 8 && self.rng.chance(0.3) {
                self.team[i].gone = true;
                let n = self.team[i].name.clone();
                self.say(&format!("{n} has burned out and quit."));
                self.last_cause = "burnout".into();
            } else if self.team[i].tired() && !was_tired {
                let n = self.team[i].name.clone();
                self.say(&format!("{n} is tired."));
            }
        }

        // Servers come back on their own, faster with ops around.
        if self.servers < 5 {
            let mut back = 1;
            if self.member_with("ops").is_some() {
                back += 1;
            }
            self.servers = (self.servers + back).min(5);
        }
        if self.servers == 0 {
            self.offline_weeks += 1;
        } else {
            self.offline_weeks = 0;
        }
        // Trust drifts back towards 55: incidents fade, and so does goodwill.
        if self.servers == 5 && self.trust < 55 && self.pace != Pace::Crunch {
            self.trust += 1;
        } else if self.trust > 55 && self.level > Level::Beginner {
            self.trust -= 1;
        }

        // Graves of earlier runs.
        if self.shown_graves < self.past.len() && self.rng.chance(0.15) {
            let t = self.past[self.shown_graves].clone();
            self.shown_graves += 1;
            self.say_wrap(&format!(
                "You pass the grave of {}. Died of {} in week {}. \"{}\"",
                t.name.to_uppercase(),
                t.cause,
                t.week,
                t.epitaph
            ));
        }

        // Things that were waiting to happen.
        let due: Vec<Pending> = self
            .pending
            .iter()
            .filter(|p| p.due <= self.week)
            .cloned()
            .collect();
        self.pending.retain(|p| p.due > self.week);
        for p in due {
            if let Some(ev) = &p.event {
                self.queue.push_back((ev.clone(), None));
            } else {
                if !p.text.is_empty() {
                    self.say_wrap(&p.text);
                }
                self.apply_effect(&p.effect, None, None, &p.cause);
            }
        }

        // The hidden attacker.
        let mut arc_fired = false;
        for a in 0..self.arcs.len() {
            if self.advance_arc(a) {
                arc_fired = true;
            }
        }
        self.hunt_boost = false;

        // A random event.
        let mut p = 0.6
            + self.level.event_pressure()
            + match self.pace {
                Pace::Careful => -0.1,
                Pace::Steady => 0.0,
                Pace::Crunch => 0.15,
            };
        if rest {
            p /= 2.0;
        }
        if arc_fired {
            p /= 2.0;
        }
        if self.rng.chance(p)
            && let Some(id) = self.draw_event()
        {
            self.queue.push_back((id.clone(), None));
            // At the top levels, trouble comes in pairs.
            if self.level >= Level::Advanced
                && self.rng.chance(p - 0.55)
                && let Some(id2) = self.draw_event()
                && id2 != id
            {
                self.queue.push_back((id2, None));
            }
        }

        if self.stage_week >= self.stage_len[self.stage] {
            self.pending_gate = true;
        }
        self.process_queue();
    }

    fn when_ok(&self, e: &EventDef) -> bool {
        let w = &e.when;
        w.controls_missing.iter().all(|c| !self.has(c))
            && w.controls_present.iter().all(|c| self.has(c))
            && w.flags.iter().all(|f| self.flags.contains(f))
            && w.not_flags.iter().all(|f| !self.flags.contains(f))
            && w.min_week.is_none_or(|m| self.week >= m)
            && w.needs_logging_from_level
                .is_none_or(|l| self.level.idx() < l || self.has("logging"))
            && (self.level.idx() >= e.min_level && self.level.idx() <= e.max_level)
            && self.target_ok(e)
    }

    fn target_ok(&self, e: &EventDef) -> bool {
        match e.target.as_deref() {
            None | Some("random") => self.active_members() > 0,
            Some(r) => self.member_with(r).is_some(),
        }
    }

    fn draw_event(&mut self) -> Option<String> {
        let stage = self.stage_id();
        let cands: Vec<(usize, u32)> = self
            .content
            .events
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.kind == "random"
                    && (e.stage == stage || e.stage == "any")
                    && self.when_ok(e)
                    && if e.repeat {
                        self.cooldown.get(&e.id).is_none_or(|&w| self.week >= w)
                    } else {
                        !self.used.contains(&e.id)
                    }
            })
            .map(|(i, e)| (i, e.weight))
            .collect();
        let total: u32 = cands.iter().map(|c| c.1).sum();
        if total == 0 {
            return None;
        }
        let mut pick = self.rng.range(0, total as i64 - 1) as u32;
        for (i, w) in cands {
            if pick < w {
                return Some(self.content.events[i].id.clone());
            }
            pick -= w;
        }
        None
    }

    /// Maybe move an arc forward one beat. Returns true if a beat event was
    /// queued.
    fn advance_arc(&mut self, a: usize) -> bool {
        let def = &self.content.arcs[self.arcs[a].def];
        match self.arcs[a].status {
            ArcStatus::Dormant if self.week >= self.arcs[a].start_week => {
                self.arcs[a].status = ArcStatus::Active;
            }
            ArcStatus::Active => {
                let mut p = [0.30, 0.38, 0.45, 0.55][self.level as usize];
                p += 0.1 * def.weak_to.iter().filter(|c| !self.has(c)).count() as f64;
                p -= 0.1 * def.slowed_by.iter().filter(|c| self.has(c)).count() as f64;
                if self.team.iter().any(|m| !m.gone && m.tired()) {
                    p += 0.1;
                }
                if self.team.iter().any(|m| !m.gone && m.compromised) {
                    p += 0.15;
                }
                p += match self.pace {
                    Pace::Crunch => 0.05,
                    _ => 0.0,
                };
                p += match self.oncall {
                    OnCall::Generous => -0.1,
                    OnCall::Normal => 0.0,
                    OnCall::Skeleton => 0.1,
                };
                if self.hunt_boost {
                    p -= 0.15;
                }
                if !self.rng.chance(p.clamp(0.05, 0.85)) {
                    return false;
                }
            }
            _ => return false,
        }
        let beat = self.arcs[a].progress;
        self.arcs[a].progress += 1;
        let ev = def.beat[beat]
            .events
            .iter()
            .find(|id| self.when_ok(self.content.event(id)))
            .cloned();
        self.arcs[a].timeline.push(TimelineEntry {
            week: self.week,
            beat,
            seen: ev.is_some(),
            choice: None,
        });
        match ev {
            Some(id) => {
                self.queue.push_back((id, Some(a)));
                true
            }
            None => false,
        }
    }

    fn process_queue(&mut self) {
        if self.check_end() {
            return;
        }
        while let Some((id, arc)) = self.queue.pop_front() {
            if self.present_event(&id, arc) {
                return;
            }
        }
        if let Some(a) = self.pending_orders.pop_front() {
            return self.review_arc(a);
        }
        if self.pending_gate {
            self.pending_gate = false;
            self.gate_fired = true;
            let stage = self.stage_id();
            if let Some(gate) = self
                .content
                .events
                .iter()
                .find(|e| e.kind == "gate" && e.stage == stage)
            {
                let id = gate.id.clone();
                if self.present_event(&id, None) {
                    return;
                }
            }
        }
        if self.gate_fired {
            self.gate_fired = false;
            return self.milestone();
        }
        self.show_menu();
    }

    fn check_end(&mut self) -> bool {
        if self.ending.is_some() {
            return true;
        }
        let ending = if self.budget < 0 {
            Some(if self.trust >= 70 {
                Ending::Acquired
            } else {
                Ending::Bankrupt
            })
        } else if self.trust < 20 {
            Some(Ending::LostCustomers)
        } else if self.offline_weeks >= 2 {
            Some(Ending::Offline)
        } else if self.active_members() == 0 {
            Some(Ending::EveryoneLeft)
        } else {
            None
        };
        match ending {
            Some(e) => {
                self.ending = Some(e);
                self.end_run();
                true
            }
            None => false,
        }
    }

    fn milestone(&mut self) {
        self.milestones += 1;
        self.trust = (self.trust + 5).min(100);
        self.stage += 1;
        self.stage_week = 0;
        let last = self.stage >= self.trail.len();
        if last {
            self.ending = Some(Ending::Ipo);
            return self.end_run();
        }
        let st = stage_info(self.trail[self.stage - 1]);
        self.say("");
        self.say_wrap(&format!(
            "You have reached {}! Revenue grows and trust rises.",
            st.milestone
        ));
        let next = stage_info(self.trail[self.stage]);
        self.say_wrap(&format!("Next stage: {}.", next.name));
        if self.level == Level::Beginner {
            self.mentor_left = 1;
        }
        self.say("");
        self.say("A vendor stop. You may buy controls before you go on.");
        self.expo();
    }

    // ----- events ------------------------------------------------------------

    fn option_visible(&self, o: &OptionDef) -> bool {
        o.requires_control.as_ref().is_none_or(|c| self.has(c))
            && o.requires_missing.as_ref().is_none_or(|c| !self.has(c))
            && o.requires_flag
                .as_ref()
                .is_none_or(|f| self.flags.contains(f))
            && o.role
                .as_ref()
                .is_none_or(|r| self.member_with(r).is_some())
    }

    fn option_available(&self, o: &OptionDef) -> bool {
        self.event_cost(o.cost) <= self.budget.max(0) && o.hours <= self.hours_left
    }

    /// Returns false if the event could not be shown (no one to target).
    fn present_event(&mut self, id: &str, arc: Option<usize>) -> bool {
        let content = self.content;
        let Some(&ev) = content.by_id.get(id) else {
            return false;
        };
        let def = &content.events[ev];
        if !self.target_ok(def) {
            return false;
        }
        if def.kind == "random" {
            if def.repeat {
                self.cooldown.insert(def.id.clone(), self.week + 4);
            } else {
                self.used.insert(def.id.clone());
            }
        }
        let target = match def.target.as_deref() {
            None => None,
            Some("random") => {
                let alive: Vec<usize> = (0..self.team.len())
                    .filter(|&i| !self.team[i].gone)
                    .collect();
                Some(alive[self.rng.index(alive.len())])
            }
            Some(r) => {
                let alive: Vec<usize> = (0..self.team.len())
                    .filter(|&i| !self.team[i].gone && self.team[i].role == r)
                    .collect();
                Some(alive[self.rng.index(alive.len())])
            }
        };
        let mut down = 0;
        if let Some([lo, hi]) = def.knock_servers {
            down = self.rng.range(lo as i64, hi as i64) as i32;
            self.servers = (self.servers - down).max(0);
        }
        let opts: Vec<usize> = (0..def.option.len())
            .filter(|&i| self.option_visible(&def.option[i]))
            .collect();
        self.cur = Some(Cur {
            ev,
            target,
            down,
            arc,
            opts,
        });
        if self.mentor_left > 0 && self.level <= Level::Intermediate {
            let mut snap = self.clone();
            snap.snapshot = None;
            snap.out.clear();
            self.snapshot = Some(Box::new(snap));
        }
        self.render_event();
        self.phase = Ctx::Event;
        true
    }

    fn render_event(&mut self) {
        let cur = self.cur.clone().unwrap();
        let def = &self.content.events[cur.ev];
        self.say("");
        let head = if def.kind == "gate" {
            def.title.to_uppercase()
        } else {
            format!("WEEK {}. {}", self.week, def.title.to_uppercase())
        };
        self.say(&head);
        let mut text = self.subst(&def.text, cur.target, cur.down);
        if def.clue && self.level == Level::Beginner {
            text = format!("CLUE: {text}");
        }
        self.say_wrap(&text);
        if self.level <= Level::Intermediate
            && let Some(g) = &def.gloss
        {
            self.say_wrap(&format!("({g})"));
        }
        if def.breach && self.level == Level::Pro {
            self.say("The 72-hour disclosure clock is running.");
        }
        self.say("");
        let choices = self.event_choices();
        for (i, c) in choices.iter().enumerate() {
            let o = &def.option[cur.opts[i]];
            let mut parts = Vec::new();
            if o.cost > 0 {
                parts.push(match self.level {
                    Level::Beginner | Level::Intermediate => money(c.cost),
                    Level::Advanced => format!(
                        "{} to {}",
                        money(round_sig(c.cost * 7 / 10)),
                        money(round_sig(c.cost * 13 / 10))
                    ),
                    Level::Pro => "cost unknown".into(),
                });
            }
            if o.hours > 0 {
                parts.push(if o.hours == 1 {
                    "1 hour".to_string()
                } else {
                    format!("{} hours", o.hours)
                });
            }
            if let Some(r) = &o.role
                && let Some(m) = self.member_with(r)
            {
                parts.push(self.team[m].name.clone());
            }
            let mut line = format!("{}. {}", i + 1, c.label);
            if !parts.is_empty() {
                line.push_str(&format!(" ({})", parts.join(", ")));
            }
            if !c.available {
                line.push_str(if o.hours > self.hours_left {
                    " [not enough hours]"
                } else {
                    " [can't afford]"
                });
            }
            for (j, l) in wrap(&line, WIDTH - 5).into_iter().enumerate() {
                self.say(&if j == 0 {
                    format!("  {l}")
                } else {
                    format!("     {l}")
                });
            }
            if self.level == Level::Beginner
                && let Some(h) = &o.hint
            {
                self.say(&format!("     Note: {h}"));
            }
        }
    }

    fn event_choices(&self) -> Vec<ChoiceInfo> {
        let Some(cur) = &self.cur else { return vec![] };
        let def = &self.content.events[cur.ev];
        cur.opts
            .iter()
            .map(|&i| {
                let o = &def.option[i];
                ChoiceInfo {
                    label: self.subst(&o.label, cur.target, cur.down),
                    cost: self.event_cost(o.cost),
                    hours: o.hours,
                    tags: o.tags.clone(),
                    available: self.option_available(o),
                    key: format!("opt:{i}"),
                }
            })
            .collect()
    }

    fn event_input(&mut self, s: &str) {
        let cur = self.cur.clone().unwrap();
        let content = self.content;
        let def = &content.events[cur.ev];
        let Some(&oi) = s
            .parse::<usize>()
            .ok()
            .and_then(|n| n.checked_sub(1))
            .and_then(|i| cur.opts.get(i))
        else {
            return self.bad_number(cur.opts.len());
        };
        let o = &def.option[oi];
        if !self.option_available(o) {
            self.say(if o.hours > self.hours_left {
                "You don't have the team hours for that this week."
            } else {
                "You can't afford that."
            });
            return;
        }
        let cause = def
            .cause
            .clone()
            .unwrap_or_else(|| def.title.to_lowercase());
        self.budget -= self.event_cost(o.cost);
        self.hours_left -= o.hours;
        self.say("");
        let mut outcome = self.subst(&o.outcome, cur.target, cur.down);
        if !outcome.is_empty() {
            self.say_wrap(&outcome);
        }
        self.apply_effect(&o.effect, cur.target, cur.arc, &cause);
        for c in &o.chance {
            if c.unless_control.as_ref().is_some_and(|u| self.has(u))
                || c.unless_flag
                    .as_ref()
                    .is_some_and(|f| self.flags.contains(f))
            {
                continue;
            }
            if !self.rng.chance(c.p) {
                continue;
            }
            let text = self.subst(&c.outcome, cur.target, cur.down);
            if c.delay == 0 {
                if !text.is_empty() {
                    self.say_wrap(&text);
                    outcome = format!("{outcome} {text}");
                }
                self.apply_effect(&c.effect, cur.target, cur.arc, &cause);
                if let Some(ev) = &c.event {
                    self.queue.push_back((ev.clone(), None));
                }
            } else {
                self.pending.push(Pending {
                    due: self.week + c.delay,
                    text,
                    effect: c.effect.clone(),
                    event: c.event.clone(),
                    cause: cause.clone(),
                });
            }
        }
        if def.breach && self.level == Level::Pro && !o.tags.iter().any(|t| t == "discloses") {
            self.pending.push(Pending {
                due: self.week + 1,
                text: "The 72-hour disclosure deadline passed without notice to customers. The regulator fines you.".into(),
                effect: Effect { budget: -10_000, trust: -5, ..Default::default() },
                event: None,
                cause: cause.clone(),
            });
        }
        let textbook = o.tags.iter().any(|t| t == "textbook");
        let alternative = if textbook {
            None
        } else {
            cur.opts
                .iter()
                .map(|&i| &def.option[i])
                .find(|x| x.tags.iter().any(|t| t == "textbook"))
                .map(|x| self.subst(&x.label, cur.target, cur.down))
        };
        let label = self.subst(&o.label, cur.target, cur.down);
        if let Some(a) = cur.arc
            && let Some(t) = self.arcs[a].timeline.last_mut()
        {
            t.choice = Some(label.clone());
        }
        self.journal.push(JournalEntry {
            week: self.week,
            title: def.title.clone(),
            choice: label,
            outcome,
            note: def.note.as_ref().map(|n| n.text.clone()),
            source: def.note.as_ref().and_then(|n| n.source.clone()),
            textbook,
            alternative,
        });
        let show_note = match self.level {
            Level::Beginner | Level::Intermediate => true,
            Level::Advanced => !textbook,
            Level::Pro => false,
        };
        if show_note && let Some(n) = &def.note {
            self.say("");
            self.say_wrap(&format!("Field note. {}", n.text));
            if let Some(src) = &n.source {
                self.say_wrap(&format!("(Source: {src})"));
            }
        }
        // An arc's objective ends the arc.
        if let Some(a) = cur.arc {
            let arc_def = &content.arcs[self.arcs[a].def];
            if self.arcs[a].status == ArcStatus::Active
                && self.arcs[a].progress >= arc_def.beat.len()
            {
                self.arcs[a].status = ArcStatus::Succeeded;
                self.pending_orders.push_back(a);
            }
        }
        self.cur = None;
        self.after = After::Queue;
        self.phase = Ctx::Continue;
    }

    fn apply_effect(&mut self, e: &Effect, target: Option<usize>, arc: Option<usize>, cause: &str) {
        let before = (self.budget, self.trust, self.servers);
        self.budget += if e.budget < 0 {
            self.event_cost(e.budget)
        } else {
            self.scaled(e.budget)
        };
        let trust = if e.trust < 0 {
            (e.trust as f64 * self.level.harm()).round() as i32
        } else {
            e.trust
        };
        self.trust = (self.trust + trust).clamp(0, 100);
        if e.restore_servers {
            self.servers = 5;
        }
        self.servers = (self.servers + e.servers).clamp(0, 5);
        if e.revenue_weeks_lost > 0 {
            self.budget -= self.weekly_revenue() * e.revenue_weeks_lost as i64;
        }
        if let Some(t) = target {
            self.team[t].fatigue += e.fatigue;
            if e.compromise_target {
                self.team[t].compromised = true;
            }
            if e.remove_target && !self.team[t].gone {
                self.team[t].gone = true;
                let n = self.team[t].name.clone();
                self.say(&format!("{n} has left the company."));
            }
        }
        if e.fatigue_all != 0 {
            for m in self.team.iter_mut().filter(|m| !m.gone) {
                m.fatigue = (m.fatigue + e.fatigue_all).max(0);
            }
        }
        if e.clear_compromised {
            for m in &mut self.team {
                m.compromised = false;
            }
        }
        if let Some(c) = &e.add_control
            && !self.has(c)
        {
            self.controls.push(c.clone());
        }
        if let Some(c) = &e.remove_control {
            self.controls.retain(|x| x != c);
        }
        for f in &e.set_flag {
            self.flags.insert(f.clone());
        }
        for f in &e.clear_flag {
            self.flags.remove(f);
        }
        if e.surfaced {
            self.surfaced += 1;
        }
        if e.knock_back
            && let Some(a) = arc
            && self.arcs[a].status == ArcStatus::Active
        {
            self.arcs[a].status = ArcStatus::Stopped;
            let beats = self.content.arcs[self.arcs[a].def].beat.len();
            self.arc_bonus += 100 * (beats - self.arcs[a].progress + 1) as i64;
            self.pending_orders.push_back(a);
        }
        if self.budget < before.0 - self.scaled(2000)
            || self.trust < before.1 - 2
            || self.servers < before.2
        {
            self.last_cause = cause.to_string();
        }
    }

    // ----- arc review (the timeline) -----------------------------------------

    fn review_arc(&mut self, a: usize) {
        let def = &self.content.arcs[self.arcs[a].def];
        let n = self.arcs[a].timeline.len();
        self.say("");
        let how = if self.arcs[a].status == ArcStatus::Stopped {
            "stopped"
        } else {
            "reached its objective"
        };
        self.say(&format!(
            "INCIDENT REVIEW: {} ({how})",
            def.name.to_uppercase()
        ));
        if self.level >= Level::Advanced && n >= 2 {
            let mut shown: Vec<usize> = (0..n).collect();
            while shown.iter().enumerate().all(|(i, &e)| i == e) {
                self.rng.shuffle(&mut shown);
            }
            self.say_wrap(
                "Reconstruct the attacker's steps. Put these in the order they happened.",
            );
            for (i, &e) in shown.iter().enumerate() {
                let label = def.beat[self.arcs[a].timeline[e].beat].label.clone();
                self.say_wrap(&format!("  {}. {}", i + 1, label));
            }
            self.order = Some(OrderState { arc: a, shown });
            self.phase = Ctx::Order;
            return;
        }
        self.render_timeline(a);
        self.after = After::Queue;
        self.phase = Ctx::Continue;
    }

    fn render_timeline(&mut self, a: usize) {
        let def = &self.content.arcs[self.arcs[a].def];
        let entries = self.arcs[a].timeline.clone();
        let mut first_chance = None;
        for t in &entries {
            let label = &def.beat[t.beat].label;
            let what = match (&t.choice, t.seen) {
                (Some(c), _) => format!("you chose \"{c}\""),
                (None, true) => "you saw it".into(),
                (None, false) => "unseen".into(),
            };
            self.say_wrap(&format!("  Week {:>2}  {label} ({what})", t.week));
            if t.seen && first_chance.is_none() {
                first_chance = Some(t.week);
            }
        }
        if self.arcs[a].status == ArcStatus::Succeeded {
            match first_chance {
                Some(w) if w < entries.last().map_or(0, |t| t.week) => {
                    self.say_wrap(&format!("The first chance to stop it was week {w}."))
                }
                _ => self.say_wrap(
                    "You never saw it coming. Logging and hunting would have shown you more.",
                ),
            }
        }
    }

    fn order_input(&mut self, s: &str) {
        let o = self.order.clone().unwrap();
        let picks: Vec<usize> = s
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|x| x.parse().ok())
            .collect();
        let n = o.shown.len();
        let mut sorted = picks.clone();
        sorted.sort();
        if sorted != (1..=n).collect::<Vec<_>>() {
            self.say(&format!(
                "Enter each number from 1 to {n} once, in order, e.g. {}.",
                (1..=n)
                    .rev()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
            return;
        }
        let right = picks
            .iter()
            .enumerate()
            .filter(|&(pos, &p)| o.shown[p - 1] == pos)
            .count();
        let points = 50 * right as i64;
        self.timeline_points += points;
        self.say("");
        self.say(&format!(
            "You placed {right} of {n} steps correctly (+{points} points). What happened:"
        ));
        self.render_timeline(o.arc);
        self.order = None;
        self.after = After::Queue;
        self.phase = Ctx::Continue;
    }

    // ----- threat hunting ------------------------------------------------------

    fn start_hunt(&mut self) {
        let n = [6, 8, 10, 12][self.level as usize];
        let subtle = self.level >= Level::Advanced;
        let (lines, answer) = logs::generate(&mut self.rng, n, subtle);
        // The hunt happens before the coming week's work, so it spends
        // that week's hours.
        self.hunted_week = self.week + 1;
        self.hours_debt += 8;
        self.say("");
        self.say("THREAT HUNT. One of these log lines is suspicious:");
        for (i, l) in lines.iter().enumerate() {
            self.say(&format!("  {:>2}. {l}", i + 1));
        }
        self.hunt = Some((lines, answer));
        self.phase = Ctx::Hunt;
    }

    fn hunt_input(&mut self, s: &str) {
        let (lines, answer) = self.hunt.clone().unwrap();
        let Some(pick) = s
            .parse::<usize>()
            .ok()
            .and_then(|n| n.checked_sub(1))
            .filter(|&i| i < lines.len())
        else {
            return self.bad_number(lines.len());
        };
        self.hunt = None;
        self.say("");
        if pick == answer {
            self.hunt_boost = true;
            let active: Vec<usize> = (0..self.arcs.len())
                .filter(|&a| self.arcs[a].status == ArcStatus::Active && self.arcs[a].progress >= 2)
                .collect();
            if let Some(&a) = active.first() {
                let name = self.content.arcs[self.arcs[a].def].name.clone();
                self.say_wrap(&format!("Good eye. That line leads you to an attacker already inside: {name}. You trace them and shut them out."));
                self.arcs[a].status = ArcStatus::Stopped;
                let beats = self.content.arcs[self.arcs[a].def].beat.len();
                self.arc_bonus += 100 * (beats - self.arcs[a].progress + 1) as i64;
                for m in &mut self.team {
                    m.compromised = false;
                }
                self.pending_orders.push_back(a);
                self.after = After::Queue;
            } else {
                self.say_wrap("Good eye. It was a probe, and you blocked it. Attackers will find it harder next week.");
                self.after = After::Menu;
            }
        } else {
            self.say_wrap(&format!(
                "A false lead. The suspicious line was number {}:",
                answer + 1
            ));
            self.say(&format!("  {}", lines[answer]));
            self.after = After::Menu;
        }
        self.phase = Ctx::Continue;
    }

    // ----- the mentor ------------------------------------------------------------

    fn use_mentor(&mut self) {
        let Some(snap) = self.snapshot.take() else {
            return;
        };
        let left = self.mentor_left - 1;
        *self = *snap;
        self.mentor_left = left;
        self.snapshot = None;
        self.out.clear();
        self.say("");
        self.say_wrap("The mentor rewinds the trail to your last decision. Choose again.");
        self.render_event();
        self.phase = Ctx::Event;
    }

    // ----- endings ------------------------------------------------------------------

    fn end_run(&mut self) {
        let e = self.ending.unwrap();
        self.say("");
        match e {
            Ending::Ipo => {
                self.say_wrap(&format!("Congratulations! {} has reached the IPO.", self.company_name));
                return self.finish();
            }
            Ending::Bankrupt => self.say_wrap(&format!("{} has run out of money.", self.company_name)),
            Ending::Acquired => self.say_wrap(&format!("{} has run out of money, but customers trust you. A bigger company buys you for parts.", self.company_name)),
            Ending::LostCustomers => self.say_wrap("Your customers have lost faith in you and left."),
            Ending::Offline => self.say_wrap("Your servers have been down for two weeks. There is no company left to run."),
            Ending::EveryoneLeft => self.say_wrap("Everyone has left. The lights are off."),
        }
        if e == Ending::Acquired {
            return self.finish();
        }
        self.phase = Ctx::Epitaph;
    }

    fn finish(&mut self) {
        let e = self.ending.unwrap();
        if !e.won() && e != Ending::Acquired {
            let t = self.tombstone();
            self.say("");
            for l in tombstone(&t.name, &t.cause, t.week, &t.epitaph) {
                self.say(&l);
            }
        }
        self.say("");
        for l in self.score_lines() {
            self.say(&l);
        }
        self.say("");
        self.say("Your trail journal lists every decision and what it taught.");
        self.phase = Ctx::Done;
    }

    pub fn tombstone(&self) -> Tombstone {
        Tombstone {
            name: self.company_name.clone(),
            cause: self.last_cause.clone(),
            week: self.week,
            epitaph: self.epitaph.clone(),
        }
    }

    pub fn survivors(&self) -> i64 {
        self.active_members() as i64
    }

    pub fn score(&self) -> i64 {
        let won = self.ending.is_some_and(|e| e.won());
        let base = self.trust as i64 * 10
            + (self.budget.max(0) as f64 / (100.0 * self.company.scale())) as i64
            + self.survivors() * 200
            + self.milestones as i64 * 300
            + self.arc_bonus
            + self.timeline_points
            - self.surfaced as i64 * 50;
        let base = if won { base } else { base / 4 };
        base.max(0) * self.company.multiplier() * self.level.multiplier()
    }

    fn score_lines(&self) -> Vec<String> {
        vec![
            "SCORE".into(),
            format!(
                "  Trust {} x 10, survivors {} x 200, milestones {} x 300",
                self.trust,
                self.survivors(),
                self.milestones
            ),
            format!(
                "  Money left, attacks stopped (+{}), timelines (+{}), cover-ups found (-{})",
                self.arc_bonus,
                self.timeline_points,
                self.surfaced as i64 * 50
            ),
            format!(
                "  x{} for {}, x{} for {}{}",
                self.company.multiplier(),
                self.company.name(),
                self.level.multiplier(),
                self.level.name(),
                if self.ending.is_some_and(|e| e.won()) {
                    ""
                } else {
                    ", quartered for not finishing"
                }
            ),
            format!("  Final score: {}", self.score()),
        ]
    }

    pub fn journal_markdown(&self) -> String {
        let mut s = format!(
            "# Trail journal: {}\n\n{} as {}, level {}, seed {}.\n\n",
            self.company_name,
            self.company_name,
            self.company.name(),
            self.level.name(),
            self.seed
        );
        let ending = match self.ending {
            Some(Ending::Ipo) => "Reached the IPO.".to_string(),
            Some(Ending::Acquired) => "Acquired for parts.".to_string(),
            Some(_) => format!("Died of {} in week {}.", self.last_cause, self.week),
            None => "Still on the trail.".to_string(),
        };
        s.push_str(&format!(
            "**Outcome:** {ending} **Score:** {}\n\n",
            self.score()
        ));
        s.push_str("## Decisions\n\n");
        for j in &self.journal {
            s.push_str(&format!("### Week {}: {}\n\n", j.week, j.title));
            s.push_str(&format!("You chose: {}. {}\n\n", j.choice, j.outcome));
            if let Some(alt) = &j.alternative {
                s.push_str(&format!("The field notes recommend: {alt}.\n\n"));
            }
            if let Some(n) = &j.note {
                s.push_str(&format!("> {n}"));
                if let Some(src) = &j.source {
                    s.push_str(&format!(" (Source: {src})"));
                }
                s.push_str("\n\n");
            }
        }
        if !self.arcs.is_empty() {
            s.push_str("## Incidents\n\n");
            for a in &self.arcs {
                let def = &self.content.arcs[a.def];
                let status = match a.status {
                    ArcStatus::Dormant => "never started",
                    ArcStatus::Active => "still under way",
                    ArcStatus::Stopped => "stopped",
                    ArcStatus::Succeeded => "reached its objective",
                };
                s.push_str(&format!("### {} ({status})\n\n", def.name));
                for t in &a.timeline {
                    let seen = if t.seen { "seen" } else { "unseen" };
                    s.push_str(&format!(
                        "- Week {}: {} ({seen})\n",
                        t.week, def.beat[t.beat].label
                    ));
                }
                s.push('\n');
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start(seed: u64, company: &str, level: &str) -> Game {
        let mut g = Game::new(seed, vec![]);
        g.input(company);
        g.input(level);
        for _ in 0..6 {
            g.input("");
        }
        g
    }

    #[test]
    fn setup_reaches_the_expo() {
        let g = start(1, "2", "1");
        assert_eq!(g.phase, Ctx::Expo);
        assert_eq!(g.budget, 80_000);
        assert_eq!(g.total_weeks(), 12);
    }

    #[test]
    fn pro_trail_is_full_length() {
        let g = start(1, "2", "4");
        assert_eq!(g.total_weeks(), 26);
        assert_eq!(g.arcs.len(), 4);
    }

    #[test]
    fn bad_input_keeps_phase() {
        let mut g = start(1, "2", "2");
        g.input("99");
        assert_eq!(g.phase, Ctx::Expo);
        g.input("banana");
        assert_eq!(g.phase, Ctx::Expo);
    }

    #[test]
    fn names_are_used() {
        let mut g = Game::new(3, vec![]);
        g.input("3");
        g.input("1");
        g.input("Garage Labs");
        g.input("Zoe");
        for _ in 0..4 {
            g.input("");
        }
        assert_eq!(g.company_name, "Garage Labs");
        assert_eq!(g.team[0].name, "Zoe");
    }
}
