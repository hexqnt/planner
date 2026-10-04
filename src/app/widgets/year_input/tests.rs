use super::*;

#[test]
fn digit_buffer_filters_input_and_clamps_years_without_integer_overflow() {
    for (input, digits, year) in [
        ("20ab30", "2030", Some(2030)),
        ("12", "12", Some(1900)),
        (
            "9999999999999999999999",
            "9999999999999999999999",
            Some(2100),
        ),
        ("00002028", "00002028", Some(2028)),
        ("", "", None),
        ("год ２０２６ 🦀", "", None),
    ] {
        let mut buffer = Digits(String::new());
        assert_eq!(
            buffer.insert_text(input, CharIndex(0)),
            digits.len(),
            "{input}"
        );
        assert_eq!(buffer.as_str(), digits, "{input}");
        assert_eq!(buffer.year().map(Year::get), year, "{input}");
    }
}

#[test]
fn digit_buffer_inserts_at_character_positions_and_deletes_ranges() {
    let mut buffer = Digits("20".into());
    assert_eq!(buffer.insert_text("a2я6", CharIndex(2)), 2);
    assert_eq!(buffer.as_str(), "2026");
    assert_eq!(buffer.insert_text("x00", CharIndex(usize::MAX)), 2);
    assert_eq!(buffer.as_str(), "202600");
    assert_eq!(buffer.insert_text("1", CharIndex(0)), 1);
    assert_eq!(buffer.as_str(), "1202600");
    buffer.delete_char_range(CharIndex(1)..CharIndex(3));
    assert_eq!(buffer.as_str(), "12600");
    buffer.delete_char_range(CharIndex(0)..CharIndex(5));
    assert_eq!(buffer.as_str(), "");
    assert_eq!(buffer.year(), None);
}
