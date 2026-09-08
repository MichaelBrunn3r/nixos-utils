#[must_use]
pub fn dedent(input: &str) -> String {
    let lines = input.lines().collect::<Vec<_>>();
    let continuation_indent = lines
        .iter()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                (*line).to_owned()
            } else {
                line.get(continuation_indent..).unwrap_or("").to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[must_use]
pub fn fmt_snapshot_case(label: &str, fields: &[(&str, &str)]) -> String {
    let fields = fields
        .iter()
        .map(|(label, value)| {
            let value = value.trim_end().replace('\n', "\n        ");
            format!("{label}: `{value}`")
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{label}\n{fields}")
}
