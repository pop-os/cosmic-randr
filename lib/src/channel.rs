// Copyright 2025 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use super::Message;

/// Create a channel for receiving messages from cosmic-randr.
pub fn channel() -> (Sender, Receiver) {
    let channel = Arc::new(Channel {
        queue: xutex::AsyncMutex::new(VecDeque::default()),
        notify: tokio::sync::Notify::const_new(),
        closed: AtomicBool::new(false),
    });

    (Sender(channel.clone()), Receiver(channel))
}

/// A channel specifically for handling cosmic-randr messages.
struct Channel {
    pub(self) queue: xutex::AsyncMutex<VecDeque<Message>>,
    pub(self) notify: tokio::sync::Notify,
    pub(self) closed: AtomicBool,
}

pub struct Sender(Arc<Channel>);

impl Sender {
    pub async fn send(&self, message: Message) {
        self.0.queue.lock().await.push_back(message);
        self.0.notify.notify_one();
    }
}

impl Drop for Sender {
    fn drop(&mut self) {
        self.0.closed.store(true, Ordering::SeqCst);
        self.0.notify.notify_one();
    }
}

pub struct Receiver(Arc<Channel>);

impl Receiver {
    /// Returns a value until the sender is dropped.
    pub async fn recv(&mut self) -> Option<Message> {
        loop {
            if let Some(value) = self.0.queue.lock().await.pop_front() {
                return Some(value);
            }

            if self.0.closed.load(Ordering::SeqCst) {
                return None;
            }

            self.0.notify.notified().await;
        }
    }

    pub async fn try_recv(&mut self) -> Option<Message> {
        self.0.queue.lock().await.pop_front()
    }
}
