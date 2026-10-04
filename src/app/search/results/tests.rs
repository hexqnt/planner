use super::highlight_job;

fn highlight(text: &str, indices: &[u32], snippet: bool) -> egui::text::LayoutJob {
    let ctx = egui::Context::default();
    let mut job = None;
    ctx.run_ui(egui::RawInput::default(), |ui| {
        job = Some(highlight_job(ui, text, indices, snippet));
    })
    .drop_without_applying_deltas();
    job.unwrap()
}

fn matched_text(job: &egui::text::LayoutJob) -> String {
    job.sections
        .iter()
        .filter(|section| section.format.underline.width > 0.0)
        .map(|section| &job.text[section.byte_range.start.0..section.byte_range.end.0])
        .collect()
}

#[test]
fn title_highlighting_preserves_graphemes_and_flattens_line_breaks() {
    let job = highlight("👩‍💻Cafe\u{301}\r\n встреча", &[1, 2, 3, 4], false);
    assert_eq!(job.text, "👩‍💻Cafe\u{301}  встреча");
    assert_eq!(matched_text(&job), "Cafe\u{301}");
    assert_eq!(job.sections.len(), 3);
}

#[test]
fn snippet_keeps_unicode_context_around_the_match_and_marks_both_cuts() {
    let text = format!("{}Cafe\u{301}{}", "а".repeat(80), "я".repeat(140));
    let job = highlight(&text, &[80, 81, 82, 83], true);
    assert_eq!(
        job.text,
        format!("…{}Cafe\u{301}{}…", "а".repeat(24), "я".repeat(72))
    );
    assert_eq!(matched_text(&job), "Cafe\u{301}");
    assert_eq!(job.sections.len(), 3);
}

#[test]
fn unhighlighted_text_uses_one_formatting_section() {
    let text = "Описание без совпадений. ".repeat(100);
    let job = highlight(&text, &[], false);
    assert_eq!(job.text, text);
    assert_eq!(job.sections.len(), 1);
}
