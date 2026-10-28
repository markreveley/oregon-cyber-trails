//! Game content: events, controls and incident arcs, read from the TOML
//! files under `content/` (embedded in the binary at build time).

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

const EVENT_FILES: &[(&str, &str)] = &[
    ("launch.toml", include_str!("../content/events/launch.toml")),
    ("signin.toml", include_str!("../content/events/signin.toml")),
    (
        "identifier.toml",
        include_str!("../content/events/identifier.toml"),
    ),
    ("keys.toml", include_str!("../content/events/keys.toml")),
    ("agents.toml", include_str!("../content/events/agents.toml")),
    ("gates.toml", include_str!("../content/events/gates.toml")),
    ("arcs.toml", include_str!("../content/events/arcs.toml")),
];
const CONTROLS: &str = include_str!("../content/controls.toml");
const ARCS: &str = include_str!("../content/arcs.toml");

/// Stage ids, in trail order.
pub const STAGE_IDS: [&str; 5] = ["launch", "signin", "identifier", "keys", "agents"];

#[derive(Debug, Deserialize)]
struct EventFile {
    #[serde(default)]
    event: Vec<EventDef>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct EventDef {
    pub id: String,
    pub title: String,
    /// "random" (drawn each week), "gate" (river crossing at a stage's end),
    /// "beat" (fired by an incident arc) or "followup" (queued by a chance).
    #[serde(default = "default_kind")]
    pub kind: String,
    /// Stage id, or "any". Gates name the stage they end.
    #[serde(default = "default_stage")]
    pub stage: String,
    #[serde(default = "one")]
    pub weight: u32,
    #[serde(default)]
    pub min_level: u8,
    #[serde(default = "three")]
    pub max_level: u8,
    /// Repeatable events come back after a cooldown; others fire once a run.
    #[serde(default)]
    pub repeat: bool,
    #[serde(default)]
    pub when: When,
    pub text: String,
    /// Plain-words explanation shown at Beginner and Intermediate.
    #[serde(default)]
    pub gloss: Option<String>,
    /// Who the event happens to: a role ("founder", "engineer", "ops",
    /// "intern") or "random". Filled into `{who}`.
    #[serde(default)]
    pub target: Option<String>,
    /// Knock this many servers offline when the event fires: [min, max].
    #[serde(default)]
    pub knock_servers: Option<[i32; 2]>,
    /// A breach starts the disclosure clock at Pro.
    #[serde(default)]
    pub breach: bool,
    /// A clue: labelled "CLUE" at Beginner.
    #[serde(default)]
    pub clue: bool,
    /// What a tombstone says killed you, e.g. "a reused password".
    #[serde(default)]
    pub cause: Option<String>,
    #[serde(default)]
    pub option: Vec<OptionDef>,
    #[serde(default)]
    pub note: Option<NoteDef>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct When {
    #[serde(default)]
    pub controls_missing: Vec<String>,
    #[serde(default)]
    pub controls_present: Vec<String>,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub not_flags: Vec<String>,
    #[serde(default)]
    pub min_week: Option<u32>,
    /// From this level up, the event is only seen with logging in place.
    #[serde(default)]
    pub needs_logging_from_level: Option<u8>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct OptionDef {
    pub label: String,
    /// Dollars, at funded-startup scale.
    #[serde(default)]
    pub cost: i64,
    /// Team hours this week.
    #[serde(default)]
    pub hours: i32,
    /// Needs a team member in this role who is still around.
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub requires_control: Option<String>,
    #[serde(default)]
    pub requires_missing: Option<String>,
    #[serde(default)]
    pub requires_flag: Option<String>,
    /// A side effect hint shown at Beginner.
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default)]
    pub outcome: String,
    #[serde(default)]
    pub effect: Effect,
    #[serde(default)]
    pub chance: Vec<Chance>,
    /// "textbook" (what the field notes recommend), "secure" (the most
    /// cautious choice), "discloses" (tells customers about a breach).
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Chance {
    pub p: f64,
    #[serde(default)]
    pub outcome: String,
    #[serde(default)]
    pub effect: Effect,
    /// Queue this event instead of (or as well as) the effect.
    #[serde(default)]
    pub event: Option<String>,
    /// Weeks before it happens (0 = now).
    #[serde(default)]
    pub delay: u32,
    /// Skipped when this control is in place.
    #[serde(default)]
    pub unless_control: Option<String>,
    /// Skipped when this flag is set.
    #[serde(default)]
    pub unless_flag: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    #[serde(default)]
    pub budget: i64,
    #[serde(default)]
    pub trust: i32,
    #[serde(default)]
    pub servers: i32,
    #[serde(default)]
    pub restore_servers: bool,
    /// Fatigue added to the event's target (or everyone, below).
    #[serde(default)]
    pub fatigue: i32,
    #[serde(default)]
    pub fatigue_all: i32,
    #[serde(default)]
    pub remove_target: bool,
    #[serde(default)]
    pub compromise_target: bool,
    #[serde(default)]
    pub clear_compromised: bool,
    #[serde(default)]
    pub add_control: Option<String>,
    #[serde(default)]
    pub remove_control: Option<String>,
    #[serde(default)]
    pub set_flag: Vec<String>,
    #[serde(default)]
    pub clear_flag: Vec<String>,
    /// Lose this many weeks of revenue.
    #[serde(default)]
    pub revenue_weeks_lost: u32,
    /// Stop the incident arc this event belongs to.
    #[serde(default)]
    pub knock_back: bool,
    /// A hidden incident has come to light.
    #[serde(default)]
    pub surfaced: bool,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct NoteDef {
    pub text: String,
    /// A public, vendor-neutral source, shown to the player.
    #[serde(default)]
    pub source: Option<String>,
    /// Internal authoring references (claim ids). Never shown.
    #[serde(default)]
    pub topics: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ControlDef {
    pub id: String,
    pub name: String,
    pub cost: i64,
    #[serde(default)]
    pub upkeep: i64,
    pub blurb: String,
    /// Plain-words explanation at Beginner.
    #[serde(default)]
    pub gloss: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ControlFile {
    control: Vec<ControlDef>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ArcDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub min_level: u8,
    /// The week window in which the first beat fires.
    pub start: [u32; 2],
    /// Missing controls (or unset flags) that speed the attacker up.
    #[serde(default)]
    pub weak_to: Vec<String>,
    /// Controls (or flags) that slow the attacker down.
    #[serde(default)]
    pub slowed_by: Vec<String>,
    pub beat: Vec<BeatDef>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct BeatDef {
    /// What happened, for the timeline.
    pub label: String,
    /// Candidate events; the first one whose conditions hold is shown. If
    /// none hold, the beat happens unseen.
    pub events: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ArcFile {
    arc: Vec<ArcDef>,
}

#[derive(Debug)]
pub struct Content {
    pub events: Vec<EventDef>,
    pub by_id: HashMap<String, usize>,
    pub controls: Vec<ControlDef>,
    pub arcs: Vec<ArcDef>,
}

fn default_kind() -> String {
    "random".into()
}
fn default_stage() -> String {
    "any".into()
}
fn one() -> u32 {
    1
}
fn three() -> u8 {
    3
}

impl Content {
    pub fn parse() -> Result<Content, String> {
        let mut events = Vec::new();
        for (name, src) in EVENT_FILES {
            let f: EventFile = toml::from_str(src).map_err(|e| format!("{name}: {e}"))?;
            events.extend(f.event);
        }
        let controls: ControlFile =
            toml::from_str(CONTROLS).map_err(|e| format!("controls.toml: {e}"))?;
        let arcs: ArcFile = toml::from_str(ARCS).map_err(|e| format!("arcs.toml: {e}"))?;
        let mut by_id = HashMap::new();
        for (i, e) in events.iter().enumerate() {
            if by_id.insert(e.id.clone(), i).is_some() {
                return Err(format!("duplicate event id {}", e.id));
            }
        }
        Ok(Content {
            events,
            by_id,
            controls: controls.control,
            arcs: arcs.arc,
        })
    }

    pub fn get() -> &'static Content {
        static C: OnceLock<Content> = OnceLock::new();
        C.get_or_init(|| Content::parse().expect("content is valid"))
    }

    pub fn event(&self, id: &str) -> &EventDef {
        &self.events[self.by_id[id]]
    }

    pub fn control(&self, id: &str) -> Option<&ControlDef> {
        self.controls.iter().find(|c| c.id == id)
    }

    /// Check every cross-reference and rule the engine relies on. Returns
    /// the list of problems (empty when the content is sound).
    pub fn check(&self) -> Vec<String> {
        let mut errs = Vec::new();
        let roles = ["founder", "engineer", "ops", "intern", "random"];
        let kinds = ["random", "gate", "beat", "followup"];
        let is_control = |id: &str| self.controls.iter().any(|c| c.id == id);
        for e in &self.events {
            let at = &e.id;
            if !kinds.contains(&e.kind.as_str()) {
                errs.push(format!("{at}: unknown kind {}", e.kind));
            }
            if e.stage != "any" && !STAGE_IDS.contains(&e.stage.as_str()) {
                errs.push(format!("{at}: unknown stage {}", e.stage));
            }
            if e.kind == "gate" && e.stage == "any" {
                errs.push(format!("{at}: a gate must name its stage"));
            }
            if let Some(t) = &e.target
                && !roles.contains(&t.as_str())
            {
                errs.push(format!("{at}: unknown target {t}"));
            }
            if e.text.contains("{who}") && e.target.is_none() {
                errs.push(format!("{at}: uses {{who}} with no target"));
            }
            if e.option.is_empty() {
                errs.push(format!("{at}: no options"));
            }
            if e.note.is_none() && e.kind != "followup" {
                errs.push(format!("{at}: no field note"));
            }
            if !e.option.iter().any(|o| {
                o.cost == 0
                    && o.hours == 0
                    && o.role.is_none()
                    && o.requires_control.is_none()
                    && o.requires_flag.is_none()
            }) {
                errs.push(format!("{at}: needs one option anyone can always take"));
            }
            for c in e
                .when
                .controls_missing
                .iter()
                .chain(&e.when.controls_present)
            {
                if !is_control(c) {
                    errs.push(format!("{at}: unknown control {c}"));
                }
            }
            for o in &e.option {
                let mut effects = vec![&o.effect];
                for c in &o.chance {
                    effects.push(&c.effect);
                    if !(0.0..=1.0).contains(&c.p) {
                        errs.push(format!("{at}: chance p out of range"));
                    }
                    if let Some(ev) = &c.event {
                        match self.by_id.get(ev) {
                            None => errs.push(format!("{at}: unknown event {ev}")),
                            Some(&i) if self.events[i].kind != "followup" => {
                                errs.push(format!("{at}: {ev} should be kind followup"))
                            }
                            _ => {}
                        }
                    }
                    if let Some(u) = &c.unless_control
                        && !is_control(u)
                    {
                        errs.push(format!("{at}: unknown control {u}"));
                    }
                }
                for c in [&o.requires_control, &o.requires_missing]
                    .into_iter()
                    .flatten()
                {
                    if !is_control(c) {
                        errs.push(format!("{at}: unknown control {c}"));
                    }
                }
                if let Some(r) = &o.role
                    && !roles.contains(&r.as_str())
                {
                    errs.push(format!("{at}: unknown role {r}"));
                }
                for ef in effects {
                    for c in [&ef.add_control, &ef.remove_control].into_iter().flatten() {
                        if !is_control(c) {
                            errs.push(format!("{at}: unknown control {c}"));
                        }
                    }
                    if (ef.remove_target || ef.compromise_target || ef.fatigue != 0)
                        && e.target.is_none()
                    {
                        errs.push(format!("{at}: effect on target with no target"));
                    }
                    if ef.knock_back && e.kind != "beat" {
                        errs.push(format!("{at}: knock_back outside an arc beat"));
                    }
                }
            }
        }
        for stage in STAGE_IDS {
            if !self
                .events
                .iter()
                .any(|e| e.kind == "gate" && e.stage == stage)
            {
                errs.push(format!("stage {stage} has no gate"));
            }
        }
        for a in &self.arcs {
            if a.beat.len() < 3 {
                errs.push(format!("arc {}: needs at least three beats", a.id));
            }
            for b in &a.beat {
                for ev in &b.events {
                    match self.by_id.get(ev) {
                        None => errs.push(format!("arc {}: unknown event {ev}", a.id)),
                        Some(&i) if self.events[i].kind != "beat" => {
                            errs.push(format!("arc {}: {ev} should be kind beat", a.id))
                        }
                        _ => {}
                    }
                }
            }
            if let Some(last) = a.beat.last()
                && let Some(ev) = last.events.first()
                && let Some(&i) = self.by_id.get(ev)
                && self.events[i].when.needs_logging_from_level.is_some()
            {
                errs.push(format!("arc {}: the objective must always be seen", a.id));
            }
        }
        for e in &self.events {
            if e.kind == "beat"
                && !self
                    .arcs
                    .iter()
                    .any(|a| a.beat.iter().any(|b| b.events.contains(&e.id)))
            {
                errs.push(format!("{}: beat not used by any arc", e.id));
            }
        }
        errs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_parses_and_checks_clean() {
        let c = Content::parse().unwrap();
        let errs = c.check();
        assert!(errs.is_empty(), "content problems:\n{}", errs.join("\n"));
    }
}
