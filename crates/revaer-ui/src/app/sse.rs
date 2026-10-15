//! SSE connection runner for the app shell.
//!
//! # Design
//! - Use fetch streaming so auth headers can be attached to requests.
//! - Expose a cancellable handle so callers can stop the stream on unmount.

use crate::app::preferences::{clear_last_event_id, load_last_event_id, persist_last_event_id};
use crate::core::auth::{AuthState, LocalAuth};
use crate::core::events::UiEventEnvelope;
use crate::core::logic::{SseEndpoint, SseQuery, backoff_delay_ms, build_sse_url};
use crate::core::store::{SseConnectionState, SseError, SseStatus};
use crate::services::sse::{SseDecodeError, SseFrame, SseParser, decode_frame};
use gloo::console;
use gloo_timers::future::TimeoutFuture;
use js_sys::Date;
use js_sys::{Reflect, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AbortController, AbortSignal, Headers, ReadableStream, ReadableStreamDefaultReader, Request,
    RequestInit, RequestMode, Response, TextDecoder,
};
use yew::Callback;

/// Active SSE stream handle for cancellation.
pub(crate) struct SseHandle {
    controller: AbortController,
}

impl SseHandle {
    pub(crate) fn close(&self) {
        self.controller.abort();
    }
}

/// Spawn an SSE loop with auth headers and return a cancellable handle.
pub(crate) fn connect_sse(
    base_url: String,
    auth: Option<AuthState>,
    query: SseQuery,
    on_event: Callback<UiEventEnvelope>,
    on_error: Callback<SseDecodeError>,
    on_state: Callback<SseStatus>,
) -> Option<SseHandle> {
    let controller = AbortController::new().ok()?;
    let signal = controller.signal();
    yew::platform::spawn_local(async move {
        run_sse_loop(base_url, auth, query, signal, on_event, on_error, on_state).await;
    });
    Some(SseHandle { controller })
}

struct SseLoopState {
    auth_label: Option<String>,
    attempt: u32,
    retry_hint_ms: Option<u64>,
    last_event_id: Option<u64>,
}

struct SseCallbacks {
    on_event: Callback<UiEventEnvelope>,
    on_error: Callback<SseDecodeError>,
    on_state: Callback<SseStatus>,
}

async fn run_sse_loop(
    base_url: String,
    auth: Option<AuthState>,
    query: SseQuery,
    signal: AbortSignal,
    on_event: Callback<UiEventEnvelope>,
    on_error: Callback<SseDecodeError>,
    on_state: Callback<SseStatus>,
) {
    let mut state = SseLoopState {
        auth_label: auth_label(&auth),
        attempt: 0,
        retry_hint_ms: None,
        last_event_id: load_last_event_id(),
    };
    let callbacks = SseCallbacks {
        on_event,
        on_error,
        on_state,
    };
    while !signal.aborted() {
        emit_stream_state(
            &callbacks,
            &state,
            SseConnectionState::Reconnecting,
            Some(SseError {
                message: "connecting".to_string(),
                status_code: None,
            }),
        );
        match open_stream(&base_url, &auth, &query, state.last_event_id, &signal).await {
            Ok(reader) => {
                let Err(error) = consume_sse_stream(reader, &signal, &mut state, &callbacks).await
                else {
                    return;
                };
                if reconnect_after_read_error(error, &signal, &state, &callbacks)
                    .await
                    .is_err()
                {
                    return;
                }
            }
            Err(error) => {
                if reconnect_after_open_error(error, &signal, &mut state, &callbacks)
                    .await
                    .is_err()
                {
                    return;
                }
            }
        }
        state.attempt = state.attempt.saturating_add(1);
    }
}

#[derive(Debug)]
enum SseReadError {
    Decoder(JsValue),
    Decode(JsValue),
    Read(String),
    Ended,
}

impl std::fmt::Display for SseReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Decoder(error) => write!(f, "decoder error: {error:?}"),
            Self::Decode(error) => write!(f, "decode error: {error:?}"),
            Self::Read(error) => write!(f, "read error: {error}"),
            Self::Ended => f.write_str("stream ended"),
        }
    }
}

async fn consume_sse_stream(
    mut reader: ReadableStreamDefaultReader,
    signal: &AbortSignal,
    state: &mut SseLoopState,
    callbacks: &SseCallbacks,
) -> Result<(), SseReadError> {
    let mut parser = SseParser::default();
    let decoder = TextDecoder::new().map_err(SseReadError::Decoder)?;
    state.attempt = 0;
    state.retry_hint_ms = None;
    emit_stream_state(callbacks, state, SseConnectionState::Connected, None);
    loop {
        if signal.aborted() {
            return Ok(());
        }
        match read_chunk(&mut reader).await.map_err(SseReadError::Read)? {
            Some(bytes) => {
                let text = decoder
                    .decode_with_js_u8_array(&bytes)
                    .map_err(SseReadError::Decode)?;
                dispatch_sse_frames(parser.push(&text), state, callbacks);
            }
            None => {
                if let Some(frame) = parser.finish()
                    && let Err(error) = dispatch_sse_frame(&frame, state, callbacks)
                {
                    if signal.aborted() {
                        return Ok(());
                    }
                    callbacks.on_error.emit(error);
                }
                return Err(SseReadError::Ended);
            }
        }
    }
}

