// Project: shopify-scraper
// File: tests/common/mod.rs
// Description: Local HTTP fixture server, never contacts external hosts.
// Author: Jan Alexandr Kopřiva
// License: AGPL-3.0-or-later
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
pub struct Server {
    pub url: String,
    pub requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl Server {
    pub fn new(handler: impl Fn(&str, usize) -> (u16, Vec<u8>) + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = requests.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let done = stop.clone();
        let thread = thread::spawn(move || {
            while !done.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        // Windows sockets inherit non-blocking mode from the listener.
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(Duration::from_secs(3)))
                            .unwrap();
                        let mut request = Vec::new();
                        let mut buf = [0; 1024];
                        loop {
                            match stream.read(&mut buf) {
                                Ok(0) | Err(_) => break,
                                Ok(n) => {
                                    request.extend_from_slice(&buf[..n]);
                                    if request.windows(4).any(|w| w == b"\r\n\r\n") {
                                        break;
                                    }
                                }
                            }
                        }
                        let request = String::from_utf8_lossy(&request).into_owned();
                        let path = request.split_whitespace().nth(1).unwrap_or("/");
                        let mut list = seen.lock().unwrap();
                        let index = list.len();
                        list.push(request.clone());
                        drop(list);
                        let (status, body) = handler(path, index);
                        let reason = if status == 200 {
                            "OK"
                        } else {
                            "Too Many Requests"
                        };
                        let _=write!(stream,"HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n",body.len());
                        let _ = stream.write_all(&body);
                        let _ = stream.shutdown(std::net::Shutdown::Write);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1))
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            url,
            requests,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}
