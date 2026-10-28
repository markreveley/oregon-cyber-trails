# Oregon Cyber Trails

A terminal game in the style of Oregon Trail that teaches identity and
security through judgement calls during attacks. "You have died of
dysentery" becomes "You are undergoing a DDoS attack. 3 of 5 servers are
down."

You are the first security hire at a young company. The trail runs for 26
weeks, from launch to the IPO, across five stages: Launch, Sign-in, The
identifier, Holding keys for others, and Agents. Each week you set the pace,
spend a budget and a few hours of the team's time, and face whatever the
trail sends: credential stuffing, phished engineers, leaked signing keys,
lapsed domains, AI agents with too much authority. Most choices have costs on
both sides, and some of the consequences land weeks later.

The design is in [`docs/spec.md`](docs/spec.md).

## Play

You need Rust (1.88 or newer).

    cargo run --release

Replay a run exactly with its seed:

    cargo run --release -- --seed 4817

Type a number and press Enter at each prompt. A run takes 10 to 25 minutes.
When you die, you write the epitaph for your company's tombstone, and later
runs on the same machine pass your grave (`~/.oregon-cyber-trails/`). At the
end you can save a trail journal: every decision you made, what happened, and
what the field notes recommend.

## How it plays

- **Company type** sets your money and score multiplier: a bank spin-off
  (x1), a funded startup (x2) or two founders in a garage (x3).
- **Level** sets how much the game explains and how cunning the attacker is.

  | | Beginner | Intermediate | Advanced | Pro |
  |---|---|---|---|---|
  | Trail | 12 weeks, three stages | 26 weeks, five stages | 26 weeks | 26 weeks |
  | Costs shown | Exact, with hints | Exact | Ranges | Hidden |
  | Clues | Labelled | Unlabelled | Some need logging | Some need logging |
  | Incident arcs | 1 | 2 | 3 | 4 |
  | Field notes | Every event | Every event | Only after a non-textbook choice | Only in the journal |
  | Mentor (rewind a decision) | Once per stage | Once per run | None | None |
  | Timeline | Shown | Shown | You put the attacker's steps in order | Same, plus a 72-hour disclosure clock |
  | Score | x1 | x2 | x3 | x5 |

- **Each week** you can continue, check supplies, look at the map, change the
  shipping pace (careful, steady, crunch) or on-call rotation, hunt for
  threats in a log, or rest a tired team.
- **Money and hours.** Controls bought at the Vendor Expo (and at the vendor
  stop after each milestone) have a price and a weekly upkeep. Most fixes cost
  security hours, and hours spent on security are hours not spent shipping,
  so revenue dips.
- **The hidden attacker.** Incident arcs (The long con, The founder's inbox,
  The quiet key, The helpful agent) advance in secret. You see clues, and
  acting on one can stop the arc. Missing controls, tired staff, crunch and a
  skeleton on-call rotation speed the attacker up. When an arc ends you get an
  incident review of what happened and when you could have stopped it.
- **River crossings.** Each stage ends with a big decision in the
  ford / caulk / ferry / wait pattern: the first customer's security
  questionnaire, build or buy sign-in, the SOC 2 audit, the enterprise
  contract, the IPO roadshow.
- **Endings.** Reach the IPO, or die: bankrupt, customers lost (trust below
  20), offline for two weeks, or everyone quit. High trust with no money gets
  you acquired for parts.

## Balance

The `sim` command plays thousands of runs with four scripted players:
*random*, *miser* (cheapest option, buys nothing), *paranoid* (most cautious
option, buys everything) and *textbook* (the option the field notes
recommend). The textbook bot knows the answer key, so its rate is an upper
bound for a careful player.

    cargo run --release -- sim --runs 400 --level all --company all

Win rates for the funded startup, 400 runs each:

| Level | random | miser | paranoid | textbook |
|---|---|---|---|---|
| Beginner | 93% | 34% | 100% | 100% |
| Intermediate | 10% | 0% | 30% | 100% |
| Advanced | 0% | 0% | 8% | 91% |
| Pro | 0% | 0% | under 1% | 51% |

Paranoid players go broke and misers lose their customers. `tests/play.rs`
keeps these within bands, so a content change that breaks the balance fails
CI. The garage is much harder: textbook wins 67% at Advanced and 17% at
Pro, mostly by running out of money.

## Content

Everything the player reads is data under `content/`, embedded in the binary
at build time:

- `content/events/*.toml`: events, one file per stage, plus river crossings
  (`gates.toml`) and the beats of the incident arcs (`arcs.toml`).
- `content/arcs.toml`: the incident arcs and their beats.
- `content/controls.toml`: what you can buy at the Vendor Expo.

An event looks like this:

```toml
[[event]]
id = "signin.no-pkce"
title = "Mobile app without PKCE"
stage = "signin"
cause = "a mobile app without PKCE"          # what the tombstone says
text = "Your new mobile app shipped its sign-in without PKCE."
gloss = "PKCE ties a sign-in code to the app that asked for it."   # Beginner and Intermediate

[[event.option]]
label = "Hotfix it now"
hours = 20
tags = ["textbook", "secure"]
outcome = "The fix ships on Thursday."

[[event.option]]
label = "Fix it in the next release"
hours = 4
outcome = "Next release is in a month."
chance = [{ p = 0.35, delay = 2, outcome = "A fake app caught users' sign-in codes.", effect = { trust = -8 } }]

[event.note]
text = "Apps on a phone cannot keep a secret, so they prove the code is theirs with PKCE instead."
source = "RFC 7636, Proof Key for Code Exchange"
topics = ["claim:pkce-protects-code-exchange"]   # internal authoring reference, never shown
```

The fields are documented in `src/content.rs`. Costs are dollars at
funded-startup scale; the engine scales them by company and level. Run
`cargo run -- check` after editing: it checks every reference, and that every
event has an option anyone can always take, so a broke player is never stuck.

## Code

| Path | What |
|---|---|
| `src/game.rs` | The engine: a pure state machine (`take_output`, `prompt`, `input`), with no I/O |
| `src/content.rs` | Content types, loading and `check` |
| `src/bots.rs` | The scripted players and `simulate` |
| `src/logs.rs` | The threat-hunting log generator |
| `src/rng.rs` | A small seeded RNG, so a seed replays a run anywhere |
| `src/main.rs` | The terminal front end, `sim` and `check` |
| `examples/trace.rs` | Watch a bot play: `cargo run --release --example trace -- textbook pro 21 --full` |

## Status

Built, from the spec: the five stages, the four levels, 65 events, four
incident arcs, river crossings, the Vendor Expo, pace and on-call settings,
threat hunting, the mentor, incident reviews and timeline reconstruction, the
Pro disclosure clock, tombstones, the trail journal, scoring, the four bots
and the content check.

Not built yet: the browser front end (the library already compiles for
`wasm32-unknown-unknown`; what is missing is the page with a terminal
emulator that drives it), the SSH host, classroom and hackathon modes (shared seeds work already;
leaderboards and shared tombstones do not), and the daily trail.

Differences from the spec: events live in one TOML file per stage rather
than one file per event, and conditions and effects are typed TOML fields
rather than a small expression language. The textbook bot's win rates are
higher than the spec's targets below Pro, because it knows the answer key.
