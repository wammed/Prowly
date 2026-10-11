use prowly::i18n::{get_dictionary, interpolate, I18n, Language, TranslationEntry};
use std::collections::HashSet;

#[test]
fn test_dictionary_key_parity() {
    let en_dict = get_dictionary(Language::En);
    let ja_dict = get_dictionary(Language::Ja);

    let en_keys: HashSet<_> = en_dict.keys().copied().collect();
    let ja_keys: HashSet<_> = ja_dict.keys().copied().collect();

    let missing_in_ja: Vec<_> = en_keys.difference(&ja_keys).collect();
    let missing_in_en: Vec<_> = ja_keys.difference(&en_keys).collect();

    assert!(
        missing_in_ja.is_empty(),
        "Keys missing in Japanese dictionary: {:?}",
        missing_in_ja
    );
    assert!(
        missing_in_en.is_empty(),
        "Keys missing in English dictionary: {:?}",
        missing_in_en
    );
}

#[test]
fn test_placeholder_parity() {
    let en_dict = get_dictionary(Language::En);
    let ja_dict = get_dictionary(Language::Ja);

    fn extract_placeholders(s: &str) -> HashSet<String> {
        let mut placeholders = HashSet::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '{' {
                let mut var = String::new();
                for inner in chars.by_ref() {
                    if inner == '}' {
                        break;
                    }
                    var.push(inner);
                }
                if !var.is_empty() {
                    placeholders.insert(var);
                }
            }
        }
        placeholders
    }

    for (key, en_entry) in en_dict.iter() {
        let ja_entry = ja_dict.get(key).unwrap();

        match (en_entry, ja_entry) {
            (TranslationEntry::Plain(en_str), TranslationEntry::Plain(ja_str)) => {
                let en_vars = extract_placeholders(en_str);
                let ja_vars = extract_placeholders(ja_str);
                assert_eq!(
                    en_vars, ja_vars,
                    "Placeholders mismatch for key '{}': EN={:?} vs JA={:?}",
                    key, en_vars, ja_vars
                );
            }
            (
                TranslationEntry::Plural {
                    one: _,
                    other: en_other,
                },
                TranslationEntry::Plural {
                    one: _,
                    other: ja_other,
                },
            ) => {
                let en_vars = extract_placeholders(en_other);
                let ja_vars = extract_placeholders(ja_other);
                assert_eq!(
                    en_vars, ja_vars,
                    "Plural other placeholders mismatch for key '{}': EN={:?} vs JA={:?}",
                    key, en_vars, ja_vars
                );
            }
            _ => {
                panic!("Entry type mismatch for key '{}'", key);
            }
        }
    }
}

#[test]
fn test_interpolation_logic() {
    let template = "Hello {name}, you have {count} updates!";
    let rendered = interpolate(template, &[("name", "Alice"), ("count", "5")]);
    assert_eq!(rendered, "Hello Alice, you have 5 updates!");
}

#[test]
fn test_pluralization_and_language_switching() {
    let i18n = I18n::new(Language::En);

    // English singular
    assert_eq!(
        i18n.t_plural("changes.files_changed", 1, &[]),
        "1 file changed"
    );
    // English plural
    assert_eq!(
        i18n.t_plural("changes.files_changed", 4, &[]),
        "4 files changed"
    );

    // Switch to Japanese
    i18n.set_language(Language::Ja);
    assert_eq!(
        i18n.t_plural("changes.files_changed", 1, &[]),
        "変更ファイル 1件"
    );
    assert_eq!(
        i18n.t_plural("changes.files_changed", 4, &[]),
        "変更ファイル 4件"
    );

    // Toggle back to English
    assert_eq!(i18n.toggle_language(), Language::En);
    assert_eq!(i18n.t("header.synced"), "Synced");

    // Toggle to Japanese
    assert_eq!(i18n.toggle_language(), Language::Ja);
    assert_eq!(i18n.t("header.synced"), "同期完了");
}
