use std::{
    net::UdpSocket,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{spawn, JoinHandle},
    time::{Duration, Instant},
};

use crate::{config::Config, screen::Screen};

pub struct ColorSenderTask {
    is_running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    config: Config,
}

impl ColorSenderTask {
    pub fn new(config: Config) -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            thread: None,
            config,
        }
    }

    pub fn start(&mut self) {
        self.is_running.store(true, Ordering::Relaxed);

        let wled_ip = self.config.wled_ip.clone();
        let mut screen = Screen::new(self.config.clone());
        let is_running = self.is_running.clone();
        self.thread = Some(spawn(move || {
            let mut frame_count = 0;
            let mut start = Instant::now();

            while is_running.load(Ordering::Relaxed) {
                let Ok(socket) = UdpSocket::bind("0.0.0.0:0") else { 
                    println!("Could not create UDP socket, trying again in 2 seconds");
                    std::thread::sleep(Duration::from_secs(2));
                    continue;
                };

                if socket.connect(format!("{}:21324", wled_ip)).is_err() {
                    println!("Could not connnect to WLED, trying again in 2 seconds");
                    std::thread::sleep(Duration::from_secs(2));
                    continue;
                }

                while is_running.load(Ordering::Relaxed) {
                    frame_count += 1;

                    if frame_count % 120 == 0 {
                        let duration = start.elapsed();
                        println!("Fps: {}", (frame_count * 1000) / duration.as_millis());
                        if duration > Duration::from_secs(5) {
                            start = Instant::now();
                            frame_count = 0;
                        }
                    }

                    screen.wait_for_next_frame();
                    if let Some(colors) = screen.get_border_colors() {
                        let buffer = colors.concat();

                        let mut wled_packet: Vec<u8> = Vec::new();
                        wled_packet.reserve(buffer.len() + 2);
                        wled_packet.extend_from_slice(&[3, 5]);
                        wled_packet.extend_from_slice(&buffer);

                        if socket.send(&wled_packet).is_err() {
                            println!("Could not send a frame. Reconnecting...");
                            break;
                        }
                    }
                }
            }
        }));
    }

    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().expect("could not stop background job");
        }
    }
}
