use std::time::Duration;
use tokio::fs;

use anyhow::anyhow;
use futures_util::StreamExt;
use revaer_events::EventEnvelope;
use uuid::Uuid;

use crate::cli::TailArgs;
use crate::client::{AppContext, CliError, CliResult, HEADER_LAST_EVENT_ID, classify_problem};

pub(crate) async fn handle_tail(ctx: &AppContext, args: TailArgs) -> CliResult<()> {
    let mut resume_id = match &args.resume_file {
        Some(path) => match fs::read_to_string(path).await {
            Ok(value) => Some(value.trim().parse::<u64>().map_err(|err| {
                CliError::validation(format!(
                    "invalid resume event ID in {}: {err}",
                    path.display()
                ))
            })?),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
            Err(err) => {
                return Err(CliError::failure(anyhow!(
                    "failed to read resume file '{}': {err}",
                    path.display()
                )));
            }
        },
        None => None,
    };

    loop {
        let mut url = ctx
            .base_url
            .join("/v1/torrents/events")
            .map_err(|err| CliError::failure(anyhow!("invalid base URL: {err}")))?;

        {
            let mut pairs = url.query_pairs_mut();
            if !args.torrent.is_empty() {
                let value = args
                    .torrent
                    .iter()
                    .map(Uuid::to_string)
                    .collect::<Vec<_>>()
                    .join(",");
                pairs.append_pair("torrent", &value);
            }
            if !args.event.is_empty() {
                let value = args.event.join(",");
                pairs.append_pair("event", &value);
            }
            if !args.state.is_empty() {
                let value = args.state.join(",");
                pairs.append_pair("state", &value);
            }
        }

        let builder = ctx.client.get(url);
        let builder = if let Some(id) = resume_id {
            builder.header(HEADER_LAST_EVENT_ID, id.to_string())
        } else {
            builder
        };

        let response = match builder.send().await {
            Ok(resp) => resp,
            Err(err) => {
                eprintln!(
                    "stream connection failed: {err:?}. retrying in {}s",
                    args.retry_secs
                );
                tokio::time::sleep(Duration::from_secs(args.retry_secs)).await;
                continue;
            }
        };

        if !response.status().is_success() {
            return Err(classify_problem(response).await);
        }

        match stream_events(response, &args, resume_id.as_mut()).await {
            Ok(last_id) => resume_id = last_id,
            Err(err) => {
                eprintln!("stream error: {err:?}. retrying in {}s", args.retry_secs);
                tokio::time::sleep(Duration::from_secs(args.retry_secs)).await;
            }
        }
    }
}

pub(crate) async fn stream_events(
    response: reqwest::Response,
    args: &TailArgs,
    mut resume_slot: Option<&mut u64>,
) -> CliResult<Option<u64>> {
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut current_event_id: Option<u64> = None;
    let mut current_data = Vec::new();
    let mut last_seen = resume_slot.as_ref().map(|slot| **slot);

    while let Some(chunk) = stream.next().await {
        let chunk = chunk
            .map_err(|err| CliError::failure(anyhow!("failed to read event stream: {err}")))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim_end_matches('\r').to_string();
            buffer.drain(..=pos);
            if line.is_empty() {
                if current_data.is_empty() {
                    current_event_id = None;
                    continue;
                }
                let payload = current_data.join("\n");
                current_data.clear();
                if let Some(id) = current_event_id.take() {
                    if Some(id) == last_seen {
                        continue;
                    }
                    last_seen = Some(id);
                    if let Some(slot) = resume_slot.as_mut() {
                        **slot = id;
                    }
                    if let Some(path) = &args.resume_file {
                        fs::write(path, id.to_string())
                            .await
                            .map_err(CliError::failure)?;
                    }
                }
                print_event_payload(&payload)?;
            } else if let Some(data) = line.strip_prefix("data:") {
                current_data.push(data.trim_start().to_string());
            } else if let Some(id) = line.strip_prefix("id:")
                && let Ok(value) = id.trim_start().parse::<u64>()
            {
                current_event_id = Some(value);
            }
            // Auxiliary and unrecognized fields do not alter event state.
        }
    }

    Ok(last_seen)
}

