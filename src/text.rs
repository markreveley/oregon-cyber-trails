//! Text helpers: word wrap, money, small ASCII art.

pub const WIDTH: usize = 72;

/// Word-wrap to `width`. An indented paragraph keeps its indent, and its
/// continuation lines are indented two further.
pub fn wrap(s: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for para in s.trim_end().trim_start_matches('\n').split('\n') {
        let indent = para.len() - para.trim_start().len();
        let mut line = " ".repeat(indent);
        let mut empty = true;
        for word in para.split_whitespace() {
            if !empty && line.len() + 1 + word.len() > width {
                let cont = if indent > 0 { indent + 2 } else { 0 };
                lines.push(std::mem::replace(&mut line, " ".repeat(cont)));
                empty = true;
            }
            if !empty {
                line.push(' ');
            }
            line.push_str(word);
            empty = false;
        }
        lines.push(line);
    }
    lines
}

pub fn money(v: i64) -> String {
    let neg = v < 0;
    let digits = v.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if neg {
        format!("-${out}")
    } else {
        format!("${out}")
    }
}

/// Round to two significant figures, for the cost ranges shown at Advanced.
pub fn round_sig(v: i64) -> i64 {
    if v < 100 {
        return v;
    }
    let mut m = 1;
    while v / m >= 100 {
        m *= 10;
    }
    (v + m / 2) / m * m
}

pub fn tombstone(name: &str, cause: &str, week: u32, epitaph: &str) -> Vec<String> {
    let inner = 21;
    let center = |s: &str| {
        let s: String = s.chars().take(inner).collect();
        let pad = inner - s.chars().count();
        format!(
            "|{}{}{}|",
            " ".repeat(pad / 2),
            s,
            " ".repeat(pad - pad / 2)
        )
    };
    let mut rows = vec![
        format!("   {}", "_".repeat(inner - 2)),
        format!("  /{}\\", " ".repeat(inner - 2)),
        format!(" {}", center("HERE LIES")),
        format!(" {}", center(&name.to_uppercase())),
        format!(" {}", center("")),
        format!(" {}", center("Died of")),
    ];
    for l in wrap(cause, inner - 2) {
        rows.push(format!(" {}", center(&l)));
    }
    rows.push(format!(" {}", center(&format!("Week {week}"))));
    rows.push(format!(" {}", center("")));
    for l in wrap(&format!("\"{epitaph}\""), inner - 2) {
        rows.push(format!(" {}", center(&l)));
    }
    rows.push(format!(" |{}|", "_".repeat(inner)));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_formats() {
        assert_eq!(money(0), "$0");
        assert_eq!(money(1500), "$1,500");
        assert_eq!(money(-1234567), "-$1,234,567");
    }

    #[test]
    fn wrap_respects_width() {
        let lines = wrap("one two three four five six seven", 10);
        assert!(lines.iter().all(|l| l.len() <= 10));
        assert_eq!(lines.join(" "), "one two three four five six seven");
    }

    #[test]
    fn wrap_keeps_indent() {
        let lines = wrap("  1. one two three four five", 12);
        assert_eq!(lines[0], "  1. one two");
        assert!(lines[1].starts_with("    "));
    }

    #[test]
    fn round_sig_two_figures() {
        assert_eq!(round_sig(1234), 1200);
        assert_eq!(round_sig(1260), 1300);
        assert_eq!(round_sig(87), 87);
    }
}
