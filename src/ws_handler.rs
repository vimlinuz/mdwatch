use actix_web::{HttpRequest, Responder, web};
use colored::Colorize;
use notify::RecursiveMode;
use notify::event::ModifyKind;
use notify::event::RemoveKind;
use notify_debouncer_full::DebouncedEvent;
use notify_debouncer_full::{DebounceEventResult, new_debouncer, notify::*};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use super::FileInfo;
use super::get_markdown;

pub async fn ws_handler(
    req: HttpRequest,
    body: web::Payload,
    file_info: web::Data<FileInfo>,
) -> actix_web::Result<impl Responder> {
    let (response, mut session, mut _msg_stream) = actix_ws::handle(&req, body)?;
    let file = file_info.file.to_path_buf();

    let file_name = match file.file_name() {
        Some(name) => name.to_os_string(),
        None => {
            return Err(actix_web::error::ErrorInternalServerError(
                "Failed to get file name",
            ));
        }
    };

    let base_dir = file_info.base_dir.to_path_buf();
    let (watch_tx, mut notify_rx) = mpsc::unbounded_channel::<DebouncedEvent>();

    let mut debouncer = new_debouncer(
        Duration::from_millis(200),
        None,
        move |result: DebounceEventResult| match result {
            Ok(events) => events.into_iter().for_each(|event| {
                let _ = watch_tx.send(event);
            }),
            Err(errors) => errors.iter().for_each(|error| {
                eprintln!(
                    "{} Watch error: {error:?}",
                    "Error:".red().bold()
                )
            }),
        },
    )
    .map_err(actix_web::error::ErrorInternalServerError)?;

    debouncer
        .watch(&base_dir, RecursiveMode::Recursive)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    actix_web::rt::spawn(async move {
        // Keep the watcher alive in this async task to keep the msg_stream alive
        let _watcher = debouncer;

        // here we initially set last_sent to 1 second ago to allow the first update to be sent immediately
        let mut last_sent = Instant::now() - Duration::from_secs(1);

        while let Some(event) = notify_rx.recv().await {
            let is_selected_file = event.paths.iter().any(|p| {
                p.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name == file_name)
                    .unwrap_or(false)
            });

            if is_selected_file {
                if matches!(event.kind, EventKind::Remove(RemoveKind::File)) {
                    eprintln!(
                        "{} File removed: {}",
                        "Warning:".yellow().bold(),
                        file.display()
                    );
                    break;
                }
                let modified_selected_file =
                    matches!(event.kind, EventKind::Modify(ModifyKind::Name(_)))
                        || matches!(event.kind, EventKind::Modify(ModifyKind::Data(_)));

                if modified_selected_file && last_sent.elapsed() >= Duration::from_secs(1) {
                    let latest_markdown = match get_markdown(&file).await {
                        Ok(md) => md,
                        Err(e) => {
                            eprintln!(
                                "{} Error reading markdown file: {e}",
                                "Error:".red().bold()
                            );
                            continue;
                        }
                    };
                    last_sent = Instant::now();
                    if session.text(latest_markdown).await.is_err() {
                        break;
                    }
                }
            }
        }

        let _ = session.close(None).await;
    });

    Ok(response)
}
