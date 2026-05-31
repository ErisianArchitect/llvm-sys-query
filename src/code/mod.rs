use std::path::Path;

use rustdoc_types::Span;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("File was not found")]
    FileNotFound,
}

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

pub fn unindent_lines(lines: &[&str]) -> String {
    let mut min_indent = usize::MAX;
    let mut total_len = 0usize;
    for line in lines.iter().copied() {
        let mut line_slice = line;
        let mut indent_end = 0usize;
        while !line_slice.is_empty() {
            if line_slice.starts_with("    ") {
                line_slice = &line_slice[4..];
                indent_end += 4;
            } else {
                break;
            }
        }
        total_len += line_slice.len() + 1;
        min_indent = min_indent.min(indent_end);
    }
    let mut output = String::with_capacity(total_len);
    for (i, line) in lines.iter().copied().enumerate() {
        if i > 0 {
            output.push('\n');
        }
        output.push_str(&line[min_indent..]);
    }
    output
}

pub fn query_code<R: AsRef<Path>>(root: R, span: &Span) -> Result<String> {
    fn query_code(root: &Path, span: &Span) -> Result<String> {
        let full_path = root.join(&span.filename);
        if !full_path.is_file() {
            return Err(Error::FileNotFound);
        }
        let source_text = std::fs::read_to_string(&full_path)?;

        let lines = source_text.lines().collect::<Vec<_>>();
        let start_line = span.begin.0 - 1;
        let end_line = span.end.0;
        let sublines = &lines[start_line..end_line];
        let source_content = unindent_lines(sublines);
        Ok(source_content)
    }
    query_code(root.as_ref(), span)
}
