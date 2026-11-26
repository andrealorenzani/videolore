// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The live picture (REQ-343): raw RGB frames read from ffmpeg's standard output. A
//! thread reads all of them and keeps only the newest, so a window that is slow to draw
//! never stalls ffmpeg, and so never the recording.

use std::io::Read;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// One picture, three bytes per pixel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

/// The newest picture of one capture process.
#[derive(Debug, Default)]
pub struct Feed {
    latest: Mutex<Option<Frame>>,
    /// Pictures read so far.
    count: AtomicU64,
}

impl Feed {
    /// Takes the newest picture not taken yet.
    pub fn take(&self) -> Option<Frame> {
        self.latest.lock().ok().and_then(|mut l| l.take())
    }

    /// How many pictures arrived.
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::SeqCst)
    }

    /// Reads pictures of `width`x`height` until the stream ends; a picture cut short at
    /// the end is dropped.
    pub fn read(&self, mut from: impl Read, width: u32, height: u32) {
        let size = width as usize * height as usize * 3;
        if size == 0 {
            return;
        }
        let mut buf = vec![0u8; size];
        while from.read_exact(&mut buf).is_ok() {
            if let Ok(mut latest) = self.latest.lock() {
                // the buffer of a picture nobody took is used again
                let old = latest.replace(Frame {
                    width,
                    height,
                    rgb: std::mem::take(&mut buf),
                });
                buf = old.map(|f| f.rgb).unwrap_or_default();
            }
            buf.resize(size, 0);
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }
}

/// Reads the pictures of `from` in a thread of its own; it ends with the stream.
pub fn start(from: Box<dyn Read + Send>, width: u32, height: u32) -> Arc<Feed> {
    let feed = Arc::new(Feed::default());
    let writer = Arc::clone(&feed);
    std::thread::spawn(move || writer.read(from, width, height));
    feed
}