fn dispatch_sse_frames(
    frames: impl IntoIterator<Item = SseFrame>,
    state: &mut SseLoopState,
    callbacks: &SseCallbacks,
) {
    for frame in frames {
        if let Some(retry) = frame.retry {
            state.retry_hint_ms = Some(retry);
        }
        if let Err(error) = dispatch_sse_frame(&frame, state, callbacks) {
            callbacks.on_error.emit(error);
        }
    }
}

fn dispatch_sse_frame(
    frame: &SseFrame,
    state: &mut SseLoopState,
    callbacks: &SseCallbacks,
) -> Result<(), SseDecodeError> {
    let envelope = decode_frame(frame)?;
    if let Some(id) = envelope.id {
        state.last_event_id = Some(id);
        persist_last_event_id(id);
    }
    emit_stream_state(callbacks, state, SseConnectionState::Connected, None);
    callbacks.on_event.emit(envelope);
    Ok(())
}

fn emit_stream_state(
    callbacks: &SseCallbacks,
    state: &SseLoopState,
    connection: SseConnectionState,
    error: Option<SseError>,
) {
    emit_status(
        &callbacks.on_state,
        connection,
        &state.auth_label,
        state.last_event_id,
        None,
        None,
        error,
    );
}

async fn reconnect_after_read_error(
    error: SseReadError,
    signal: &AbortSignal,
    state: &SseLoopState,
    callbacks: &SseCallbacks,
) -> Result<(), SseReadError> {
    let fatal = matches!(&error, SseReadError::Decoder(_) | SseReadError::Decode(_));
    if fatal && signal.aborted() {
        return Err(error);
    }
    let status_error = SseError {
        message: error.to_string(),
        status_code: None,
    };
    emit_stream_state(
        callbacks,
        state,
        SseConnectionState::Disconnected,
        Some(status_error.clone()),
    );
    if fatal || signal.aborted() {
        return Err(error);
    }
    schedule_reconnect(
        &callbacks.on_state,
        &state.auth_label,
        state.retry_hint_ms,
        state.attempt,
        state.last_event_id,
        status_error,
    )
    .await;
    Ok(())
}

async fn reconnect_after_open_error(
    error: ConnectError,
    signal: &AbortSignal,
    state: &mut SseLoopState,
    callbacks: &SseCallbacks,
) -> Result<(), ConnectError> {
    if signal.aborted() {
        return Err(error);
    }
    let conflict = matches!(&error, ConnectError::Conflict);
    if conflict {
        clear_last_event_id();
        state.last_event_id = None;
        state.retry_hint_ms = None;
    }
    let status_error = connect_error_to_sse_error(&error);
    emit_stream_state(
        callbacks,
        state,
        SseConnectionState::Disconnected,
        Some(status_error.clone()),
    );
    if !(conflict || should_reconnect(&error)) || signal.aborted() {
        return Err(error);
    }
    let attempt = if conflict { 0 } else { state.attempt };
    schedule_reconnect(
        &callbacks.on_state,
        &state.auth_label,
        state.retry_hint_ms,
        attempt,
        state.last_event_id,
        status_error,
    )
    .await;
    Ok(())
}

async fn open_stream(
    base_url: &str,
    auth: &Option<AuthState>,
    query: &SseQuery,
    last_event_id: Option<u64>,
    signal: &AbortSignal,
) -> Result<ReadableStreamDefaultReader, ConnectError> {
    let primary = build_sse_url(base_url, SseEndpoint::Primary, Some(query));
    let response = fetch_stream(&primary, auth, last_event_id, signal).await?;
    stream_reader(response)
}

fn stream_reader(response: Response) -> Result<ReadableStreamDefaultReader, ConnectError> {
    let stream: ReadableStream = response.body().ok_or(ConnectError::Stream)?;
    let reader = stream
        .get_reader()
        .dyn_into::<ReadableStreamDefaultReader>()
        .map_err(|_| ConnectError::Reader)?;
    Ok(reader)
}

async fn fetch_stream(
    url: &str,
    auth: &Option<AuthState>,
    last_event_id: Option<u64>,
    signal: &AbortSignal,
) -> Result<Response, ConnectError> {
    let window = web_sys::window().ok_or(ConnectError::Window)?;
    let init = RequestInit::new();
    init.set_method("GET");
    init.set_mode(RequestMode::Cors);
    init.set_signal(Some(signal));

    let headers = Headers::new().map_err(|_| ConnectError::Headers)?;
    apply_auth(&headers, auth);
    if let Some(id) = last_event_id {
        set_header(&headers, "Last-Event-ID", &id.to_string());
    }
    init.set_headers(&headers);

    let request = Request::new_with_str_and_init(url, &init).map_err(|_| ConnectError::Request)?;
    let resp = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| ConnectError::Fetch)?;
    let response: Response = resp.dyn_into().map_err(|_| ConnectError::Fetch)?;
    if response.status() == 409 {
        return Err(ConnectError::Conflict);
    }
    if !response.ok() {
        return Err(ConnectError::Status(response.status()));
    }
    Ok(response)
}

