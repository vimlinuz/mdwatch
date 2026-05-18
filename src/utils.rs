use ammonia::Builder;
use ammonia::UrlRelative::PassThrough;
use colored::Colorize;
use pulldown_cmark::Options;
use regex::Regex;
use rust_embed::Embed;
use std::net::UdpSocket;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

pub fn get_random_port() -> u16 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH);
    match now {
        Ok(duration) => {
            let millis = duration.as_millis() as u16;
            8080 + (millis % 1000)
        }
        Err(_) => 8080,
    }
}

pub fn get_local_ip() -> Option<String> {
    // os will choose an available ephemeral port.
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    Some(ip.to_string())
}

#[derive(Embed)]
#[folder = "$CARGO_MANIFEST_DIR/static"]
#[prefix = "static/"]
struct Static;

pub fn get_embedded_file(file_path: &str) -> String {
    match Static::get(file_path) {
        Some(file) => match std::str::from_utf8(&file.data) {
            Ok(content) => content.to_string(),
            Err(e) => {
                eprintln!(
                    "{} Failed to read embedded file: {e}",
                    "Error:".red().bold()
                );
                String::new()
            }
        },
        None => {
            eprintln!(
                "{} File not found in embedded files.",
                "Error:".red().bold()
            );
            String::new()
        }
    }
}

/// Rewrite local image `src` attributes to use the `/_local_image/` prefix.
/// Remote images (http://, https://, //, data:) are left untouched.
pub fn rewrite_image_paths(html: &str) -> String {
    let re = Regex::new(r#"(<img\s[^>]*?src\s*=\s*")([^"]*?)(")"#).expect("invalid regex");
    re.replace_all(html, |caps: &regex::Captures| {
        let prefix = &caps[1];
        let src = &caps[2];
        let suffix = &caps[3];
        // Skip remote URLs and data URIs
        if src.starts_with("http://")
            || src.starts_with("https://")
            || src.starts_with("//")
            || src.starts_with("data:")
        {
            format!("{}{}{}", prefix, src, suffix)
        } else {
            format!("{}/_local_image/{}{}", prefix, src, suffix)
        }
    })
    .to_string()
}

/// Sanitize HTML while preserving relative URLs (needed for /_local_image/ paths).
pub fn sanitize_html(html: &str) -> String {
    Builder::default()
        .url_relative(PassThrough)
        .add_generic_attributes(&["align"])
        .add_tag_attributes("code", &["class"])
        .clean(html)
        .to_string()
}

pub fn rewrite_mermaid_tags(html: &str) -> String {
    let re = Regex::new(r#"<pre><code class="language-mermaid">([\s\S]*?)</code></pre>"#)
        .expect("invalid regex");
    let src = r#"<pre class="mermaid">$1</pre>"#;
    re.replace_all(html, src).to_string()
}

pub async fn get_markdown(file_path: &PathBuf) -> std::io::Result<String> {
    let markdown_input: String = fs::read_to_string(file_path).await?;
    let options = Options::all();
    let parser = pulldown_cmark::Parser::new_ext(&markdown_input, options);

    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);
    html_output = rewrite_image_paths(&html_output);
    html_output = sanitize_html(&html_output);
    html_output = rewrite_mermaid_tags(&html_output);
    Ok(html_output)
}
