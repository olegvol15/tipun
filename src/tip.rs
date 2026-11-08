const DIM: &str = "\x1b[2m";
const LABEL: &str = "\x1b[1;36m";
const SUGGESTION: &str = "\x1b[1;32m";
const RESET: &str = "\x1b[0m";
const TITLE: &str = " tipun ";
const MINT: &str = "\x1b[38;5;121m";
const GOLD: &str = "\x1b[38;5;221m";
const FACE: &str = "\x1b[38;5;23;48;5;121m";
const CREATURE: [&str; 4] = ["   ▄█▄   ", " ▄█████▄ ", " █>██_██ ", " ▀█▀▀▀█▀ "];
const CREATURE_WIDTH: usize = 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Terminal {
    pub columns: Option<usize>,
    pub color: bool,
}

pub fn render(suggestion: &str, reason: &str, terminal: Option<Terminal>) -> String {
    let plain = format!("tipun tip: {suggestion}\n  {reason}\n");
    let Some(terminal) = terminal else {
        return plain;
    };
    if !suggestion.is_ascii() || !reason.is_ascii() {
        return plain;
    }
    let width = suggestion.len().max(reason.len()).max(TITLE.len());
    let Some(columns) = terminal.columns.filter(|columns| width + 4 <= *columns) else {
        return plain;
    };
    let paint = |code: &str, text: &str| {
        if terminal.color {
            format!("{code}{text}{RESET}")
        } else {
            text.to_string()
        }
    };
    let border = |text: &str| paint(DIM, text);
    let line = |text: String, len: usize| {
        let pad = " ".repeat(width - len);
        format!("{} {text}{pad} {}", border("│"), border("│"))
    };
    let top = format!(
        "{}{}{}",
        border("╭"),
        paint(LABEL, TITLE),
        border(&format!("{}╮", "─".repeat(width + 2 - TITLE.len())))
    );
    let bottom = border(&format!("╰{}╯", "─".repeat(width + 2)));
    let lines = [
        top,
        line(paint(SUGGESTION, suggestion), suggestion.len()),
        line(paint(DIM, reason), reason.len()),
        bottom,
    ];
    let with_creature = width + 4 + CREATURE_WIDTH <= columns;
    let mut out = String::from("\n");
    for (row, line) in lines.iter().enumerate() {
        if with_creature {
            out.push_str(&creature(row, terminal.color));
            out.push_str("  ");
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn creature(row: usize, color: bool) -> String {
    let line = CREATURE[row];
    if !color {
        return line.to_string();
    }
    line.chars()
        .map(|c| match c {
            ' ' => c.to_string(),
            '>' | '_' => format!("{FACE}{c}{RESET}"),
            _ if row == 0 => format!("{GOLD}{c}{RESET}"),
            _ => format!("{MINT}{c}{RESET}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUGGESTION_TEXT: &str = "grep ERROR app.log";
    const REASON_TEXT: &str = "grep can read the file itself, no need for cat.";
    const BOX_ONLY: &str = "
╭ tipun ──────────────────────────────────────────╮
│ grep ERROR app.log                              │
│ grep can read the file itself, no need for cat. │
╰─────────────────────────────────────────────────╯
";

    fn boxed(columns: usize, color: bool) -> Option<Terminal> {
        Some(Terminal {
            columns: Some(columns),
            color,
        })
    }

    fn strip_colors(text: &str) -> String {
        let mut plain = String::new();
        let mut chars = text.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                chars.by_ref().find(|&c| c == 'm');
            } else {
                plain.push(c);
            }
        }
        plain
    }

    #[test]
    fn prints_plain_lines_when_not_on_a_terminal() {
        assert_eq!(
            render(SUGGESTION_TEXT, REASON_TEXT, None),
            "tipun tip: grep ERROR app.log\n  grep can read the file itself, no need for cat.\n"
        );
    }

    #[test]
    fn draws_the_creature_beside_the_box() {
        let expected = "
   ▄█▄     ╭ tipun ──────────────────────────────────────────╮
 ▄█████▄   │ grep ERROR app.log                              │
 █>██_██   │ grep can read the file itself, no need for cat. │
 ▀█▀▀▀█▀   ╰─────────────────────────────────────────────────╯
";
        assert_eq!(
            render(SUGGESTION_TEXT, REASON_TEXT, boxed(80, false)),
            expected
        );
        assert_eq!(
            render(SUGGESTION_TEXT, REASON_TEXT, boxed(62, false)),
            expected
        );
    }

    #[test]
    fn draws_only_the_box_when_the_creature_does_not_fit() {
        for columns in [51, 55, 61] {
            let output = render(SUGGESTION_TEXT, REASON_TEXT, boxed(columns, false));
            assert_eq!(output, BOX_ONLY, "columns: {columns}");
        }
    }

    #[test]
    fn colors_the_creature_and_the_box_when_allowed() {
        let colored = render(SUGGESTION_TEXT, REASON_TEXT, boxed(80, true));
        assert!(colored.contains("\x1b[1;36m tipun \x1b[0m"));
        assert!(colored.contains("\x1b[1;32mgrep ERROR app.log\x1b[0m"));
        assert!(colored.contains("\x1b[2m│\x1b[0m"));
        assert!(colored.contains("\x1b[38;5;221m█\x1b[0m"));
        assert!(colored.contains("\x1b[38;5;121m▀\x1b[0m"));
        assert!(colored.contains("\x1b[38;5;23;48;5;121m>\x1b[0m"));
        let plain = render(SUGGESTION_TEXT, REASON_TEXT, boxed(80, false));
        assert_eq!(strip_colors(&colored), plain);
        let box_only = render(SUGGESTION_TEXT, REASON_TEXT, boxed(55, true));
        assert_eq!(strip_colors(&box_only), BOX_ONLY);
    }

    #[test]
    fn keeps_a_minimum_width_for_the_title() {
        let expected = "
╭ tipun ──╮
│ ls      │
│ short   │
╰─────────╯
";
        assert_eq!(render("ls", "short", boxed(15, false)), expected);
    }

    #[test]
    fn falls_back_to_plain_lines() {
        let plain = render(SUGGESTION_TEXT, REASON_TEXT, None);
        let cases = [
            boxed(50, false),
            boxed(50, true),
            Some(Terminal {
                columns: None,
                color: true,
            }),
        ];
        for terminal in cases {
            assert_eq!(
                render(SUGGESTION_TEXT, REASON_TEXT, terminal),
                plain,
                "{terminal:?}"
            );
        }
        assert_eq!(
            render("grep x 'café'", REASON_TEXT, boxed(80, false)),
            render("grep x 'café'", REASON_TEXT, None)
        );
    }
}
