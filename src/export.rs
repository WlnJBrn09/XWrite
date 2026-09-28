//! Export document body to md / txt / docx / pdf (local downloads).

use std::io::{Cursor, Write};

use serde::Deserialize;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[derive(Debug, Deserialize)]
pub struct ExportBody {
    pub format: String,
    pub title: Option<String>,
    pub html: String,
    pub markdown: Option<String>,
}

#[derive(Debug)]
pub struct ExportFile {
    pub filename: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub enum ExportError {
    Unsupported,
    Other(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "unsupported export format"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

pub fn export_document(body: &ExportBody) -> Result<ExportFile, ExportError> {
    let title = sanitize_filename(body.title.as_deref().unwrap_or("document"));
    let plain = html_to_plain(&body.html);
    match body.format.to_lowercase().as_str() {
        "txt" => Ok(ExportFile {
            filename: format!("{title}.txt"),
            content_type: "text/plain; charset=utf-8".into(),
            bytes: plain.into_bytes(),
        }),
        "md" | "markdown" => {
            let md = body
                .markdown
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| html_to_markdown(&body.html));
            Ok(ExportFile {
                filename: format!("{title}.md"),
                content_type: "text/markdown; charset=utf-8".into(),
                bytes: md.into_bytes(),
            })
        }
        "docx" => {
            let bytes = build_docx(&title, &plain).map_err(ExportError::Other)?;
            Ok(ExportFile {
                filename: format!("{title}.docx"),
                content_type:
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document".into(),
                bytes,
            })
        }
        "pdf" => {
            let bytes = build_simple_pdf(&title, &plain).map_err(ExportError::Other)?;
            Ok(ExportFile {
                filename: format!("{title}.pdf"),
                content_type: "application/pdf".into(),
                bytes,
            })
        }
        _ => Err(ExportError::Unsupported),
    }
}

fn sanitize_filename(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| {
            // Keep letters from every script; headers carry them via RFC 5987 `filename*`.
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let t = t.trim().trim_matches('.');
    if t.is_empty() {
        "document".into()
    } else {
        t.chars().take(80).collect()
    }
}

pub fn html_to_plain(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('<') {
            if let Some(end) = after.find('>') {
                let tag = after[..end].trim().to_ascii_lowercase();
                let name = tag
                    .trim_start_matches('/')
                    .split(|c: char| c.is_whitespace() || c == '/')
                    .next()
                    .unwrap_or("");
                let closing = tag.starts_with('/');
                if name == "br" {
                    out.push('\n');
                } else if matches!(name, "td" | "th") && closing {
                    out.push('\t');
                } else if matches!(name, "p" | "div" | "h1" | "h2" | "h3" | "h4" | "li" | "tr")
                    && (closing || name == "li")
                {
                    if !out.ends_with('\n') {
                        out.push('\n');
                    }
                    out.push('\n');
                }
                rest = &after[end + 1..];
                continue;
            }
        }
        if let Some(after) = rest.strip_prefix('&') {
            if let Some(end) = after.find(';').filter(|n| *n <= 12) {
                out.push_str(&decode_entity(&after[..end]));
                rest = &after[end + 1..];
                continue;
            }
        }
        let c = rest.chars().next().unwrap();
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out.split("\n\n")
        .map(|p| {
            p.lines()
                .map(|l| {
                    // Keep tabs between table cells; collapse other runs of whitespace.
                    l.trim_end_matches('\t')
                        .split('\t')
                        .map(|cell| cell.split_whitespace().collect::<Vec<_>>().join(" "))
                        .collect::<Vec<_>>()
                        .join("\t")
                })
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn decode_entity(e: &str) -> String {
    match e {
        "amp" => "&".into(),
        "lt" => "<".into(),
        "gt" => ">".into(),
        "quot" => "\"".into(),
        "nbsp" => " ".into(),
        "apos" => "'".into(),
        e if e.starts_with("#x") || e.starts_with("#X") => u32::from_str_radix(&e[2..], 16)
            .ok()
            .and_then(char::from_u32)
            .map(|c| c.to_string())
            .unwrap_or_else(|| format!("&{e};")),
        e if e.starts_with('#') => e[1..]
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .map(|c| c.to_string())
            .unwrap_or_else(|| format!("&{e};")),
        _ => format!("&{e};"),
    }
}

pub fn html_to_markdown(html: &str) -> String {
    // Very small HTML→MD: headings, bold, italic, lists, links, paragraphs
    let mut s = html.to_string();
    s = s
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n");
    s = replace_tag_content(&s, "h1", "# ", "\n\n");
    s = replace_tag_content(&s, "h2", "## ", "\n\n");
    s = replace_tag_content(&s, "h3", "### ", "\n\n");
    s = replace_tag_content(&s, "h4", "#### ", "\n\n");
    s = replace_tag_content(&s, "h5", "##### ", "\n\n");
    s = replace_tag_content(&s, "h6", "###### ", "\n\n");
    s = replace_tag_content(&s, "strong", "**", "**");
    s = replace_tag_content(&s, "b", "**", "**");
    s = replace_tag_content(&s, "em", "*", "*");
    s = replace_tag_content(&s, "i", "*", "*");
    s = replace_tag_content(&s, "code", "`", "`");
    s = number_ordered_lists(&s);
    s = replace_tag_content(&s, "li", "- ", "\n");
    s = s.replace("</td>", " | ").replace("</th>", " | ").replace("</TD>", " | ").replace("</TH>", " | ");
    s = s.replace("</tr>", "\n").replace("</TR>", "\n");
    s = replace_links(&s);
    s = s.replace("</p>", "\n\n").replace("</div>", "\n\n");
    s = strip_tags(&s);
    html_to_plain(&s.chars().map(|c| c).collect::<String>())
        .lines()
        .map(|l| l.trim().trim_end_matches(" |").trim_end_matches('|').trim_end())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn replace_tag_content(html: &str, tag: &str, before: &str, after: &str) -> String {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    // ASCII case folding keeps byte offsets aligned for Unicode document text.
    let lower = html.to_ascii_lowercase();
    let mut out = String::new();
    let mut i = 0;
    let bytes = html.as_bytes();
    let lower_b = lower.as_bytes();
    while i < bytes.len() {
        if let Some(rel) = find_tag(&lower_b[i..], open.as_bytes()) {
            let start = i + rel;
            out.push_str(&html[i..start]);
            // find end of open tag
            let after_open = match html[start..].find('>') {
                Some(p) => start + p + 1,
                None => {
                    out.push_str(&html[start..]);
                    break;
                }
            };
            if let Some(rel2) = find_subslice(&lower_b[after_open..], close.as_bytes()) {
                let content_end = after_open + rel2;
                let content = &html[after_open..content_end];
                out.push_str(before);
                out.push_str(content);
                out.push_str(after);
                i = content_end + close.len();
                continue;
            } else {
                out.push_str(&html[start..after_open]);
                i = after_open;
                continue;
            }
        }
        out.push_str(&html[i..]);
        break;
    }
    out
}

fn replace_links(html: &str) -> String {
    let mut result = String::new();
    let mut rest = html;
    while let Some(start) = rest.to_ascii_lowercase().find("<a ") {
        result.push_str(&rest[..start]);
        let anchor = &rest[start..];
        let Some(open_end) = anchor.find('>') else {
            result.push_str(anchor);
            return result;
        };
        let Some(close_start) = anchor[open_end + 1..].to_ascii_lowercase().find("</a>") else {
            result.push_str(anchor);
            return result;
        };
        let label = &anchor[open_end + 1..open_end + 1 + close_start];
        let opening = &anchor[..open_end + 1];
        let href = opening.split_whitespace().find_map(|part| {
            let value = part.strip_prefix("href=")?;
            Some(value.trim_matches(['"', '\'', '>']).to_string())
        });
        if let Some(url) = href.filter(|url| {
            (url.starts_with("https://")
                || url.starts_with("http://")
                || url.starts_with("mailto:")
                || url.starts_with('#'))
                && !url.contains('<')
                && !url.contains('>')
        }) {
            result.push_str(&format!("[{label}]({})", url.replace(')', "\\)")));
        } else {
            result.push_str(label);
        }
        rest = &anchor[open_end + 1 + close_start + 4..];
    }
    result.push_str(rest);
    result
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Find an opening tag such as `<b`, skipping longer names that share the prefix
/// (`<blockquote>`, `<br>`, `<img>` for `<i`, `<embed>` for `<em`, …).
fn find_tag(hay: &[u8], open: &[u8]) -> Option<usize> {
    let mut from = 0;
    while let Some(rel) = find_subslice(&hay[from..], open) {
        let at = from + rel;
        match hay.get(at + open.len()) {
            Some(b'>' | b'/') | None => return Some(at),
            Some(c) if c.is_ascii_whitespace() => return Some(at),
            _ => from = at + 1,
        }
    }
    None
}

/// Number the items of each `<ol>` so ordered lists survive as `1.`, `2.`, …
fn number_ordered_lists(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut out = String::new();
    let mut i = 0;
    while let Some(rel) = find_tag(&lower.as_bytes()[i..], b"<ol") {
        let start = i + rel;
        let Some(end_rel) = lower[start..].find("</ol>") else {
            break;
        };
        let end = start + end_rel;
        out.push_str(&html[i..start]);
        let block = &html[start..end];
        let block_lower = &lower[start..end];
        let mut j = 0;
        let mut n = 0;
        while let Some(li_rel) = find_tag(&block_lower.as_bytes()[j..], b"<li") {
            let li = j + li_rel;
            let Some(gt) = block[li..].find('>') else {
                break;
            };
            n += 1;
            out.push_str(&block[j..li]);
            out.push_str(&format!("{n}. "));
            j = li + gt + 1;
            if let Some(close) = block_lower[j..].find("</li>") {
                out.push_str(&block[j..j + close]);
                out.push('\n');
                j += close + "</li>".len();
            }
        }
        out.push_str(&block[j..]);
        i = end + "</ol>".len();
    }
    out.push_str(&html[i..]);
    out
}

fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

fn build_docx(_title: &str, plain: &str) -> Result<Vec<u8>, String> {
    let mut buf = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut buf);
        let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("[Content_Types].xml", opts)
            .map_err(|e| e.to_string())?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
        )
        .map_err(|e| e.to_string())?;

        zip.start_file("_rels/.rels", opts)
            .map_err(|e| e.to_string())?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
        )
        .map_err(|e| e.to_string())?;

        zip.start_file("word/document.xml", opts)
            .map_err(|e| e.to_string())?;
        let mut body = String::new();
        body.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
        body.push_str(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>"#,
        );
        for para in plain.split("\n\n") {
            body.push_str("<w:p>");
            for (i, line) in para.split('\n').enumerate() {
                if i > 0 {
                    body.push_str("<w:r><w:br/></w:r>");
                }
                body.push_str("<w:r><w:t xml:space=\"preserve\">");
                body.push_str(&xml_escape(line));
                body.push_str("</w:t></w:r>");
            }
            body.push_str("</w:p>");
        }
        body.push_str(r#"<w:sectPr/></w:body></w:document>"#);
        zip.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
        zip.finish().map_err(|e| e.to_string())?;
    }
    Ok(buf.into_inner())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Minimal multi-line text PDF (Type1 Helvetica-like via built-in PDF fonts).
fn build_simple_pdf(title: &str, plain: &str) -> Result<Vec<u8>, String> {
    let mut lines: Vec<String> = Vec::new();
    lines.push(title.to_string());
    lines.push(String::new());
    for para in plain.split("\n\n") {
        // wrap ~90 chars
        let words: Vec<&str> = para.split_whitespace().collect();
        let mut cur = String::new();
        for w in words {
            if cur.len() + w.len() + 1 > 90 {
                lines.push(cur);
                cur = w.to_string();
            } else {
                if !cur.is_empty() {
                    cur.push(' ');
                }
                cur.push_str(w);
            }
        }
        if !cur.is_empty() {
            lines.push(cur);
        }
        lines.push(String::new());
    }
    // A letter page holds 49 lines at 14 pt leading with the chosen margins.
    // Keep every paragraph rather than silently truncating long documents.
    let pages: Vec<&[String]> = lines.chunks(49).collect();
    let font_id = 3 + pages.len() * 2;
    let mut pdf = Vec::new();
    let mut offsets = Vec::new();

    fn write_obj(pdf: &mut Vec<u8>, offsets: &mut Vec<usize>, n: usize, body: &[u8]) {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{n} 0 obj\n").as_bytes());
        pdf.extend_from_slice(body);
        if !body.ends_with(b"\n") {
            pdf.push(b'\n');
        }
        pdf.extend_from_slice(b"endobj\n");
    }

    pdf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    write_obj(
        &mut pdf,
        &mut offsets,
        1,
        b"<< /Type /Catalog /Pages 2 0 R >>",
    );
    let kids = (0..pages.len())
        .map(|i| format!("{} 0 R", 3 + 2 * i))
        .collect::<Vec<_>>()
        .join(" ");
    write_obj(
        &mut pdf,
        &mut offsets,
        2,
        format!("<< /Type /Pages /Kids [{kids}] /Count {} >>", pages.len()).as_bytes(),
    );
    for (i, page_lines) in pages.iter().enumerate() {
        let page_id = 3 + i * 2;
        let content_id = page_id + 1;
        write_obj(&mut pdf, &mut offsets, page_id,
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {content_id} 0 R /Resources << /Font << /F1 {font_id} 0 R >> >> >>").as_bytes());
        let mut content = b"BT\n/F1 12 Tf\n14 TL\n50 758 Td\n".to_vec();
        for (line_no, line) in page_lines.iter().enumerate() {
            if line_no > 0 {
                content.extend_from_slice(b"T*\n");
            }
            content.push(b'(');
            content.extend_from_slice(&pdf_escape(line));
            content.extend_from_slice(b") Tj\n");
        }
        content.extend_from_slice(b"ET");
        let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
        stream.extend_from_slice(&content);
        stream.extend_from_slice(b"\nendstream");
        write_obj(&mut pdf, &mut offsets, content_id, &stream);
    }
    write_obj(
        &mut pdf,
        &mut offsets,
        font_id,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );

    let xref_pos = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len() + 1,
            xref_pos
        )
        .as_bytes(),
    );
    Ok(pdf)
}

fn pdf_escape(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for c in s.chars() {
        match c {
            '\\' | '(' | ')' => {
                out.push(b'\\');
                out.push(c as u8);
            }
            '\t' | '\n' | '\r' => out.push(b' '),
            c if c.is_ascii() && !c.is_control() => out.push(c as u8),
            c if (0xa0..=0xff).contains(&(c as u32)) => out.push(c as u8),
            '€' => out.push(0x80),
            '…' => out.push(0x85),
            '‘' => out.push(0x91),
            '’' => out.push(0x92),
            '“' => out.push(0x93),
            '”' => out.push(0x94),
            '•' => out.push(0x95),
            '–' => out.push(0x96),
            '—' => out.push(0x97),
            _ => out.push(b'?'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_strips_tags() {
        let p = html_to_plain("<p>Hi <b>there</b></p>");
        assert_eq!(p, "Hi there");
        assert_eq!(
            html_to_plain("<p>A &amp; B<br>C</p><p>D&#233;</p>"),
            "A & B\nC\n\nDé"
        );
    }

    #[test]
    fn markdown_keeps_paragraphs_and_unicode() {
        assert_eq!(
            html_to_markdown("<p>Résumé <strong>bold</strong></p><p>Next</p>"),
            "Résumé **bold**\n\nNext"
        );
        assert_eq!(
            html_to_markdown("<p><a href=\"https://example.com\">Link</a></p>"),
            "[Link](https://example.com)"
        );
    }

    #[test]
    fn markdown_does_not_confuse_tags_sharing_a_prefix() {
        assert_eq!(
            html_to_markdown("<blockquote><p>quote</p></blockquote><p>then <b>bold</b></p>"),
            "quote\n\nthen **bold**"
        );
        assert_eq!(
            html_to_markdown("<p><img src=\"x.png\"></p><p>then <i>it</i></p>"),
            "then *it*"
        );
    }

    #[test]
    fn markdown_numbers_ordered_lists_and_separates_table_cells() {
        assert_eq!(
            html_to_markdown("<ol><li>first</li><li>second</li></ol><ul><li>dot</li></ul>"),
            "1. first\n\n2. second\n\n- dot"
        );
        assert_eq!(
            html_to_markdown("<table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr></table>"),
            "A | B\n\n1 | 2"
        );
        assert_eq!(
            html_to_plain("<table><tr><td>A</td><td>B c</td></tr><tr><td>1</td><td>2</td></tr></table>"),
            "A\tB c\n\n1\t2"
        );
    }

    #[test]
    fn docx_round_trip_keeps_text_without_extra_title() {
        let bytes = build_docx("File title", "A & B\n\nDéjà vu").unwrap();
        let opened =
            crate::files::open_bytes("file.docx", "file.docx", "docx", "File title", &bytes)
                .unwrap();
        assert!(opened.html.contains("A &amp; B"));
        assert!(opened.html.contains("Déjà vu"));
        assert!(!opened.html.contains("File title</p>"));
    }

    #[test]
    fn pdf_builds() {
        let b = build_simple_pdf("T", "Café € —").unwrap();
        assert!(b.starts_with(b"%PDF"));
        assert!(b.windows(3).any(|w| w == [0xe9, b' ', 0x80]));
    }

    #[test]
    fn pdf_keeps_long_documents() {
        let text = (0..150)
            .map(|i| format!("Unique line {i}"))
            .collect::<Vec<_>>()
            .join("\n\n");
        let pdf = build_simple_pdf("Long", &text).unwrap();
        assert!(String::from_utf8_lossy(&pdf).contains("Unique line 149"));
        assert!(String::from_utf8_lossy(&pdf).contains("/Count 7"));
    }
}