fn apply_auth(headers: &Headers, auth: &Option<AuthState>) {
    match auth {
        Some(AuthState::ApiKey(key)) if !key.trim().is_empty() => {
            set_header(headers, "x-revaer-api-key", key);
        }
        Some(AuthState::Local(auth)) => {
            if let Some(header) = basic_auth_header(auth) {
                set_header(headers, "Authorization", &header);
            } else {
                console::error!("basic auth header unavailable");
            }
        }
        _ => {}
    }
}

fn set_header(headers: &Headers, name: &'static str, value: &str) {
    if let Err(err) = headers.set(name, value) {
        log_header_error(name, err);
    }
}

fn log_header_error(name: &'static str, err: JsValue) {
    console::error!("request header set failed", name, err);
}

fn basic_auth_header(auth: &LocalAuth) -> Option<String> {
    let raw = format!("{}:{}", auth.username, auth.password);
    let encoded = web_sys::window()?.btoa(&raw).ok()?;
    Some(format!("Basic {}", encoded))
}

async fn read_chunk(
    reader: &mut ReadableStreamDefaultReader,
) -> Result<Option<Uint8Array>, String> {
    let chunk = JsFuture::from(reader.read())
        .await
        .map_err(|err| format!("read failed: {err:?}"))?;
    let done = Reflect::get(&chunk, &JsValue::from_str("done"))
        .map_err(|err| format!("chunk done lookup failed: {err:?}"))?
        .as_bool()
        .unwrap_or(false);
    if done {
        return Ok(None);
    }
    let value = Reflect::get(&chunk, &JsValue::from_str("value"))
        .map_err(|err| format!("chunk value lookup failed: {err:?}"))?;
    Ok(Some(Uint8Array::new(&value)))
}

fn emit_status(
    on_state: &Callback<SseStatus>,
    state: SseConnectionState,
    auth_label: &Option<String>,
    last_event_id: Option<u64>,
    backoff_ms: Option<u64>,
    next_retry_at_ms: Option<u64>,
    last_error: Option<SseError>,
) {
    on_state.emit(SseStatus {
        state,
        backoff_ms,
        next_retry_at_ms,
        last_event_id,
        last_error,
        auth_mode: auth_label.clone(),
    });
}

fn auth_label(auth: &Option<AuthState>) -> Option<String> {
    match auth {
        Some(AuthState::ApiKey(_)) => Some("API key".to_string()),
        Some(AuthState::Local(_)) => Some("Local auth".to_string()),
        Some(AuthState::Anonymous) => Some("Anonymous".to_string()),
        None => None,
    }
}

fn connect_error_to_sse_error(err: &ConnectError) -> SseError {
    SseError {
        message: err.to_string(),
        status_code: err.status_code(),
    }
}

fn now_ms() -> u64 {
    Date::now() as u64
}

async fn schedule_reconnect(
    on_state: &Callback<SseStatus>,
    auth_label: &Option<String>,
    retry_hint_ms: Option<u64>,
    attempt: u32,
    last_event_id: Option<u64>,
    error: SseError,
) {
    let delay_ms = retry_hint_ms.unwrap_or(u64::from(backoff_delay_ms(attempt)));
    let next_retry_at = now_ms().saturating_add(delay_ms);
    emit_status(
        on_state,
        SseConnectionState::Reconnecting,
        auth_label,
        last_event_id,
        Some(delay_ms),
        Some(next_retry_at),
        Some(error),
    );
    TimeoutFuture::new(delay_ms as u32).await;
}

#[derive(Debug)]
enum ConnectError {
    Window,
    Headers,
    Request,
    Fetch,
    Stream,
    Reader,
    Conflict,
    Status(u16),
}

impl ConnectError {
    fn status_code(&self) -> Option<u16> {
        match self {
            Self::Conflict => Some(409),
            Self::Status(code) => Some(*code),
            _ => None,
        }
    }
}

impl std::fmt::Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Window => write!(f, "window unavailable"),
            Self::Headers => write!(f, "headers unavailable"),
            Self::Request => write!(f, "request build failed"),
            Self::Fetch => write!(f, "fetch failed"),
            Self::Stream => write!(f, "SSE response missing body"),
            Self::Reader => write!(f, "SSE stream reader unavailable"),
            Self::Conflict => write!(f, "conflict"),
            Self::Status(code) => write!(f, "http {code}"),
        }
    }
}

fn should_reconnect(err: &ConnectError) -> bool {
    matches!(err, ConnectError::Fetch)
}
