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

#[must_use]
pub fn fmt_snapshot_cases<Cases, Case, Format>(cases: Cases, format: Format) -> String
where
    Cases: IntoIterator<Item = Case>,
    Format: FnMut(Case) -> String,
{
    cases
        .into_iter()
        .map(format)
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[must_use]
/// # Panics
///
/// Panics if miette cannot render the diagnostic report.
pub fn fmt_diagnostic_case<E>(label: &str, input: &str, error: E) -> String
where
    E: miette::Diagnostic + Send + Sync + 'static,
{
    let handler = miette::GraphicalReportHandler::new_themed(miette::GraphicalTheme::none());
    let report = miette::Report::new(error)
        .with_source_code(miette::NamedSource::new("input.dj", input.to_owned()));
    let mut rendered = String::new();
    handler
        .render_report(&mut rendered, report.as_ref())
        .expect("rendering a diagnostic should succeed");
    fmt_snapshot_case(label, &[("error", &rendered)])
}
