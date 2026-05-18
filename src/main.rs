mod args;
mod test;
pub mod utils;
mod ws_handler;

use actix_files::NamedFile;
use actix_web::App;
use actix_web::HttpResponse;
use actix_web::HttpServer;
use actix_web::get;
use actix_web::web;
use args::MdwatchArgs;
use askama::Template;
use clap::Parser;
use std::path::PathBuf;

use utils::{get_embedded_file, get_local_ip, get_markdown, get_random_port};
use ws_handler::ws_handler;

#[derive(Template)]
#[template(path = "main.html")]
struct Mdwatch {
    content: String,
    title: String,
}

#[get("/")]
async fn home(file_info: web::Data<FileInfo>) -> actix_web::Result<HttpResponse> {
    let file = &file_info.file;
    let file_name = match file.file_name() {
        Some(name) => name,
        None => {
            return Err(actix_web::error::ErrorInternalServerError(
                "Failed to get file name",
            ));
        }
    };

    if let Some(extension) = file.extension()
        && extension != "md"
    {
        eprintln!(
            "Warning: Unsupported file type: .{}",
            extension.to_string_lossy()
        );
        return Err(actix_web::error::ErrorInternalServerError(
            "Unsupported file type. Please provide a markdown (.md) file.",
        ));
    };

    let html_output = match get_markdown(&file.to_path_buf()).await {
        Ok(html) => html,
        Err(e) => {
            eprintln!("Error processing markdown file: {e}");
            return Err(actix_web::error::ErrorInternalServerError(
                "Failed to process markdown file",
            ));
        }
    };

    let template = Mdwatch {
        content: html_output,
        title: file_name.to_string_lossy().to_string(),
    };

    match template.render() {
        Ok(rendered) => Ok(HttpResponse::Ok().content_type("text/html").body(rendered)),
        Err(e) => {
            eprintln!("Template rendering error: {e}");

            Ok(HttpResponse::InternalServerError()
                .content_type("text/plain")
                .body("Failed to render template"))
        }
    }
}

/// Serve local image files referenced in the markdown.
/// Resolves the requested path relative to the markdown file's parent directory.
#[get("/_local_image/{path:.*}")]
async fn serve_local_image(
    path: web::Path<PathBuf>,
    file_info: web::Data<FileInfo>,
) -> actix_web::Result<NamedFile> {
    let requested = path.into_inner();
    let base_dir = &file_info.base_dir;
    let resolved = base_dir.join(&requested);

    // Canonicalize to prevent directory traversal attacks (e.g. ../../etc/passwd)
    let canonical = resolved
        .canonicalize()
        .map_err(|_| actix_web::error::ErrorNotFound("Image not found"))?;

    let base_canonical = base_dir
        .canonicalize()
        .map_err(|_| actix_web::error::ErrorInternalServerError("Invalid base directory"))?;

    if !canonical.starts_with(&base_canonical) {
        return Err(actix_web::error::ErrorForbidden(
            "Access denied: path outside base directory",
        ));
    }

    Ok(NamedFile::open(canonical)?)
}

#[get("/libs/{lib}")]
async fn serve_libs(lib: web::Path<String>) -> actix_web::Result<HttpResponse> {
    let (content, content_type) = match lib.as_str() {
        "hljs-theme-dark" => (
            get_embedded_file("static/lib/github-dark.min.css"),
            "text/css",
        ),
        "hljs-theme-light" => (
            get_embedded_file("static/lib/github-light.min.css"),
            "text/css",
        ),
        "style" => (get_embedded_file("static/global.css"), "text/css"),
        "mermaid-script" => (
            get_embedded_file("static/lib/mermaid.min.js"),
            "application/javascript",
        ),
        "hljs-script" => (
            get_embedded_file("static/lib/highlight.min.js"),
            "application/javascript",
        ),
        "client" => (
            get_embedded_file("static/client.js"),
            "application/javascript",
        ),
        _ => {
            return Err(actix_web::error::ErrorNotFound("lib not found"));
        }
    };
    Ok(HttpResponse::Ok().content_type(content_type).body(content))
}

#[derive(Clone)]
pub struct FileInfo {
    file: PathBuf,
    base_dir: PathBuf,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = MdwatchArgs::parse();

    let file = args.file;
    let mut ip = args.ip;
    let port = args.port.unwrap_or_else(get_random_port);

    // Resolve the parent directory of the markdown file for serving local images
    let base_dir: PathBuf = file
        .parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|| PathBuf::from("."));

    let file_info = FileInfo { file, base_dir };

    if ip == "0.0.0.0" {
        eprintln!("  Warning: Binding to 0.0.0.0 exposes your server to the entire network!");
        eprintln!("         Make sure you trust your network or firewall settings.");
        ip = get_local_ip().unwrap_or(String::from("0.0.0.0"));
    }

    println!("Server running at:");
    println!(" - http://{}:{}/", ip, port);

    match HttpServer::new(move || {
        App::new()
            .route("/ws", web::get().to(ws_handler))
            .service(home)
            .service(serve_local_image)
            .service(serve_libs)
            .app_data(web::Data::new(file_info.clone()))
    })
    .bind(format!("{}:{}", ip, port))
    {
        Ok(server) => {
            if let Err(e) = webbrowser::open(&format!("http://localhost:{}/", port)) {
                eprintln!("Failed to open browser: {e}");
            }
            server.run().await
        }
        Err(e) => {
            eprintln!("Failed to start server: {e}");
            std::process::exit(1);
        }
    }
}
