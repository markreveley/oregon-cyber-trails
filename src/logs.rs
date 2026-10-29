//! The threat-hunting minigame: spot the one suspicious line in a log.

use crate::rng::Rng;

const USERS: &[&str] = &[
    "ana",
    "ben",
    "priya",
    "lee",
    "sam",
    "svc-billing",
    "svc-deploy",
];
const PLACES: &[&str] = &["London", "London", "London", "Manchester", "Leeds"];

fn normal(rng: &mut Rng) -> String {
    let u = USERS[rng.index(USERS.len())];
    let p = PLACES[rng.index(PLACES.len())];
    let h = rng.range(8, 18);
    let m = rng.range(0, 59);
    match rng.index(8) {
        0 => format!("{h:02}:{m:02} sign-in ok      user={u} from={p}"),
        1 => format!(
            "{h:02}:{m:02} deploy          user={u} service=web version=1.{}.{}",
            rng.range(2, 9),
            rng.range(0, 30)
        ),
        2 => format!("{h:02}:{m:02} sign-in failed  user={u} from={p} reason=wrong-password"),
        3 => format!("{h:02}:{m:02} token refresh   user={u} client=mobile"),
        4 => format!(
            "{h:02}:{m:02} export          user={u} rows={}",
            rng.range(10, 400)
        ),
        5 => format!("{h:02}:{m:02} mfa ok          user={u} from={p}"),
        6 => format!("{h:02}:{m:02} config change   user={u} key=feature-flags"),
        _ => format!(
            "{h:02}:{m:02} api call        user={u} endpoint=/v1/orders count={}",
            rng.range(1, 60)
        ),
    }
}

/// Obvious anomalies, for the lower levels.
fn loud(rng: &mut Rng) -> String {
    let u = USERS[rng.index(5)];
    match rng.index(5) {
        0 => format!(
            "03:{:02} sign-in ok      user={u} from=Pyongyang",
            rng.range(0, 59)
        ),
        1 => format!(
            "14:{:02} sign-in failed  user={u} from=unknown count=4812",
            rng.range(0, 59)
        ),
        2 => format!(
            "02:{:02} export          user={u} rows=812,330",
            rng.range(0, 59)
        ),
        3 => format!(
            "04:{:02} deploy key add  user=sam key=ssh-ed25519 from=unknown",
            rng.range(0, 59)
        ),
        _ => format!(
            "23:{:02} mfa disabled    user={u} from=unknown",
            rng.range(0, 59)
        ),
    }
}

/// Subtle anomalies, for Advanced and Pro.
fn quiet(rng: &mut Rng) -> String {
    let u = USERS[rng.index(5)];
    match rng.index(5) {
        0 => format!(
            "{:02}:{:02} sign-in ok      user={u} from=Jakarta",
            rng.range(9, 17),
            rng.range(0, 59)
        ),
        1 => format!(
            "{:02}:{:02} token refresh   user=svc-billing client=browser",
            rng.range(9, 17),
            rng.range(0, 59)
        ),
        2 => format!(
            "{:02}:{:02} export          user=svc-deploy rows={}",
            rng.range(9, 17),
            rng.range(0, 59),
            rng.range(2000, 9000)
        ),
        3 => format!(
            "{:02}:{:02} config change   user={u} key=mail-forwarding",
            rng.range(9, 17),
            rng.range(0, 59)
        ),
        _ => format!(
            "{:02}:{:02} sign-in ok      user={u} from=London client=curl/8.4",
            rng.range(9, 17),
            rng.range(0, 59)
        ),
    }
}

/// A log of `n` lines with one anomaly. Returns the lines and the index of
/// the anomaly.
pub fn generate(rng: &mut Rng, n: usize, subtle: bool) -> (Vec<String>, usize) {
    let mut lines: Vec<String> = (0..n - 1).map(|_| normal(rng)).collect();
    let bad = if subtle { quiet(rng) } else { loud(rng) };
    let at = rng.index(n);
    lines.insert(at, bad);
    (lines, at)
}
