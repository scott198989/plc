const CORPUS: &str = include_str!("../../tools/phase2/isolation-fuzz-corpus.tsv");

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FuzzCase {
    pub category: String,
    pub id: String,
    pub value: String,
}

pub fn cases() -> Vec<FuzzCase> {
    let cases = CORPUS
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let mut fields = line.splitn(3, '\t');
            let category = fields.next().expect("fuzz category");
            let id = fields.next().expect("fuzz case id");
            let encoded = fields.next().expect("fuzz JSON string");
            assert!(!category.is_empty() && !id.is_empty());
            FuzzCase {
                category: category.to_owned(),
                id: id.to_owned(),
                value: decode_json_string(encoded),
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(cases.len(), 27, "canonical fuzz corpus size");
    let mut ids = cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), cases.len(), "canonical fuzz case IDs are unique");
    cases
}

fn decode_json_string(encoded: &str) -> String {
    let inner = encoded
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .expect("fuzz value is one JSON string");
    let mut decoded = String::new();
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            assert!(!character.is_control(), "unescaped JSON control character");
            decoded.push(character);
            continue;
        }
        match characters.next().expect("complete JSON escape") {
            '"' => decoded.push('"'),
            '\\' => decoded.push('\\'),
            '/' => decoded.push('/'),
            'b' => decoded.push('\u{0008}'),
            'f' => decoded.push('\u{000c}'),
            'n' => decoded.push('\n'),
            'r' => decoded.push('\r'),
            't' => decoded.push('\t'),
            'u' => {
                let digits = (0..4)
                    .map(|_| characters.next().expect("complete JSON Unicode escape"))
                    .collect::<String>();
                let scalar = u32::from_str_radix(&digits, 16).expect("hex JSON Unicode escape");
                decoded.push(char::from_u32(scalar).unwrap_or(char::REPLACEMENT_CHARACTER));
            }
            escape => panic!("unsupported JSON escape {escape}"),
        }
    }
    decoded
}

#[cfg(test)]
mod tests {
    use super::cases;

    #[test]
    fn canonical_corpus_decodes_nul_backslash_and_surrogate_cases() {
        let cases = cases();
        assert!(cases.iter().any(|case| case.value.contains('\0')));
        assert!(cases.iter().any(|case| case.value.starts_with(r"\\")));
        assert!(
            cases
                .iter()
                .any(|case| case.value.contains(char::REPLACEMENT_CHARACTER))
        );
    }
}
