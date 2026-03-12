//! Content Extractor — converts HTML to clean text and markdown
//! Strips navigation, ads, boilerplate — keeps the article content.

use scraper::{Html, Selector};

/// Extracted content from a web page
#[derive(Debug, Clone)]
pub struct ExtractedContent {
    pub title: String,
    pub clean_text: String,
    pub markdown: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub headings: Vec<String>,
}

/// Extracts clean content from HTML pages
pub struct ContentExtractor;

impl ContentExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Extract clean content from HTML
    pub fn extract(&self, html: &str, _url: &str) -> ExtractedContent {
        let document = Html::parse_document(html);

        let title = self.extract_title(&document);
        let author = self.extract_meta(&document, "author");
        let description = self
            .extract_meta(&document, "description")
            .or_else(|| self.extract_meta_property(&document, "og:description"));

        let headings = self.extract_headings(&document);
        let clean_text = self.extract_main_content(&document);
        let markdown = self.html_to_markdown(&document);

        ExtractedContent {
            title,
            clean_text,
            markdown,
            author,
            description,
            headings,
        }
    }

    /// Extract all links from HTML
    pub fn extract_links(&self, html: &str, base_url: &str) -> Vec<String> {
        let document = Html::parse_document(html);
        let selector = Selector::parse("a[href]").unwrap();
        let base = url::Url::parse(base_url).ok();

        document
            .select(&selector)
            .filter_map(|el| {
                el.value().attr("href").and_then(|href| {
                    if href.starts_with('#') || href.starts_with("javascript:") || href.starts_with("mailto:") {
                        return None;
                    }

                    if href.starts_with("http://") || href.starts_with("https://") {
                        Some(href.to_string())
                    } else if let Some(ref base) = base {
                        base.join(href).ok().map(|u| u.to_string())
                    } else {
                        None
                    }
                })
            })
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    fn extract_title(&self, doc: &Html) -> String {
        // Try <title> tag first
        if let Ok(sel) = Selector::parse("title") {
            if let Some(el) = doc.select(&sel).next() {
                let title = el.text().collect::<String>().trim().to_string();
                if !title.is_empty() {
                    return title;
                }
            }
        }

        // Try <h1>
        if let Ok(sel) = Selector::parse("h1") {
            if let Some(el) = doc.select(&sel).next() {
                let title = el.text().collect::<String>().trim().to_string();
                if !title.is_empty() {
                    return title;
                }
            }
        }

        // Try og:title
        self.extract_meta_property(doc, "og:title")
            .unwrap_or_else(|| "Untitled".to_string())
    }

    fn extract_meta(&self, doc: &Html, name: &str) -> Option<String> {
        let selector_str = format!(r#"meta[name="{}"]"#, name);
        if let Ok(sel) = Selector::parse(&selector_str) {
            doc.select(&sel)
                .next()
                .and_then(|el| el.value().attr("content"))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        } else {
            None
        }
    }

    fn extract_meta_property(&self, doc: &Html, property: &str) -> Option<String> {
        let selector_str = format!(r#"meta[property="{}"]"#, property);
        if let Ok(sel) = Selector::parse(&selector_str) {
            doc.select(&sel)
                .next()
                .and_then(|el| el.value().attr("content"))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        } else {
            None
        }
    }

    fn extract_headings(&self, doc: &Html) -> Vec<String> {
        let mut headings = Vec::new();
        for tag in &["h1", "h2", "h3"] {
            if let Ok(sel) = Selector::parse(tag) {
                for el in doc.select(&sel) {
                    let text = el.text().collect::<String>().trim().to_string();
                    if !text.is_empty() {
                        headings.push(text);
                    }
                }
            }
        }
        headings
    }

    /// Extract main content — strips navigation, footer, sidebar, ads
    fn extract_main_content(&self, doc: &Html) -> String {
        // Try to find main content containers first
        let content_selectors = [
            "article",
            "main",
            r#"[role="main"]"#,
            ".post-content",
            ".article-content",
            ".entry-content",
            ".content",
            "#content",
        ];

        for selector_str in &content_selectors {
            if let Ok(sel) = Selector::parse(selector_str) {
                if let Some(el) = doc.select(&sel).next() {
                    let text = el.text().collect::<Vec<_>>().join(" ");
                    let cleaned = self.clean_text(&text);
                    if cleaned.len() > 100 {
                        return cleaned;
                    }
                }
            }
        }

        // Fallback: extract all paragraph text
        if let Ok(sel) = Selector::parse("p") {
            let paragraphs: Vec<String> = doc
                .select(&sel)
                .map(|el| el.text().collect::<String>().trim().to_string())
                .filter(|t| t.len() > 20) // Skip tiny fragments
                .collect();

            if !paragraphs.is_empty() {
                return paragraphs.join("\n\n");
            }
        }

        // Last resort: body text
        if let Ok(sel) = Selector::parse("body") {
            if let Some(body) = doc.select(&sel).next() {
                return self.clean_text(&body.text().collect::<Vec<_>>().join(" "));
            }
        }

        String::new()
    }

    /// Convert HTML to basic markdown
    fn html_to_markdown(&self, doc: &Html) -> String {
        let mut md = String::new();

        // Headings
        for (tag, prefix) in [("h1", "# "), ("h2", "## "), ("h3", "### ")] {
            if let Ok(sel) = Selector::parse(tag) {
                for el in doc.select(&sel) {
                    let text = el.text().collect::<String>().trim().to_string();
                    if !text.is_empty() {
                        md.push_str(&format!("{}{}\n\n", prefix, text));
                    }
                }
            }
        }

        // Paragraphs
        if let Ok(sel) = Selector::parse("p") {
            for el in doc.select(&sel) {
                let text = el.text().collect::<String>().trim().to_string();
                if text.len() > 20 {
                    md.push_str(&format!("{}\n\n", text));
                }
            }
        }

        // Lists
        if let Ok(sel) = Selector::parse("li") {
            for el in doc.select(&sel) {
                let text = el.text().collect::<String>().trim().to_string();
                if !text.is_empty() {
                    md.push_str(&format!("- {}\n", text));
                }
            }
        }

        md
    }

    /// Clean extracted text — normalize whitespace, remove junk
    fn clean_text(&self, text: &str) -> String {
        // Replace multiple whitespace with single space
        let re_whitespace = regex::Regex::new(r"\s+").unwrap();
        let cleaned = re_whitespace.replace_all(text, " ");

        // Split into lines and clean
        cleaned
            .split('\n')
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    }
}

impl Default for ContentExtractor {
    fn default() -> Self {
        Self::new()
    }
}
