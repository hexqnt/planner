//! Канонизация регистронезависимых токенов до разбора: `icalendar` определяет тип и экранирование по именам в верхнем регистре.

pub(super) fn unfold(input: &str) -> String {
    let mut output = icalendar::parser::unfold(input);
    let mut start = 0;
    while start < output.len() {
        let end = output[start..]
            .find('\n')
            .map_or(output.len(), |index| start + index + 1);
        normalize_line(&mut output[start..end]);
        start = end;
    }
    output
}

fn delimiter(input: &str) -> Option<usize> {
    let mut quoted = false;
    input.char_indices().find_map(|(index, character)| {
        if character == '"' {
            quoted = !quoted;
        }
        (!quoted && matches!(character, ';' | ':')).then_some(index)
    })
}

fn normalize_line(line: &mut str) {
    let Some(name_end) = delimiter(line) else {
        return;
    };
    line[..name_end].make_ascii_uppercase();
    let uppercase_value = matches!(
        &line[..name_end],
        "BEGIN" | "END" | "METHOD" | "STATUS" | "TRANSP" | "RRULE"
    );
    let mut start = name_end;
    while line[start..].starts_with(';') {
        start += 1;
        let Some(length) = delimiter(&line[start..]) else {
            return;
        };
        let end = start + length;
        let parameter = &mut line[start..end];
        let key_end = parameter.find('=').unwrap_or(parameter.len());
        parameter[..key_end].make_ascii_uppercase();
        if &parameter[..key_end] == "VALUE" {
            parameter[key_end..].make_ascii_uppercase();
        }
        start = end;
    }
    if uppercase_value {
        line[start + 1..].make_ascii_uppercase();
    }
}
