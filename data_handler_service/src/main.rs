use std::{collections::HashMap, sync::Arc, thread, vec};

use tokio::{sync::Mutex, task::JoinHandle};
use zmq::{Context, Socket, SUB};

use movements::{Data3d, Position};
use prost::Message;
use std::time::Duration;

pub mod movements {
    tonic::include_proto!("movements");
}

const URL: &'static str = "tcp://127.0.0.1:5555";

#[derive(Debug)]
struct RingBuffer<T: Clone> {
    buffer: Vec<Option<T>>,
    head: usize,
    tail: usize,
}

impl<T: Clone> RingBuffer<T> {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![None; size],
            head: 0,
            tail: 0,
        }
    }

    fn push(&mut self, item: T) {
        if self.is_full() {
            self.tail = (self.tail + 1) % self.buffer.len();
        }
        self.buffer[self.head] = Some(item);
        self.head = (self.head + 1) % self.buffer.len();
    }

    fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        let item = self.buffer[self.tail].take();
        self.tail = (self.tail + 1) % self.buffer.len();
        item
    }

    fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    fn is_full(&self) -> bool {
        !self.is_empty() && self.head == self.tail
    }

    fn iter(&self) -> RingBufferIterator<T> {
        RingBufferIterator {
            buffer: &self.buffer,
            current_index: self.tail,
            remaining: self.buffer.len(),
        }
    }
}

struct RingBufferIterator<'a, T: Clone> {
    buffer: &'a Vec<Option<T>>,
    current_index: usize,
    remaining: usize,
}

impl<'a, T: Clone> Iterator for RingBufferIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining > 0 {
            let item = self.buffer[self.current_index].as_ref()?;
            self.remaining -= 1;
            self.current_index = (self.current_index + 1) % self.buffer.len();
            Some(item)
        } else {
            None
        }
    }
}

#[tokio::main]
async fn main() {
    let ctx = Context::new();
    let subscriber: Socket = ctx.socket(SUB).unwrap();
    subscriber.bind(URL).unwrap();
    println!("Subscriber connected to server");
    subscriber.set_subscribe(b"").unwrap();

    let player_map = Arc::new(Mutex::new(HashMap::<u64, RingBuffer<Data3d>>::new()));
    let player_map_clone = Arc::clone(&player_map);

    // Thread for populating player_map
    let populate_thread: JoinHandle<()> = tokio::spawn(async move {
        loop {
            let msg = subscriber.recv_msg(0).unwrap();
            let position = Position::decode(&*msg).unwrap();

            let mut map = player_map.lock().await;
            map.entry(position.sensor_id)
                .or_insert_with(|| RingBuffer::new(50))
                .push(position.position);
        }
    });

    // Thread for periodically printing player_map
    let print_thread: JoinHandle<()> = tokio::spawn(async move {
        loop {
            thread::sleep(Duration::from_secs(10));
            let map = player_map_clone.lock().await;
            for (id, buffer) in map.iter() {
                println!("Player {}: ", id);
                buffer.iter().for_each(|data| {
                    println!("    {:?}", data);
                });
            }
        }
    });
    populate_thread.await.unwrap();
    print_thread.await.unwrap();
}
