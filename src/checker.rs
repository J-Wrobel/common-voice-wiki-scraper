use crate::config::Config;
use toml::Value;
use regex::Regex;

pub fn check(rules: &Config, raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.len() < rules.min_trimmed_length
        || rules.quote_start_with_letter
            && trimmed.chars().nth(0) == Some('"')
            && trimmed
                .chars()
                .nth(1)
                .map(|c| !c.is_alphabetic())
                .unwrap_or_default()
        || trimmed.chars().filter(|c| c.is_alphabetic()).count() < rules.min_characters
        || !rules.may_end_with_colon && trimmed.ends_with(':')
        || rules.needs_punctuation_end && trimmed.ends_with(|c: char| c.is_alphabetic())
        || rules.needs_letter_start && trimmed.starts_with(|c: char| !c.is_alphabetic())
        || rules.needs_uppercase_start && trimmed.starts_with(|c: char| c.is_lowercase())
        || trimmed.contains('\n')
        || trimmed.contains(char::is_numeric)
    {
        return false;
    }

    let invalid_symbols = if !rules.allowed_symbols_regex.is_empty() {
            let regex = Regex::new(&rules.allowed_symbols_regex).unwrap();
            trimmed.chars().any(|c| {
                !regex.is_match(&c.to_string())
            })
        } else {
            trimmed.chars().any(|c| {
                rules.disallowed_symbols.contains(&Value::try_from(c).unwrap())
            })
        };

    if invalid_symbols {
        return false;
    }

    if rules.broken_whitespace.iter().any(|broken| trimmed.contains(Value::as_str(broken).unwrap())) {
        return false;
    }

    let words = trimmed.split_whitespace();
    let word_count = words.clone().count();
    if word_count < rules.min_word_count
        || word_count > rules.max_word_count
        || words.into_iter().any(|word| rules.disallowed_words.contains(
             &word.trim_matches(|c: char| !c.is_alphabetic()).to_lowercase()
           ))
    {
        return false;
    }

    let abbr = rules.abbreviation_patterns.iter().any(|pattern| {
        let regex = Regex::new(Value::as_str(pattern).unwrap()).unwrap();
        regex.is_match(&trimmed)
    });
    if abbr {
        return false;
    }

    if !rules.even_symbols.is_empty() {
        let has_uneven_symbols = rules.even_symbols.iter().any(|even_symbol| {
            let count = trimmed.matches(Value::as_str(even_symbol).unwrap()).count();
            return count % 2 != 0;
        });
        if has_uneven_symbols {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::load_config;
    use toml::Value;

    #[test]
    fn test_min_trimmed_length() {
        let rules : Config = Config {
            min_trimmed_length: 3,
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("  aa     ")), false);
        assert_eq!(check(&rules, &String::from("  aaa     ")), true);
    }

    #[test]
    fn test_min_word_count() {
        let rules : Config = Config {
            min_word_count: 2,
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("one")), false);
        assert_eq!(check(&rules, &String::from("two words")), true);
    }

    #[test]
    fn test_max_word_count() {
        let rules : Config = Config {
            max_word_count: 2,
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("three words now")), false);
        assert_eq!(check(&rules, &String::from("two words")), true);
    }

    #[test]
    fn test_min_characters() {
        let rules : Config = Config {
            min_characters: 3,
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("no!!")), false);
        assert_eq!(check(&rules, &String::from("yes!")), true);
    }

    #[test]
    fn test_may_end_with_colon() {
        let mut rules : Config = Config {
            may_end_with_colon: false,
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("ends with colon:")), false);

        rules = Config {
            may_end_with_colon: true,
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("ends with colon:")), true);
    }

    #[test]
    fn test_quote_start_with_letter() {
        let mut rules : Config = Config {
            quote_start_with_letter: false,
            needs_letter_start: false,
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("\"😊 foo")), true);

        rules = Config {
            quote_start_with_letter: true,
            needs_letter_start: false,
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("\"😊 foo")), false);
    }

    #[test]
    fn test_needs_punctuation_end() {
        let mut rules : Config = Config {
            needs_punctuation_end: false,
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("This has no punctuation")), true);
        assert_eq!(check(&rules, &String::from("This has punctuation.")), true);

        rules = Config {
            needs_punctuation_end: true,
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("This has no punctuation")), false);
        assert_eq!(check(&rules, &String::from("This has punctuation.")), true);
    }

    #[test]
    fn test_needs_letter_start() {
        let mut rules : Config = Config {
            needs_letter_start: false,
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("?Foo")), true);
        assert_eq!(check(&rules, &String::from("This has a normal start")), true);

        rules = Config {
            needs_letter_start: true,
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("?Foo")), false);
        assert_eq!(check(&rules, &String::from("This has a normal start")), true);
    }

    #[test]
    fn test_needs_uppercase_start() {
        let mut rules : Config = Config {
            needs_uppercase_start: false,
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("foo")), true);
        assert_eq!(check(&rules, &String::from("Foo")), true);

        rules = Config {
            needs_uppercase_start: true,
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("foo")), false);
        assert_eq!(check(&rules, &String::from("Foo")), true);
    }

    #[test]
    fn test_disallowed_symbols() {
        let rules : Config = Config {
            disallowed_symbols: vec![Value::try_from('%').unwrap()],
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("This has no percentage but other & characters")), true);
        assert_eq!(check(&rules, &String::from("This has a %")), false);
    }

    #[test]
    fn test_allowed_symbols_regex() {
        let rules : Config = Config {
            allowed_symbols_regex: String::from("[\u{0020}-\u{005A}]"),
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("ONLY UPPERCASE AND SPACE IS ALLOWED")), true);
        assert_eq!(check(&rules, &String::from("This is not uppercase")), false);
    }

    #[test]
    fn test_allowed_symbols_regex_over_disallowed() {
        let rules : Config = Config {
            allowed_symbols_regex: String::from("[\u{0020}-\u{005A}]"),
            disallowed_symbols: vec![Value::try_from('O').unwrap()],
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("ONLY UPPERCASE AND SPACE IS ALLOWED AND DISALLOWED O IS OKAY")), true);
    }

    #[test]
    fn test_disallowed_words() {
        let rules : Config = Config {
            disallowed_words: ["blerg"].iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("This has blerg")), false);
        assert_eq!(check(&rules, &String::from("This has a capital bLeRg")), false);
        assert_eq!(check(&rules, &String::from("This has many blergs blerg blerg blerg")), false);
        assert_eq!(check(&rules, &String::from("Here is a blerg, with comma")), false);
        assert_eq!(check(&rules, &String::from("This hasn't bl e r g")), true);

        let rules : Config = Config {
            disallowed_words: ["a's"].iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        assert_eq!(check(&rules, &String::from("This has a's")), false);
    }

    #[test]
    fn test_broken_whitespace() {
        let rules : Config = Config {
            broken_whitespace: vec![Value::try_from("  ").unwrap()],
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("This has no broken whitespace")), true);
        assert_eq!(check(&rules, &String::from("This has  broken whitespace")), false);
    }

    #[test]
    fn test_abbreviation_patterns() {
        let rules : Config = Config {
            abbreviation_patterns: vec![Value::try_from("[A-Z]{2}").unwrap()],
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("This no two following uppercase letters")), true);
        assert_eq!(check(&rules, &String::from("This has two FOllowing uppercase letters")), false);
    }

    #[test]
    fn test_uneven_quotes_allowed_default() {
        let rules : Config = Config {
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("This has \"uneven quotes and it is fine!")), true);
    }

    #[test]
    fn test_uneven_quotes_allowed() {
        let rules : Config = Config {
            even_symbols: vec![],
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("This has \"uneven quotes and it is fine!")), true);
        assert_eq!(check(&rules, &String::from("This has (uneven parenthesis and it is fine!")), true);
    }

    #[test]
    fn test_uneven_quotes_not_allowed() {
        let rules : Config = Config {
            even_symbols: vec![Value::try_from("\"").unwrap(), Value::try_from("(").unwrap()],
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("This has \"uneven quotes and it is not fine!")), false);
        assert_eq!(check(&rules, &String::from("This has (uneven parenthesis and it is not fine!")), false);
    }

    #[test]
    fn test_uneven_quotes_not_allowed_even() {
        let rules : Config = Config {
            even_symbols: vec![Value::try_from("\"").unwrap()],
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("This has \"even\" quotes and it is fine!")), true);
    }

    #[test]
    fn test_uneven_quotes_not_allowed_multiple() {
        let rules : Config = Config {
            even_symbols: vec![Value::try_from("\"").unwrap(), Value::try_from("'").unwrap()],
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("This has \"uneven quotes' and it is fine!")), false);
    }

    #[test]
    fn test_uneven_quotes_not_allowed_multiple_one_ok() {
        let rules : Config = Config {
            even_symbols: vec![Value::try_from("\"").unwrap(), Value::try_from("'").unwrap()],
            ..Default::default()
        };

        assert_eq!(check(&rules, &String::from("This has \"uneven\" quotes' and it is fine!")), false);
    }

    #[test]
    fn test_english() {
        let rules : Config = load_config("english");

        assert_eq!(check(&rules, &String::from("")), false);
        assert_eq!(check(&rules, &String::from("\"😊")), false);
        assert_eq!(check(&rules, &String::from("This ends with:")), false);
        assert_eq!(check(&rules, &String::from(" AA ")), false);
        assert_eq!(check(&rules, &String::from("This has broken  space")), false);
        assert_eq!(check(&rules, &String::from("This as well !")), false);
        assert_eq!(check(&rules, &String::from("And this ;")), false);
        assert_eq!(check(&rules, &String::from("This is gonna be way way way way way way way way way way too long")), false);
        assert_eq!(check(&rules, &String::from("This is absolutely valid.")), true);
        assert_eq!(check(&rules, &String::from("This contains 1 number")), false);
        assert_eq!(check(&rules, &String::from("this is lowercase")), true);
        assert_eq!(check(&rules, &String::from("foo\n\nfoo")), false);
        assert_eq!(check(&rules, &String::from("foo\\foo")), false);
        assert_eq!(check(&rules, &String::from("foo<>")), false);
        assert_eq!(check(&rules, &String::from("foo*@")), false);
        assert_eq!(check(&rules, &String::from("A.B")), false);
        assert_eq!(check(&rules, &String::from("S.T.A.L.K.E.R.")), false);
    }

    #[test]
    fn test_french() {
        let rules : Config = load_config("french");

        assert_eq!(check(&rules, &String::from("")), false);
        assert_eq!(check(&rules, &String::from("\"😊")), false);
        assert_eq!(check(&rules, &String::from("This ends with:")), false);
        assert_eq!(check(&rules, &String::from("This does not end with a period")), false);
        assert_eq!(check(&rules, &String::from("?This does not start with a letter")), false);
        assert_eq!(check(&rules, &String::from("this starts with lowercase")), false);
        assert_eq!(check(&rules, &String::from(" AA ")), false);
        assert_eq!(check(&rules, &String::from("This has broken  space")), false);
        assert_eq!(check(&rules, &String::from("This as well !")), false);
        assert_eq!(check(&rules, &String::from("And this ;")), false);
        assert_eq!(check(&rules, &String::from("This is gonna be way way way way way way way way way way too long")), false);
        assert_eq!(check(&rules, &String::from("Short")), false);
        assert_eq!(check(&rules, &String::from("This is absolutely validé.")), true);
        assert_eq!(check(&rules, &String::from("No!!!")), false);
        assert_eq!(check(&rules, &String::from("This contains 1 number")), false);
        assert_eq!(check(&rules, &String::from("foo\n\nfoo")), false);
        assert_eq!(check(&rules, &String::from("foo<>")), false);
        assert_eq!(check(&rules, &String::from("foo«")), false);
        assert_eq!(check(&rules, &String::from("foo*@")), false);
        assert_eq!(check(&rules, &String::from("A.B")), false);
        assert_eq!(check(&rules, &String::from("S.T.A.L.K.E.R.")), false);
        assert_eq!(check(&rules, &String::from("Some sentence that ends with A.")), false);
    }

    #[test]
    fn test_german() {
        let rules : Config = load_config("german");

        assert_eq!(check(&rules, &String::from("Dies ist ein korrekter Satz.")), true);
        assert_eq!(check(&rules, &String::from("Satzzeichen in der Mitte. Wird nicht akzeptiert.")), false);
        assert_eq!(check(&rules, &String::from("Satzzeichen in der Mitte? Wird nicht akzeptiert.")), false);
        assert_eq!(check(&rules, &String::from("Satzzeichen in der Mitte! Wird nicht akzeptiert.")), false);
        assert_eq!(check(&rules, &String::from("Französische Satzzeichen werden ignorierté.")), false);
        assert_eq!(check(&rules, &String::from("Andere Satzzeichen wie Åblabla werden auch ignoriert.")), false);
        assert_eq!(check(&rules, &String::from("Γεια σας")), false);
        assert_eq!(check(&rules, &String::from("Sätze dürfen keine Wörter mit nur einem B Buchstaben haben.")), false);
        assert_eq!(check(&rules, &String::from("A auch nicht am Anfang.")), false);
        assert_eq!(check(&rules, &String::from("Oder am Ende e.")), false);
        assert_eq!(check(&rules, &String::from("Oder am Ende e.")), false);
        assert_eq!(check(&rules, &String::from("AmSi ist eine schwarze Masse, isomorph mit LaSi")), false);
        assert_eq!(check(&rules, &String::from("Die Aussperrung ist nach Art.")), false);
        assert_eq!(check(&rules, &String::from("Remy & Co.")), false);
        assert_eq!(check(&rules, &String::from("Es ist die sog.")), false);
        assert_eq!(check(&rules, &String::from("Kein deutsches Wort: ambiguous.")), false);
        assert_eq!(check(&rules, &String::from("Bundesliga am Anfang eines Satzes.")), false);
        assert_eq!(check(&rules, &String::from("Liga am Anfang eines Satzes.")), false);
        assert_eq!(check(&rules, &String::from("Abkürzung am Ende hl.")), false);
        assert_eq!(check(&rules, &String::from("Abkürzung am Ende geb.")), false);
    }
}
