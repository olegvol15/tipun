const DIM: &str = "\x1b[2m";
const LABEL: &str = "\x1b[1;36m";
const SUGGESTION: &str = "\x1b[1;32m";
const RESET: &str = "\x1b[0m";
const TITLE: &str = " tipun ";

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
    if terminal.columns.is_none_or(|columns| width + 4 > columns) {
        return plain;
    }
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
        format!("{} {text}{pad} {}\n", border("│"), border("│"))
    };
    let top = format!(
        "{}{}{}\n",
        border("╭"),
        paint(LABEL, TITLE),
        border(&format!("{}╮", "─".repeat(width + 2 - TITLE.len())))
    );
    let bottom = border(&format!("╰{}╯", "─".repeat(width + 2)));
    format!(
        "\n{top}{}{}{bottom}\n",
        line(paint(SUGGESTION, suggestion), suggestion.len()),
        line(paint(DIM, reason), reason.len()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUGGESTION_TEXT: &str = "grep ERROR app.log";
    const REASON_TEXT: &str = "grep can read the file itself, no need for cat.";

    fn boxed(columns: usize, color: bool) -> Option<Terminal> {
        Some(Terminal {
            columns: Some(columns),
            color,
        })
    }

    #[test]
    fn prints_plain_lines_when_not_on_a_terminal() {
        assert_eq!(
            render(SUGGESTION_TEXT, REASON_TEXT, None),
            "tipun tip: grep ERROR app.log\n  grep can read the file itself, no need for cat.\n"
        );
    }

    #[test]
    fn draws_a_box_on_a_terminal() {
        let expected = "
╭ tipun ──────────────────────────────────────────╮
│ grep ERROR app.log                              │
│ grep can read the file itself, no need for cat. │
╰─────────────────────────────────────────────────╯
";
        assert_eq!(
            render(SUGGESTION_TEXT, REASON_TEXT, boxed(80, false)),
            expected
        );
    }

    #[test]
    fn colors_the_box_when_allowed() {
        let colored = render(SUGGESTION_TEXT, REASON_TEXT, boxed(80, true));
        assert!(colored.contains("\x1b[1;36m tipun \x1b[0m"));
        assert!(colored.contains("\x1b[1;32mgrep ERROR app.log\x1b[0m"));
        assert!(colored.contains("\x1b[2m│\x1b[0m"));
        let stripped = colored
            .replace(DIM, "")
            .replace(LABEL, "")
            .replace(SUGGESTION, "")
            .replace(RESET, "");
        assert_eq!(
            stripped,
            render(SUGGESTION_TEXT, REASON_TEXT, boxed(80, false))
        );
    }

    #[test]
    fn keeps_a_minimum_width_for_the_title() {
        let expected = "
╭ tipun ──╮
│ ls      │
│ short   │
╰─────────╯
";
        assert_eq!(render("ls", "short", boxed(80, false)), expected);
    }

    #[test]
    fn falls_back_to_plain_lines() {
        let plain = render(SUGGESTION_TEXT, REASON_TEXT, None);
        let narrow = REASON_TEXT.len() + 3;
        let cases = [
            boxed(narrow, false),
            boxed(narrow, true),
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
        assert!(render(SUGGESTION_TEXT, REASON_TEXT, boxed(narrow + 1, false)).contains('╭'));
    }
}
