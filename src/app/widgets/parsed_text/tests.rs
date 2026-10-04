use super::*;

#[test]
fn optional_links_normalize_and_invalid_values_preserve_the_draft() {
    let mut draft = LinkDraft::from("  HTTPS://EXAMPLE.COM/call  ");
    draft.finish_edit();
    assert_eq!(draft.as_str(), "https://example.com/call");
    assert_eq!(
        draft.take().unwrap().unwrap().as_str(),
        "https://example.com/call"
    );
    assert_eq!(draft.as_str(), "");
    assert!(draft.value().unwrap().is_none());
    for text in [
        "example.com",
        "mailto:a@example.com",
        "https://example.com/a b",
    ] {
        let mut draft = LinkDraft::from(text);
        draft.finish_edit();
        assert_eq!(draft.take(), Err(InputError::EventLink));
        assert_eq!(draft.as_str(), text);
    }
    assert!(LinkDraft::from(" \t").take().unwrap().is_none());
}

#[test]
fn email_lists_parse_separators_and_normalize_without_losing_addresses() {
    let mut draft = EmailListDraft::from(" a+b@example.com;\nfirst.last@EXAMPLE.com, , ");
    draft.finish_edit();
    assert_eq!(draft.as_str(), "a+b@example.com\nfirst.last@EXAMPLE.com");
    let emails = draft.take().unwrap();
    assert_eq!(emails.len(), 2);
    assert_eq!(emails[0].as_str(), "a+b@example.com");
    assert_eq!(emails[1].as_str(), "first.last@EXAMPLE.com");
    assert_eq!(draft.value().unwrap().as_slice(), []);
    let mut invalid = EmailListDraft::from("ok@example.com, bad-email");
    assert_eq!(invalid.take(), Err(InputError::Participant));
    assert_eq!(invalid.as_str(), "ok@example.com, bad-email");
}
