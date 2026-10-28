use crate::lexer::Word;

pub struct Spec {
    pub flags: &'static str,
    pub with_value: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args<'a> {
    pub flags: Vec<char>,
    pub operands: Vec<&'a Word>,
}

pub fn split<'a>(args: &'a [Word], spec: &Spec) -> Option<Args<'a>> {
    let mut flags = Vec::new();
    let mut operands = Vec::new();
    let mut words = args.iter();
    while let Some(word) = words.next() {
        let value = word.value.as_str();
        if value == "-" || value.starts_with("--") {
            return None;
        }
        let Some(cluster) = value.strip_prefix('-') else {
            operands.push(word);
            continue;
        };
        // GNU tools accept flags after operands, but BSD tools (macOS) read them as operands.
        if !operands.is_empty() {
            return None;
        }
        for (i, c) in cluster.char_indices() {
            if spec.flags.contains(c) {
                flags.push(c);
            } else if spec.with_value.contains(c) {
                flags.push(c);
                if i + c.len_utf8() == cluster.len() {
                    words.next()?;
                }
                break;
            } else {
                return None;
            }
        }
    }
    Some(Args { flags, operands })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: Spec = Spec {
        flags: "abc",
        with_value: "n",
    };

    fn words(input: &[&str]) -> Vec<Word> {
        input
            .iter()
            .map(|s| Word {
                raw: s.to_string(),
                value: s.to_string(),
            })
            .collect()
    }

    #[test]
    fn splits_flags_and_operands() {
        let cases: Vec<(&[&str], &str, &[&str])> = vec![
            (&[], "", &[]),
            (&["x"], "", &["x"]),
            (&["x", "y"], "", &["x", "y"]),
            (&["-a", "x"], "a", &["x"]),
            (&["-a", "-b", "x"], "ab", &["x"]),
            (&["-abc"], "abc", &[]),
            (&["-n", "5", "x"], "n", &["x"]),
            (&["-n5", "x"], "n", &["x"]),
            (&["-an", "5"], "an", &[]),
            (&["-an5"], "an", &[]),
            (&["-n", "-a"], "n", &[]),
            (&["-na"], "n", &[]),
            (&["x=-a"], "", &["x=-a"]),
            (&[""], "", &[""]),
        ];
        for (input, flags, operands) in cases {
            let args = words(input);
            let split = split(&args, &SPEC).expect("input must split");
            let split_operands: Vec<&str> = split.operands.iter().map(|w| w.raw.as_str()).collect();
            assert_eq!(
                split.flags,
                flags.chars().collect::<Vec<_>>(),
                "input: {input:?}"
            );
            assert_eq!(split_operands, operands, "input: {input:?}");
        }
    }

    #[test]
    fn rejects_unknown_or_ambiguous_args() {
        let cases: Vec<&[&str]> = vec![
            &["-d"],
            &["-ad"],
            &["-"],
            &["--"],
            &["--", "x"],
            &["--all"],
            &["-n"],
            &["-a", "-n"],
            &["x", "-a"],
            &["-ü"],
        ];
        for input in cases {
            assert_eq!(split(&words(input), &SPEC), None, "input: {input:?}");
        }
    }

    #[test]
    fn judges_quoted_words_by_value() {
        let args = vec![Word {
            raw: "'-d'".to_string(),
            value: "-d".to_string(),
        }];
        assert_eq!(split(&args, &SPEC), None);
    }
}
