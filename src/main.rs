use std::time::SystemTime;

use movements::{Data3d, Position};
use rand::Rng;

use tokio::time::{sleep, Duration};

pub mod movements {
    tonic::include_proto!("movements");
}

const FIELD_WIDTH: f32 = 100.0;
const FIELD_HEIGHT: f32 = 100.0;
const PLAYER_COUNT: u8 = 10;
const NOISE_RANGE: f32 = 0.3;

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

    fn update(&mut self, velocity: f32) -> &mut Position {
        let mut rng = rand::thread_rng();
        let direction_change_x: f32 = rng.gen_range(-0.1..=0.1);
        let direction_change_y: f32 = rng.gen_range(-0.1..=0.1);

        // Update position with direction change
        self.position.x += direction_change_x + rng.gen_range(-1.0..=1.0) + velocity;
        self.position.y += direction_change_y + rng.gen_range(-1.0..=1.0) + velocity;
        self
    }

    fn apply_noise(&mut self) -> &mut Position {
        let mut rng = rand::thread_rng();
        self.position.x += rng.gen_range(-NOISE_RANGE..=NOISE_RANGE);
        self.position.y += rng.gen_range(-NOISE_RANGE..=NOISE_RANGE);
        self
    }

    fn ensure_in_bounds(&mut self) {
        if self.position.x < 0.0 {
            self.position.x = 0.0;
        } else if self.position.x > FIELD_WIDTH {
            self.position.x = FIELD_WIDTH;
        }

        if self.position.y < 0.0 {
            self.position.y = 0.0;
        } else if self.position.y > FIELD_HEIGHT {
            self.position.y = FIELD_HEIGHT;
        }
    }
}

struct Signal;

impl Signal {
    fn broadcast(&self, position: &mut Position, velocity: f32) {
        position.update(velocity).apply_noise().ensure_in_bounds();
        println!("{:?}: {:?}", position.sensor_id, position.position);
    }
}

#[tokio::main]
async fn main() {
    let mut positions = Vec::new();
    let mut rng = rand::thread_rng();
    for i in 0..PLAYER_COUNT {
        positions.push(Position::new(i as u64));
    }

    let signal = Signal;
    loop {
        for position in &mut positions {
            let velocity = rng.gen_range(-1.0..=1.0) as f32;
            signal.broadcast(position, velocity);
        }
        sleep(Duration::from_millis(1000)).await;
    }
}
