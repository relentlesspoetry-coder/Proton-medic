// Copyright 2026 Proton Medic Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Retry only metadata GETs, before parsing or any update download/installation.
use reqwest::{header::HeaderMap, Client, Response, Url};
use std::time::Duration;
use tokio::time::{sleep, Instant};

const BACKOFF: [Duration; 2] = [Duration::from_millis(500), Duration::from_secs(1)];

pub(super) async fn fetch(
    client: &Client,
    url: Url,
    headers: HeaderMap,
    budget: Duration,
) -> reqwest::Result<Response> {
    fetch_with_backoff(client, url, headers, budget, &BACKOFF).await
}

// reqwest's Display alone omits the cause (DNS, TLS, reset, etc.). Keep the
// source chain in the existing collapsed technical details, not the status UI.
pub(super) fn describe(error: &reqwest::Error) -> String {
    use std::error::Error;
    let category = if error.is_timeout() {
        "Update metadata request timed out"
    } else if error.is_connect() {
        "Update metadata connection failed"
    } else {
        "Update metadata request failed"
    };
    let mut message = format!("{category}: {error}");
    let mut source = error.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

async fn fetch_with_backoff(
    client: &Client,
    url: Url,
    headers: HeaderMap,
    budget: Duration,
    backoff: &[Duration],
) -> reqwest::Result<Response> {
    let started = Instant::now();
    for attempt in 0..=backoff.len() {
        let response = client
            .get(url.clone())
            .headers(headers.clone())
            // Share one budget across attempts and waits, rather than tripling it.
            .timeout(budget.saturating_sub(started.elapsed()))
            .send()
            .await;
        let retry = match &response {
            Ok(response) => matches!(response.status().as_u16(), 408 | 500 | 502 | 503 | 504),
            // Do not classify generic connection errors as transient: they include
            // certificate/TLS failures. No matching of user-visible error strings.
            Err(error) => error.is_timeout(),
        };
        let Some(delay) = backoff.get(attempt) else {
            return response;
        };
        if !retry || budget.saturating_sub(started.elapsed()) <= *delay {
            return response;
        }
        drop(response);
        sleep(*delay).await;
    }
    unreachable!("the final attempt returns its response")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Arc},
        thread,
    };

    struct Server {
        url: Url,
        count: Arc<AtomicUsize>,
        stop: Arc<AtomicBool>,
        thread: Option<thread::JoinHandle<()>>,
    }
    impl Server {
        fn new(responses: Vec<(u16, &'static str, Duration)>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let url = format!("http://{}/latest.json", listener.local_addr().unwrap()).parse().unwrap();
            let count = Arc::new(AtomicUsize::new(0));
            let stop = Arc::new(AtomicBool::new(false));
            let (requests, stopped) = (count.clone(), stop.clone());
            let thread = thread::spawn(move || {
                while !stopped.load(Ordering::SeqCst) {
                    let (mut stream, _) = match listener.accept() {
                        Ok(connection) => connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(1));
                            continue;
                        }
                        Err(error) => panic!("fixture accept: {error}"),
                    };
                    stream.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
                    let mut request = [0; 8192];
                    let _ = stream.read(&mut request);
                    let index = requests.fetch_add(1, Ordering::SeqCst);
                    let (status, body, delay) = responses[index.min(responses.len() - 1)];
                    thread::sleep(delay);
                    let _ = write!(stream, "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                }
            });
            Self { url, count, stop, thread: Some(thread) }
        }
        fn count(&self) -> usize { self.count.load(Ordering::SeqCst) }
    }
    impl Drop for Server {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            self.thread.take().unwrap().join().unwrap();
        }
    }
    fn runtime() -> tokio::runtime::Runtime {
        #[cfg(feature = "rustls-tls")]
        let _ = rustls::crypto::ring::default_provider().install_default();
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()
    }
    fn response(status: u16) -> (u16, &'static str, Duration) {
        (status, "{}", Duration::ZERO)
    }
    async fn fast_fetch(server: &Server) -> reqwest::Result<Response> {
        let client = Client::builder().no_proxy().build().unwrap();
        fetch_with_backoff(&client, server.url.clone(), HeaderMap::new(), Duration::from_secs(2), &[Duration::ZERO; 2]).await
    }

    #[test]
    fn recovers_after_transient_statuses() {
        for status in [408, 500, 502, 503, 504] {
            let server = Server::new(vec![response(status), response(status), response(200)]);
            assert_eq!(runtime().block_on(fast_fetch(&server)).unwrap().status(), 200);
            assert_eq!(server.count(), 3);
        }
    }

    #[test]
    fn exhaustion_preserves_last_status_and_stops_after_three_requests() {
        let server = Server::new(vec![response(503), response(502), response(504)]);
        assert_eq!(runtime().block_on(fast_fetch(&server)).unwrap().status(), 504);
        assert_eq!(server.count(), 3);
    }

    #[test]
    fn permanent_statuses_and_no_update_are_not_retried() {
        // 429 is deliberately not retried: its Retry-After may exceed our budget.
        for status in [200, 204, 400, 401, 403, 404, 429, 501] {
            let server = Server::new(vec![response(status)]);
            assert_eq!(runtime().block_on(fast_fetch(&server)).unwrap().status(), status);
            assert_eq!(server.count(), 1);
        }
    }

    #[test]
    fn invalid_json_is_not_retried() {
        let server = Server::new(vec![(200, "not json", Duration::ZERO)]);
        runtime().block_on(async {
            let error = fast_fetch(&server).await.unwrap().json::<serde_json::Value>().await.unwrap_err();
            assert!(error.is_decode());
        });
        assert_eq!(server.count(), 1);
    }

    #[test]
    fn tls_failure_is_not_retried() {
        let mut server = Server::new(vec![response(200)]);
        server.url.set_scheme("https").unwrap();
        let error = runtime().block_on(fast_fetch(&server)).unwrap_err();
        assert!(!error.is_timeout(), "fixture must produce a TLS failure, not a timeout");
        let details = describe(&error);
        assert!(details.starts_with("Update metadata connection failed:"));
        let cause = std::error::Error::source(&error).unwrap().to_string();
        assert!(details.contains(&cause));
        assert_eq!(server.count(), 1);
    }

    #[test]
    fn timeout_uses_one_total_budget() {
        let server = Server::new(vec![(200, "{}", Duration::from_millis(150))]);
        let started = std::time::Instant::now();
        let error = runtime().block_on(async {
            fetch(&Client::builder().no_proxy().build().unwrap(), server.url.clone(), HeaderMap::new(), Duration::from_millis(50)).await
        }).unwrap_err();
        assert!(error.is_timeout());
        assert!(describe(&error).starts_with("Update metadata request timed out:"));
        assert!(started.elapsed() < Duration::from_millis(400));
        assert_eq!(server.count(), 1);
    }

    #[test]
    fn does_not_wait_or_retry_when_backoff_exceeds_remaining_budget() {
        let server = Server::new(vec![response(503)]);
        let result = runtime().block_on(async {
            fetch(&Client::builder().no_proxy().build().unwrap(), server.url.clone(), HeaderMap::new(), Duration::from_millis(100)).await
        }).unwrap();
        assert_eq!(result.status(), 503);
        assert_eq!(server.count(), 1);
    }
}
