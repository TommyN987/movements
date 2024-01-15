use std::time::SystemTime;

use movements::{Data3d, Position};
use prost::Message;
use rand::Rng;

use tokio::time::{sleep, Duration};
use zmq::{Context, Socket, PUB};

pub mod movements {
    tonic::include_proto!("movements");
}

const FIELD_WIDTH: f32 = 100.0;
const FIELD_HEIGHT: f32 = 100.0;
const PLAYER_COUNT: u8 = 10;
const NOISE_RANGE: f32 = 0.3;
const URL: &str = "tcp://127.0.0.1:5555";

impl Data3d {
    fn new() -> Self {
        Self {
            x: rand::random::<f32>() * FIELD_WIDTH,
            y: rand::random::<f32>() * FIELD_HEIGHT,
            z: 0.0,
        }
    }
}

impl Position {
    fn new(sensor_id: u64) -> Self {
        Self {
            sensor_id,
            timestamp_usec: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            position: Data3d::new(),
        }
    }
}

enum Direction {
    Positive,
    Negative,
}

impl Direction {
    fn random() -> Self {
        match rand::random() {
            true => Self::Positive,
            false => Self::Negative,
        }
    }

    fn turn(&mut self) {
        match self {
            Self::Positive => *self = Self::Negative,
            Self::Negative => *self = Self::Positive,
        }
    }

    fn get_factor(&self) -> f32 {
        match self {
            Self::Positive => 1.0,
            Self::Negative => -1.0,
        }
    }
}

struct Movement {
    position: Position,
    direction_x: Direction,
    direction_y: Direction,
}

impl Movement {
    fn new(position: Position) -> Self {
        Self {
            position,
            direction_x: Direction::random(),
            direction_y: Direction::random(),
        }
    }

    fn update(&mut self) -> &mut Movement {
        let mut rng = rand::thread_rng();

        self.position.position.x += self.direction_x.get_factor() * rng.gen_range(0.0..=1.0);
        self.position.position.y += self.direction_y.get_factor() * rng.gen_range(0.0..=1.0);

        self.position.timestamp_usec = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        self
    }

    fn apply_noise(&mut self) -> &mut Movement {
        let mut rng = rand::thread_rng();
        self.position.position.x += rng.gen_range(-NOISE_RANGE..=NOISE_RANGE);
        self.position.position.y += rng.gen_range(-NOISE_RANGE..=NOISE_RANGE);
        self
    }

    fn ensure_in_bounds(&mut self) {
        if self.position.position.x < 0.0 {
            self.position.position.x = 0.0;
            self.direction_x.turn();
        } else if self.position.position.x > FIELD_WIDTH {
            self.position.position.x = FIELD_WIDTH;
            self.direction_x.turn();
        }

        if self.position.position.y < 0.0 {
            self.position.position.y = 0.0;
            self.direction_y.turn();
        } else if self.position.position.y > FIELD_HEIGHT {
            self.position.position.y = FIELD_HEIGHT;
            self.direction_y.turn();
        }
    }
}

struct Signal;

impl Signal {
    fn broadcast(&self, movement: &mut Movement, publisher: &Socket) {
        movement.update().apply_noise().ensure_in_bounds();
        let mut posistion_bytes = Vec::new();
        movement.position.encode(&mut posistion_bytes).unwrap();

        publisher.send(&posistion_bytes, 0).unwrap();
    }
}

#[tokio::main]
async fn main() {
    let mut handles = Vec::new();
    let ctx = Context::new();

    let mut publishers = Vec::new();
    for _ in 0..PLAYER_COUNT {
        let publisher = ctx.socket(PUB).unwrap();
        publisher.connect(URL).unwrap();
        publishers.push(publisher);
    }

    publishers
        .into_iter()
        .enumerate()
        .for_each(|(i, publisher)| {
            let handle = tokio::spawn(async move {
                let mut movement = Movement::new(Position::new(i as u64));
                loop {
                    Signal.broadcast(&mut movement, &publisher);
                    sleep(Duration::from_millis(1000)).await;
                }
            });
            handles.push(handle);
        });

    for handle in handles {
        handle.await.unwrap();
    }
}
