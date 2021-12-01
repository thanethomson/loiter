//! String manipulation utilities.

/// Converts the given string into its "slugified" representation.
pub fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter_map(|c| match c {
            'a'..='z' | '0'..='9' => Some(c),
            '-' | ' ' | '_' => Some('-'),
            _ => None,
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod test {
    use super::slugify;

    #[test]
    fn test_slugify() {
        const TEST_CASES: &[(&str, &str)] = &[
            ("Project 1", "project-1"),
            ("String with spaces", "string-with-spaces"),
            ("String with emojis 😀", "string-with-emojis"),
            ("  String with leading spaces", "string-with-leading-spaces"),
            (
                "String with trailing spaces  ",
                "string-with-trailing-spaces",
            ),
            (
                "It's more complicated than that.",
                "its-more-complicated-than-that",
            ),
            (
                "\"Quoted strings\" slugify naturally",
                "quoted-strings-slugify-naturally",
            ),
        ];
        for (s, expected) in TEST_CASES {
            let actual = slugify(s);
            assert_eq!(actual, expected.to_string());
        }
    }
}
