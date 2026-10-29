# Oregon Cyber Trails: a spec for an Oregon Trail-style security game

**Date:** 2026-10-28
**Status:** Draft spec and plan, not an implementation. Mark answered the first six open questions on 2026-10-28 (name: Oregon Cyber Trails; levels from beginner to pro; 15 to 25 minute runs; no direct IdentiKey connection, but the trail covers the ground IdentiKey works in; Rust; dry humour). The rest are in the last section.
**Inspiration:** the *Identity under Attack* Blue Team challenge (Amrita Vishwa Vidyapeetham, Chennai, on Unstop, 2026): a four-hour online investigation in which teams of one to three trace a compromised identity as it spreads across accounts, systems and cloud, reconstruct the timeline, and recommend containment. Open to students, no Blue Team experience required.
**Material (for authors, not shown to players):** in `markreveley/identikey-dev`, the tutorial "IdentiKey v2" (`tutorials/identikey-from-the-ground-up.md`), the agent authority tutorial (`tutorials/identikey-agent-authority.md`), and the tutorial claims in the DAG (`dag/claims/tutorial/`).

## Contents

- [1. The pitch](#1-the-pitch)
- [2. What the hackathon gives us, and what a game adds](#2-what-the-hackathon-gives-us-and-what-a-game-adds)
- [3. Design goals and non-goals](#3-design-goals-and-non-goals)
- [4. The Oregon Trail mapping](#4-the-oregon-trail-mapping)
- [5. The core loop](#5-the-core-loop)
- [6. State: resources, team and the hidden attacker](#6-state-resources-team-and-the-hidden-attacker)
- [7. The trail: five stages](#7-the-trail-five-stages)
- [8. Levels: beginner to pro](#8-levels-beginner-to-pro)
- [9. Events and judgement calls](#9-events-and-judgement-calls)
- [10. Incident arcs: the investigation layer](#10-incident-arcs-the-investigation-layer)
- [11. Teaching: field notes, the trail journal and sources](#11-teaching-field-notes-the-trail-journal-and-sources)
- [12. Scoring, endings and tombstones](#12-scoring-endings-and-tombstones)
- [13. A sample session](#13-a-sample-session)
- [14. Content format](#14-content-format)
- [15. Architecture and tech stack](#15-architecture-and-tech-stack)
- [16. Balancing and testing](#16-balancing-and-testing)
- [17. Modes: solo, classroom and hackathon](#17-modes-solo-classroom-and-hackathon)
- [18. Plan and milestones](#18-plan-and-milestones)
- [19. Risks](#19-risks)
- [20. Open questions](#20-open-questions)

## 1. The pitch

You are the first security hire at a startup with five people, a few servers and a deadline. The trail runs from launch to a public milestone (the first enterprise contract, then the audit, then the IPO). Each week you set the pace, spend a budget, and face whatever the trail throws at you: a DDoS, a phished engineer, a leaked token, a vendor breach, an agent with too much authority. Every event is a judgement call with costs on both sides, and the consequences often arrive weeks later.

"You have died of dysentery" becomes:

```
WEEK 7. You are undergoing a DDoS attack. 3 of 5 servers are down.

  1. Absorb it: scale up and wait (costs $8,000)
  2. Turn on the CDN's attack mode (costs $2,000, breaks the mobile app's login)
  3. Block the top 50 source networks by hand
  4. Ask the team to stay up and watch it

What is your choice?
```

And later, the tombstone:

```
        _______________
       /               \
      |    HERE LIES    |
      |   ACME, INC.    |
      |                 |
      |  Died of a      |
      |  reused API key |
      |   Week 9, Q3    |
      |_________________|
```

The joke carries the lesson: the cause of death is always a specific, nameable security failure, and the trail journal at the end shows the decision that led to it.

## 2. What the hackathon gives us, and what a game adds

The *Identity under Attack* format has three ideas worth keeping:

- **Identity is the attack path.** One compromised identity spreads across accounts, systems and cloud. The game's incident arcs ([section 10](#10-incident-arcs-the-investigation-layer)) follow the same shape.
- **Evidence over trivia.** Teams correlate traces and reconstruct a timeline. The game's threat-hunting minigame and clue system ask for the same skill at a smaller scale.
- **Low barrier.** No prior Blue Team experience is needed. The game assumes none either.

What a game adds is time and trade-offs. A hackathon investigates one breach after it happens. On the trail the player makes many small decisions before anything happens, with a budget that runs out, and sees which of those decisions mattered. That is the judgement a defender actually exercises: not "what is the right answer" but "what can I afford to defend, and what do I accept".

## 3. Design goals and non-goals

**Goals**

1. **Judgement, not quizzes.** Most events have no single right answer. Each option trades budget, uptime, trust or team time. A few have a clearly wrong answer, and those teach the principles that have no exceptions (do not paste a secret into a public repo).
2. **Delayed consequences.** A choice in week 2 can kill you in week 9. This is the main thing a game teaches that a lecture cannot.
3. **Fair.** Every bad outcome is foreshadowed by a clue the player could have noticed. No gotchas.
4. **Beginner to pro.** Four levels ([section 8](#8-levels-beginner-to-pro)) take a player from "what is phishing" to reconstructing a multi-stage intrusion under a disclosure deadline.
5. **Short and replayable.** A run takes 15 to 25 minutes. Events are drawn at random from a weighted pool, so each run differs.
6. **Every lesson is sourced.** Each field note cites public material (standards, OWASP, NIST, vendor-neutral guides).
7. **Content is data.** Events are written as data files, so anyone can add a scenario without touching the engine.
8. **Text first.** It runs in a terminal, line by line, like the 1985 original, and the same engine runs in a browser.

**Non-goals**

- Not a realistic simulation. Numbers are tuned for fun and for the lesson, not for accuracy.
- Not a hacking game. The player defends; attacker tooling is never taught.
- No IdentiKey connection. The game never names IdentiKey, its products or its terms (XID, custody ladder, C1 to C4). It travels the regions IdentiKey works in (sign-in, tokens, keys and rotation, holding keys for others, delegated authority for agents) in vendor-neutral terms. Internally, authors use the IdentiKey tutorial and its DAG claims as a checklist of what each region should teach ([section 11](#11-teaching-field-notes-the-trail-journal-and-sources)).
- No graphics beyond ASCII.

## 4. The Oregon Trail mapping

| Oregon Trail | Oregon Cyber Trails | What it teaches |
|---|---|---|
| Profession (banker, carpenter, farmer) sets money and score multiplier | **Company type**: bank spin-off ($200k, score ×1), funded startup ($80k, ×2), two founders in a garage ($20k, ×3) | Security on a budget is the real game |
| Party of five who can die | **The team**: five named people who can be phished, burn out, quit or be fired | People are the attack surface |
| Matt's General Store | **The Vendor Expo** before launch, and a vendor stop at each landmark | Controls cost money and you cannot buy them all |
| Oxen, food, clothing, ammunition, spare parts | **Servers, budget, team hours, controls, backups** ([section 6](#6-state-resources-team-and-the-hidden-attacker)) | Each resource fails differently |
| Pace (steady, strenuous, grueling) | **Shipping pace** (careful, steady, crunch) | Speed widens the attack surface and tires the team |
| Rations | **On-call rotation** (generous, normal, skeleton) | Fewer people watching means slower detection |
| Random events (snakebite, broken axle, dysentery) | **Security events**: DDoS, phishing, credential stuffing, leaked tokens, expired certificates | The threat catalogue |
| River crossings (ford, caulk, ferry, wait) | **Big decisions** at each stage gate: build or buy SSO, migrate auth, rotate keys | Architecture choices are crossings: costly to redo |
| Landmarks and forts | **Milestones**: first customer, Series A, SOC 2 audit, enterprise deal, IPO | Requirements change as you grow |
| Hunting | **Threat hunting**: spot the bad line in a scrolling log | Detection is a skill |
| Tombstones left by earlier players | **Tombstones** with epitaphs, shown to later players on the same machine or leaderboard | Learn from others' failures |
| Score at Oregon | **Score at IPO**: survivors, trust, budget, multiplier | Survive with trust intact |

## 5. The core loop

Each turn is one week. A run is about 26 weeks (two quarters), and a turn takes 30 to 60 seconds.

1. **Status screen.** Week, stage, budget, servers up, trust, team (with status: fine, tired, compromised, gone), and controls in place.
2. **Player options** (the Oregon Trail menu):
   1. Continue on the trail
   2. Check supplies (resources and controls)
   3. Look at the map (stages and milestones)
   4. Change shipping pace
   5. Change on-call rotation
   6. Hunt for threats (minigame, costs team hours)
   7. Buy or retire a control (only at a vendor stop)
   8. Rest (skip a week of shipping to recover the team)
3. **Advance.** The world ticks: the budget burns, the product ships (raising revenue and attack surface), the hidden attacker moves ([section 6](#6-state-resources-team-and-the-hidden-attacker)).
4. **Event draw.** Zero, one or (rarely) two events, weighted by stage, pace, controls and attacker state.
5. **Choice and outcome.** The player picks an option; immediate effects apply; delayed effects are queued.
6. **Field note.** A two or three line debrief after significant events ([section 11](#11-teaching-field-notes-the-trail-journal-and-sources)).
7. **Check for endings.** Out of money, out of trust, or the whole team gone ends the run.

## 6. State: resources, team and the hidden attacker

### Visible resources

| Resource | Starts at | Goes down when | Runs out means |
|---|---|---|---|
| **Budget** ($) | Set by company type | Weekly burn, controls, incident costs, fines | Bankrupt: game over |
| **Servers** (n of 5) | 5 | DDoS, ransomware, outages, bad deploys | Below 2: no revenue that week; 0 for two weeks is game over |
| **Trust** (0 to 100) | 60 | Breaches, outages, disclosed incidents, slow response | Below 20: customers leave, game over |
| **Team hours** (per week) | 5 people × 40 | Crunch, incidents, threat hunting | Tired people get phished more and make mistakes |
| **Controls** | None, or what you bought | Retired, misconfigured, or bypassed | Not a failure on its own; raises event odds |
| **Backups** (0 to 3 copies, age) | 0 | Never tested, ransomware hits connected copies | Ransomware with no clean backup is very expensive |

Trust is the score's heart. Hiding an incident saves trust now and costs far more if it surfaces later (a delayed effect with a high chance). Disclosing promptly costs a little and caps the damage.

### The team

Five people with names (the player can rename them at the start, as in the original): a founder, two engineers, an ops person and an intern. Each has a role that matters to events (only the ops person can rotate production keys quickly; the founder has the finance access a CEO-fraud email wants). Status:

- **Fine.**
- **Tired** (after crunch or incidents): higher chance to fall for phishing, slower to respond.
- **Compromised** (phished, SIM-swapped, laptop stolen): the attacker holds that person's access until you notice and revoke it. The player is not told directly; clues appear.
- **Gone** (quit, burned out, fired for cause): their access must be removed, or it becomes a leftover account the attacker can use.

The classic death line is reused: "Priya has been phished." "Sam has burned out." "The intern has died of a reused password."

### Controls

Bought at the Vendor Expo and vendor stops. Each has a cost, an upkeep (hours or dollars per week) and the events it changes. A first catalogue:

| Control | Cost | Changes |
|---|---|---|
| Password manager for the team | Low | Password reuse, credential stuffing |
| One-time-code MFA | Low | Stuffing, most phishing (not real-time phishing proxies, not SIM swap for SMS) |
| Passkeys / security keys | Medium | Phishing, including real-time proxies; SIM swap |
| CDN / DDoS protection | Medium, weekly | DDoS |
| Single sign-on with a central identity provider | Medium | Offboarding, leftover accounts; one place to revoke |
| Centralised logging | Medium, weekly | Detection speed, clue quality |
| Secret scanning in CI | Low | Leaked keys in repos |
| Offline, tested backups | Medium, hours to test | Ransomware |
| Hardware key storage (HSM or enclave) | High | Signing-key theft |
| Bug bounty | Weekly | Early warning of some events, occasional noisy reports |
| Short-lived tokens, rotation runbook | Hours | Token theft, key compromise recovery |

No control is free and none is perfect. Passkeys stop phishing but bring a recovery event ("Dana lost her only security key on a plane"); synced passkeys move the risk to the password-manager account. This is where the game teaches trade-offs rather than a checklist.

### The hidden attacker

The game keeps a hidden **attacker state**: a small kill chain per active campaign, with stages *recon → initial access → foothold → privilege → objective*. Each week each campaign may advance, with odds raised by missing controls, tired staff and fast pace, and lowered by controls and detection. The player never sees this state directly. They see **clues**: an odd login time in the logs, a support ticket about a password reset nobody asked for, an invoice change request. Acting on clues (investigate, revoke, rotate) knocks a campaign back. Ignoring them lets it reach its objective, which is the big event (data theft, ransomware, wire fraud).

This is what makes the game more than a choose-your-own-adventure: the same week-9 ransomware can be a surprise or something the player saw coming since week 4.

## 7. The trail: five stages

The trail crosses five regions, the same ground IdentiKey's tutorial covers, in the same order, but in vendor-neutral terms. The "Region it covers" column is an authoring note and never appears in the game.

| Stage (weeks) | Milestone at the end | Theme | Region it covers (tutorial part, internal) | Typical events |
|---|---|---|---|---|
| 1. **Launch** (1 to 5) | First paying customer | Passwords, MFA, phishing, DDoS | Part 1. The landscape | Credential stuffing, phishing email, DDoS, reused password, lost laptop |
| 2. **Sign-in** (6 to 10) | Series A | Building login: OAuth, OIDC, tokens | Part 2. Signing in to apps | Open redirect URI, missing PKCE, token in a URL, consent phishing, device-code phishing, shared user IDs let partners correlate users |
| 3. **The identifier** (11 to 15) | SOC 2 audit (a river crossing) | Keys, rotation, resolution, freshness | Part 3. The identifier underneath | Signing key leaked, no rotation plan, stale cached key document, replayed token, expired certificate, domain about to lapse |
| 4. **Holding keys for others** (16 to 20) | Enterprise contract | Custody and consent: you hold keys for users | Part 4. The hosted service | Insider at your own company, a server breach (what does the attacker get?), recovery social-engineering, consent-log tampering |
| 5. **Agents** (21 to 26) | IPO | Delegated authority for AI agents | Part 5 and the agent authority tutorial | An agent holding an upstream API key, a reused approval, a revoked grant still accepted offline, an agent sub-delegating more than it has, a prompt-injected agent |

Each stage gate is a **river crossing**: one big decision with four options in the ford / caulk / ferry / wait pattern. Example for the SOC 2 gate:

```
You have reached the SOC 2 audit. Your auditor asks how production signing
keys are stored and rotated.

  1. Ford it: show them the key in the deploy config and promise to fix it
  2. Caulk it: write a rotation runbook this week (costs 60 team hours)
  3. Ferry: buy an HSM service (costs $15,000 + $1,000/week)
  4. Wait: ask for a two-week extension (trust -5, audit risk later)
```

## 8. Levels: beginner to pro

Four levels, chosen at the start (with a one-line description of who each is for). All four are open from the first run, and finishing a level suggests the next. The level is separate from the company type: company type sets money and the score multiplier, level sets how much the game explains and how cunning the attacker is. Both multiply the score.

| | Beginner | Intermediate | Advanced | Pro |
|---|---|---|---|---|
| **For** | First time thinking about security | Knows what MFA, OAuth and a token are | Has been on call for an incident | Runs incident response or designs auth |
| **Trail** | Stages 1, 2 and 4 (about 12 weeks, 10 minutes) | All five stages (26 weeks) | All five | All five, plus a regulator's disclosure clock |
| **Event pool** | Basics: passwords, phishing, MFA, DDoS, backups, offboarding | Adds tokens, redirects, PKCE, consent screens, rotation | Adds replay, stale caches, insider risk, key custody, agent delegation | All events, with harder variants (real-time phishing proxies, chained OAuth abuse, sub-agent escalation) |
| **Costs shown** | Exact, with a hint of the side effect | Exact | Ranges ("$5,000 to $12,000") | Hidden until paid |
| **Clues** | Labelled "Clue:" | Present but unlabelled | Mixed with noise | Mostly in logs you must hunt for |
| **Hidden attacker** | One campaign, slow | One or two campaigns | Two or three, faster | Adaptive: targets your weakest control and avoids ones you just bought |
| **Arcs** | One short arc (three beats) | Two arcs | Three arcs, one concurrent | Up to three concurrent, including insider and supply chain |
| **Field notes** | After every event, with plain-words glosses | After significant events | Only when something goes wrong | Only in the end-of-run journal |
| **Mentor** | Once per stage, undo one decision and see the alternative | Once per run | None | None |
| **Timeline** | Shown to you after each arc | Shown after each arc | You order the beats, then see the answer | You reconstruct it from logs and are scored on it, as in *Identity under Attack* |
| **Score multiplier** | ×1 | ×2 | ×3 | ×5 |

Most events exist at every level, with per-level text and numbers: at Beginner the leaked-token event explains what a bearer token is; at Pro it says "an access token is in a support screenshot" and moves on. Some events are level-gated (Pro-only arcs, Beginner-only tutorial beats).

Pro is not Beginner with bigger numbers. Three mechanics appear only at the top levels: the **adaptive attacker**, the **disclosure clock** (a breach starts a 72-hour timer; missing it costs trust and a fine), and **scored timeline reconstruction**, where the player enters the order of the attacker's steps from evidence and earns points for each one placed right.

## 9. Events and judgement calls

### Anatomy of an event

- **Trigger:** stage, weight, and conditions (a control missing, a person tired, a campaign at a certain step, a previous choice).
- **Text:** one to four lines, concrete, with numbers.
- **Options:** two to four, each with a cost and effects. At least one option is "do nothing / accept the risk", because accepting a risk is a legitimate decision.
- **Outcomes:** immediate effects, plus queued delayed effects with probabilities.
- **Field note:** what the event teaches and a public source, plus internal `topics` for authors.

### Kinds of judgement

The event pool is balanced across five kinds of decision, so the game does not collapse into "always buy the control":

1. **Spend or accept.** DDoS: pay to absorb, or accept a few hours down. Right answer depends on budget and trust.
2. **Fast or careful.** Leaked key: rotate now and break integrations for a day, or rotate over a week and stay exposed.
3. **Disclose or contain quietly.** A breach of 200 accounts: tell customers (trust -5 now) or not (trust -30 if found later, 40% chance).
4. **Convenience or safety.** Sales wants "sign in with a magic link" for the enterprise demo; the passkey-only policy costs the deal a week.
5. **Investigate or act.** A clue appears: spend hours to investigate (learn the truth, maybe find nothing) or act blind (revoke the account, annoy a real employee if wrong).

### A first event catalogue

About 60 events are needed for good replay; these are the seed list, by stage. The last column is the internal authoring reference (a claim id in identikey-dev `dag/claims/tutorial/`) that the event's lesson must agree with; it is never shown to players.

| # | Stage | Event | The judgement | Authoring reference |
|---|---|---|---|---|
| 1 | Launch | DDoS: x of 5 servers down | Pay, mitigate with side effects, block by hand, or wait | (generic) |
| 2 | Launch | 10,000 logins from one botnet using leaked passwords | Rate-limit, force resets, add MFA, or watch | (generic) |
| 3 | Launch | Engineer clicks a convincing login page; one-time code entered | Revoke sessions, reset, or hope; passkeys would have stopped it | `passkeys-resist-phishing` |
| 4 | Launch | "Can you buy gift cards for a client?" email from the founder | Verify out of band, or comply | `signature-does-not-prove-truth` |
| 5 | Launch | Intern leaves; their accounts are scattered over twelve tools | Hunt them down by hand, or buy SSO | `idp-centralizes-sign-in` |
| 6 | Launch | Laptop stolen from a car | Remote wipe (if managed), revoke sessions, rotate what was on it | `key-storage-decides-safety` |
| 7 | Sign-in | A partner asks to register a wildcard redirect URL | Allow (faster), or require exact URLs | `exact-redirect-match` |
| 8 | Sign-in | Mobile app ships without PKCE | Hotfix now, or next release | `pkce-protects-code-exchange`, `public-clients-rely-on-pkce` |
| 9 | Sign-in | An access token turns up in a support ticket screenshot | Revoke and shorten lifetimes, or ignore | `bearer-tokens-are-like-cash` |
| 10 | Sign-in | Two partner apps match up users by your shared user ID | Switch to per-app IDs (migration cost) or accept | `shared-sub-enables-correlation`, `pairwise-sub-prevents-correlation` |
| 11 | Sign-in | A user approves "Calendar Helper" asking for full mailbox access | Review consent screens, block unverified apps | `authentication-is-not-authorization` |
| 12 | Sign-in | Smart-TV login code arrives by SMS from a stranger | Teach users, shorten code life, show device details | `device-grant-for-browserless` |
| 13 | Identifier | The token-signing key is found in an old backup | Rotate now (log everyone out) or quietly later | `rotation-is-ordinary` |
| 14 | Identifier | A cache serves an old copy of a user's key list after rotation | Shorten cache times, require freshness checks | `resolver-can-serve-stale-document`, `freshness-is-security` |
| 15 | Identifier | A captured login response is replayed an hour later | Check nonces and expiry, or not | `nonce-prevents-replay`, `audience-and-expiry-protect-freshness` |
| 16 | Identifier | The TLS certificate expires on a Saturday | Automate renewal now or keep the calendar reminder | (generic) |
| 17 | Identifier | The domain's registration lapses in 30 days and the reminder went to an ex-employee | Renew and add monitoring | (generic) |
| 18 | Identifier | Marketing wants to move login to a new domain | Passkeys are bound to the old one | `rp-id-is-permanent` |
| 19 | Hosted | An ops engineer can read users' wrapped keys | Split duties and log, or trust them | `c2-splits-privileges` |
| 20 | Hosted | The server is breached: what can the attacker do with the key store? | Depends on earlier choices: signing only, or full control | `breach-yields-signing-not-control` |
| 21 | Hosted | "I lost my phone, please reset my account" from a convincing caller | Strict recovery path, or help them | `account-and-key-recovery-never-merge` |
| 22 | Hosted | A support script signs on users' behalf without fresh consent | Require a fresh proof each time | `c2-signatures-need-logged-consent`, `managed-challenge-needs-fresh-proof` |
| 23 | Hosted | A user's synced passkey is taken over through their cloud account | Offer device-bound keys to high-value users | `synced-passkeys-trade-off` |
| 24 | Agents | The support agent is given the company's payment API key directly | Give it a narrow grant instead | `agent-must-not-hold-upstream-credential` |
| 25 | Agents | An approval for one refund is used for nine more | Bind approvals to one action | `approvals-must-not-be-reusable` |
| 26 | Agents | A revoked agent keeps working because the service checks signatures offline | Add a live revocation check | `revocation-needs-live-state`, `revocation-fails-closed` |
| 27 | Agents | An agent hands a sub-agent more access than it has | Enforce narrowing only | `attenuation-only-narrows` |
| 28 | Agents | Revoking a parent agent leaves its children running | Cascade revocation | `revocation-cascades` |
| 29 | Agents | A web page tells the browsing agent to email the customer list | Enforce limits where the action runs, not in the prompt | `enforce-at-execution-boundary` |
| 30 | Any | A dependency in your build is taken over | Pin, review, or trust | (generic) |

Content gaps: the tutorial claims do not cover DDoS, supply chain, certificates or domains, so those events rest on external material alone. Whether to add generic security claims to the DAG is open question 8.

## 10. Incident arcs: the investigation layer

To echo the hackathon, some events belong to **arcs**: multi-week campaigns driven by the hidden attacker ([section 6](#6-state-resources-team-and-the-hidden-attacker)). An arc has three to six beats, each a clue or an event, and the player can break it at any beat.

**Example arc: "The long con"** (identity spreads across systems, as in *Identity under Attack*):

| Week | Beat | What the player sees | If they act |
|---|---|---|---|
| 3 | Recon | Field note: "Someone is scraping the team page on your site." | Nothing to do yet; foreshadowing |
| 5 | Initial access | Ops engineer Lee is phished (only if no passkeys) | Notice it in logs via threat hunting, revoke Lee's sessions |
| 6 | Clue | "Lee logged in from two countries an hour apart." | Investigate (8 hours): confirms; revoke and rotate Lee's keys |
| 8 | Foothold | A new deploy key appears in the CI system | Only seen with logging; remove it |
| 10 | Privilege | "Cloud bill up 40%: unknown instances in a region you don't use." | Contain: shut down, rotate cloud credentials |
| 12 | Objective | "Your customer database is for sale on a forum." | Disclosure choice; big trust and budget hit |

After the arc resolves, the **timeline screen** reconstructs it: what happened each week, which clue the player saw, and the cheapest point where it could have been stopped. This is the hackathon's "reconstruct the incident timeline" task, done for the player after the fact and scored.

Planned arcs: The long con (phishing to data theft), The CEO's inbox (business email compromise to wire fraud), The friendly partner (OAuth app to data scraping), The quiet key (leaked signing key to forged tokens), The helpful agent (prompt injection to unauthorised refunds).

## 11. Teaching: field notes, the trail journal and sources

Learning happens in three places, each short.

1. **Field notes** after an event, two or three lines, in the voice of a trail diary:
   > Field note. A one-time code can be typed into a fake page just as easily as a real one. A passkey checks which site is asking, so it cannot be phished this way. (Source: FIDO Alliance.)
2. **The trail journal** at the end of a run: every decision, its outcome, and for the ones that hurt, the alternative and the principle. This is the main teaching artefact, and it can be saved as a Markdown file.
3. **Sources.** Each field note ends with one public, vendor-neutral source (an RFC, an OWASP cheat sheet, NIST guidance, the FIDO Alliance). In the web build it is a link; in the terminal, a short title. A player who wants depth follows it.

Rules for writing field notes: one idea each; no jargon without a plain-words gloss; say why, not just what; never blame the player.

Behind the scenes, each event can carry `topics`: claim ids from the IdentiKey tutorial DAG that its lesson must agree with. Players never see them. They give authors a checklist of what each region should teach and a way to review accuracy, and a CI step checks the ids still exist when claims are renamed. Levels change how much teaching the player sees ([section 8](#8-levels-beginner-to-pro)): field notes after every event at Beginner, only in the end-of-run journal at Pro.

## 12. Scoring, endings and tombstones

### Endings

- **Reached the IPO.** The good ending. Score shown.
- **Bankrupt.** Ran out of money.
- **Lost the customers.** Trust below 20.
- **Offline.** No servers for two weeks.
- **Everyone left.** The whole team is gone.
- **Acquired for parts.** A special ending when trust is high but budget is gone: you survive as someone else's product.

### Score

`score = (trust × 10 + budget / 100 + survivors × 200 + milestones × 300) × company multiplier`

Bonuses for each incident arc stopped before its objective, scaled by how early. A small penalty for each hidden incident that surfaced. The multiplier rewards the hard start, as the original did.

### Tombstones

On death, the player writes a one-line epitaph. The tombstone records the cause (generated from the event that ended the run), the week and the epitaph. Later runs pass earlier tombstones on the trail ("You pass the grave of GARAGE LABS. Died of an unrotated signing key. 'We were going to fix it next sprint.'"). Locally they live in a file; in hackathon mode they are shared on a leaderboard.

## 13. A sample session

```
OREGON CYBER TRAILS

Many kinds of companies set out on the trail.
You may:
  1. Be a bank spin-off ($200,000)
  2. Be a funded startup ($80,000)
  3. Be two founders in a garage ($20,000)
  4. Find out the differences between these choices

What is your choice? 3

Choose your level:
  1. Beginner      (first time thinking about security)
  2. Intermediate  (you know what MFA and OAuth are)
  3. Advanced      (you have been on call for an incident)
  4. Pro           (you run incident response)

What is your choice? 1

What are the names of your team?
  Founder: Ana
  Engineer: Ben
  Engineer: Priya
  Ops: Lee
  Intern: Sam

Before you launch, you visit the Vendor Expo.
  1. Password manager ......... $500       You have $20,000
  2. One-time-code MFA ........ $300
  3. Security keys (5) ........ $250
  4. DDoS protection .......... $400/week
  5. Logging .................. $200/week
  6. Done shopping

Which item? 3
...

WEEK 4   Stage: Launch   Budget: $14,250   Servers: 5/5   Trust: 62
Team: Ana (fine) Ben (tired) Priya (fine) Lee (fine) Sam (fine)

Ben got an email: "Your GitHub session expired, sign in again."
He clicked the link. The page asked for his security key.
The key refused: the page was not github.com.

Field note. A security key checks which site is asking before it signs.
The fake page could copy GitHub's look but not its name.
(Source: FIDO Alliance, "How FIDO works")

  Press ENTER to continue.

WEEK 7   Stage: Launch   Budget: $9,800   Servers: 2/5   Trust: 58

You are undergoing a DDoS attack. 3 of 5 servers are down.
  1. Absorb it: scale up and wait ($8,000)
  2. Turn on the CDN's attack mode ($2,000; the mobile app's login breaks)
  3. Block the top 50 source networks by hand (Lee: 20 hours)
  4. Wait it out

What is your choice? 2

The attack fades. 140 mobile users could not sign in. Trust -3.
```

## 14. Content format

Events are TOML files, one per event, in `content/events/<stage>/`. TOML is readable, takes comments, and is easy for non-programmers to edit.

```toml
id = "launch.ddos"
stage = "launch"
levels = ["beginner", "intermediate", "advanced", "pro"]
weight = 3
kind = "spend-or-accept"
when = { servers_min = 3, controls_missing = ["ddos-protection"] }

text = """
You are undergoing a DDoS attack. {down} of {servers} servers are down.
"""
vars = { down = "rand(2, servers - 1)" }

[[option]]
label = "Absorb it: scale up and wait"
cost = { budget = 8000 }
effect = { servers = "restore" }

[[option]]
label = "Turn on the CDN's attack mode"
cost = { budget = 2000 }
effect = { servers = "restore", trust = -3 }
note = "The mobile app's login breaks for a day."

[[option]]
label = "Block the top 50 source networks by hand"
needs = { role = "ops", hours = 20 }
effect = { servers = "+1", chance = { p = 0.4, then = "launch.ddos-returns" } }

[[option]]
label = "Wait it out"
effect = { trust = -8, revenue_weeks_lost = 1 }

[field_note]
text = """
A DDoS is a budget question more than a technical one. Paying a provider
in advance costs less than absorbing an attack on the day.
"""
sources = ["https://www.cisa.gov/..."]   # shown to the player
topics = []                              # internal authoring references, never shown
```

Arcs are TOML files in `content/arcs/`, listing beats with conditions on the hidden campaign state. Controls, company types and milestones are data too. The engine knows only the expression language for conditions and effects (a small, safe evaluator; no general scripting).

An event can override its text or numbers per level (`[level.pro]`), so the beginner version explains terms and the pro version does not. A `content-check` command validates every file: schema, reachable options, `topics` ids that exist in identikey-dev `dag/claims/`, and that every option has a consequence.

## 15. Architecture and tech stack

**Engine:** a pure state machine, `step(state, input, rng) -> (state, output)`. No I/O inside. A seeded RNG makes every run reproducible from its seed and inputs, which gives shareable runs ("try seed 4817"), bug reports, and simulation for balancing.

**Front ends:** the engine emits text lines and prompts; front ends only print and read.

- **Terminal:** plain line-based output, like the 1985 game. No full-screen TUI is needed, which keeps the port simple and works over SSH and in any terminal. Optional colour.
- **Web:** the same engine compiled to WebAssembly (or run directly, see below) behind a terminal emulator (xterm.js) in a page. That page can be published as an Artifact, or hosted for a hackathon.
- **SSH (later):** a public SSH host for booths and talks, on a game-specific domain.

**Language: Rust** (decided 2026-10-28). It ships as one binary, compiles to WebAssembly cleanly for the web build, and suits a pure engine. Suggested crates: `serde` and `toml` for content, `rand_chacha` for a seeded, portable RNG, `wasm-bindgen` for the web build. No TUI framework. The content format does not depend on the language.

**Where it lives:** this repo (`markreveley/oregon-cyber-trails`), spec included. The game has no IdentiKey connection, so it does not depend on IdentiKey code. Only the optional `topics` check reads identikey-dev's `dag/`, from a bundle fetched in CI.

**Licence:** a game is a teaching product, not protocol. AGPL to match IdentiKey's product code, or a permissive licence to encourage classroom forks. Open question 7.

## 16. Balancing and testing

- **Bots.** Four scripted players run thousands of seeds in CI: *Random* (random choices), *Miser* (always cheapest), *Paranoid* (always the most secure), and *Textbook* (the choice the field notes recommend, budget permitting).
- **Targets, per level.** Textbook reaches the IPO about 85% of the time at Beginner, 60 to 75% at Intermediate, 40 to 55% at Advanced and 25 to 40% at Pro. Paranoid goes bankrupt more often than Textbook (security that costs too much is a failure too). Miser rarely gets past stage 3. Random rarely finishes. No single strategy dominates across company types.
- **Event coverage.** Every event appears in at least 5% of runs; every option is chosen by some bot.
- **Fairness check.** For every game-ending event, the run had at least one clue or warning at least one week earlier.
- **Human playtests** at each milestone, with three questions: was it fun, did any death feel unfair, what did you learn (compared with the field notes).
- **Unit tests** on the engine: determinism (same seed and inputs give the same run), effects, and the condition evaluator.

## 17. Modes: solo, classroom and hackathon

- **Solo.** The default: random seed, local tombstones, saved trail journal.
- **Classroom.** A facilitator gives everyone the same seed. Afterwards the class compares journals: same events, different choices, different deaths. A facilitator guide lists each arc and the discussion question it raises.
- **Hackathon.** Teams of one to three, as in *Identity under Attack*. A fixed seed with a harder arc mix, a shared leaderboard and tombstones. An optional "write an event" track asks teams to contribute a scenario in the content format, judged on accuracy and fun, which grows the event pool.
- **Daily trail (later).** One seed per day, as in daily puzzle games.

## 18. Plan and milestones

| Milestone | What | Done when |
|---|---|---|
| **M0. Paper prototype** | 15 events and one arc in a spreadsheet; play by hand with a die | Three playtesters finish a run and one dies fairly |
| **M1. Engine and stage 1** | Engine, terminal front end, content format, content check, 12 launch events, Vendor Expo, tombstones, Beginner level | A full stage plays end to end from a seed |
| **M2. Full trail** | All five stages, about 40 events, three arcs, river crossings, threat-hunting minigame, trail journal, Intermediate level | A 26-week run is complete; bots run in CI |
| **M3. Web, Advanced and Pro** | WebAssembly build behind xterm.js, published page, Advanced and Pro levels (adaptive attacker, timeline reconstruction), 60 events, five arcs, balance pass | Textbook bot within targets; five human playtests |
| **M4. Classroom and hackathon** | Fixed seeds, facilitator guide, leaderboard, shared tombstones, "write an event" guide | Run at one workshop or hackathon |

M0 needs no code and tests the core question (is making these calls fun?) before anything is built. M1 to M3 are each a few weeks of one person's evenings, mostly in content rather than engine.

## 19. Risks

| Risk | Mitigation |
|---|---|
| It turns into a quiz with a wagon painted on | The five kinds of judgement in [section 9](#9-events-and-judgement-calls); bot checks that no strategy dominates |
| Deaths feel random and unfair | Fairness check: every death is foreshadowed; the journal shows the clue |
| Too much text | Events capped at four lines; field notes at three |
| Pro is just Beginner with bigger numbers | Pro adds mechanics (adaptive attacker, timeline reconstruction, deadlines), not only harder odds |
| Content goes stale or wrong | Claim ids checked in CI; each generic event cites a source |
| Reads as an IdentiKey ad | No IdentiKey names or terms anywhere in the game; field notes cite vendor-neutral sources only |
| Writing 60 good events is the real work | Content is data; the hackathon "write an event" track |
| Teaching attack technique | Events describe what the defender sees, never how to attack |

## 20. Open questions

These were asked in the thread. Until answered, the spec assumes the default.

Answered by Mark on 2026-10-28:

1. **Audience.** Levels from beginner to pro ([section 8](#8-levels-beginner-to-pro)).
2. **Length of a run.** 15 to 25 minutes, replayable (Beginner is shorter, about 10).
3. **How closely it tracks IdentiKey.** No direct IdentiKey connection; the trail navigates the regions IdentiKey applies to. IdentiKey material is used only internally, as an authoring checklist.
4. **Tech stack.** Rust, compiled to WebAssembly for the web.
5. **Tone.** Dry Oregon Trail humour.
6. **Name.** Oregon Cyber Trails.

Still open:

7. **Licence.** Undecided (AGPL or permissive).
8. **Generic claims in the DAG.** Should DDoS, supply chain, certificates and domains get claims in identikey-dev's DAG so every event has an authoring reference, or stay outside it?
9. **First step.** Default: the M0 paper prototype before any code.