fn print_event_payload(payload: &str) -> CliResult<()> {
    match serde_json::from_str::<EventEnvelope>(payload) {
        Ok(event) => {
            let text = serde_json::to_string_pretty(&event)
                .map_err(|err| CliError::failure(anyhow!("failed to format event JSON: {err}")))?;
            println!("{text}");
        }
        Err(err) => {
            eprintln!("discarding malformed event payload: {err} -- {payload}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::HEADER_REQUEST_ID;
    use anyhow::Result;
    use chrono::Utc;
    use httpmock::MockServer;
    use httpmock::prelude::*;
    use reqwest::Client;
    use revaer_events::{Event, EventEnvelope};
    use tokio::fs;
    use uuid::Uuid;

    fn build_envelope(id: u64) -> EventEnvelope {
        EventEnvelope {
            id,
            timestamp: Utc::now(),
            event: Event::TorrentAdded {
                torrent_id: Uuid::new_v4(),
                name: "demo".to_string(),
            },
        }
    }

    fn sse_payload(id: u64, envelope: &EventEnvelope) -> String {
        let payload = serde_json::to_string(envelope).expect("serialize event");
        format!("event: update\nretry: 1000\n{HEADER_REQUEST_ID}: req\nid:{id}\ndata:{payload}\n\n")
    }

    fn tail_args(resume_file: Option<std::path::PathBuf>) -> TailArgs {
        TailArgs {
            torrent: Vec::new(),
            event: Vec::new(),
            state: Vec::new(),
            resume_file,
            retry_secs: 0,
        }
    }

    #[tokio::test]
    async fn handle_tail_reports_invalid_resume_files_before_connecting() -> Result<()> {
        let directory =
            std::env::temp_dir().join(format!("revaer-tail-invalid-{}", Uuid::new_v4()));
        fs::create_dir(&directory).await?;
        let path = directory.join("resume");
        fs::write(&path, "invalid-event-id").await?;
        let context = AppContext {
            client: Client::new(),
            base_url: reqwest::Url::parse("http://127.0.0.1:1")?,
            api_key: None,
        };
        let invalid = handle_tail(&context, tail_args(Some(path))).await;
        assert!(invalid.is_err());
        if let Err(error) = invalid {
            assert!(error.display_message().contains("invalid resume event ID"));
        }
        let unreadable = handle_tail(&context, tail_args(Some(directory.clone()))).await;
        assert!(unreadable.is_err());
        if let Err(error) = unreadable {
            assert!(
                error
                    .display_message()
                    .contains("failed to read resume file")
            );
        }
        fs::remove_dir_all(directory).await?;
        Ok(())
    }

    #[tokio::test]
    async fn stream_events_updates_resume_and_file() -> Result<()> {
        let server = MockServer::start_async().await;
        let envelope1 = build_envelope(1);
        let envelope2 = build_envelope(2);
        let body = format!(
            "{}{}",
            sse_payload(1, &envelope1),
            sse_payload(2, &envelope2)
        );
        server.mock(|when, then| {
            when.method(GET).path("/v1/torrents/events");
            then.status(200)
                .header("content-type", "text/event-stream")
                .body(body);
        });

        let resume_path = std::env::temp_dir().join("revaer-tail-resume.txt");
        let args = tail_args(Some(resume_path.clone()));
        let response = Client::new()
            .get(format!("{}/v1/torrents/events", server.base_url()))
            .send()
            .await?;
        let mut resume_id = 0;
        let last_id = stream_events(response, &args, Some(&mut resume_id)).await?;

        assert_eq!(last_id, Some(2));
        assert_eq!(resume_id, 2);
        let stored = fs::read_to_string(&resume_path).await?;
        assert_eq!(stored.trim(), "2");
        fs::remove_file(&resume_path).await?;
        Ok(())
    }

    #[tokio::test]
    async fn stream_events_skips_duplicate_event_ids() -> Result<()> {
        let server = MockServer::start_async().await;
        let envelope = build_envelope(2);
        server.mock(|when, then| {
            when.method(GET).path("/v1/torrents/events");
            then.status(200)
                .header("content-type", "text/event-stream")
                .body(sse_payload(2, &envelope));
        });

        let resume_path = std::env::temp_dir().join("revaer-tail-resume-dup.txt");
        fs::write(&resume_path, "2").await?;
        let args = tail_args(Some(resume_path.clone()));
        let response = Client::new()
            .get(format!("{}/v1/torrents/events", server.base_url()))
            .send()
            .await?;
        let mut resume_id = 2;
        let last_id = stream_events(response, &args, Some(&mut resume_id)).await?;

        assert_eq!(last_id, Some(2));
        assert_eq!(resume_id, 2);
        let stored = fs::read_to_string(&resume_path).await?;
        assert_eq!(stored.trim(), "2");
        fs::remove_file(&resume_path).await?;
        Ok(())
    }
}
