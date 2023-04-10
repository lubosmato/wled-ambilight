mod color_sender_task;
mod config;
mod screen;

use color_sender_task::ColorSenderTask;
use config::Config;
use {std::sync::mpsc, tray_item::TrayItem};

enum Message {
    Start,
    Stop,
    Quit,
}

fn main() {
    let config = Config::load();
    let mut tray = TrayItem::new("WLED Ambilight", "tray-icon").unwrap();

    tray.add_label("WLED Ambilight").unwrap();

    let (tx, rx) = mpsc::channel();

    let start_tx = tx.clone();
    tray.add_menu_item("Start", move || {
        start_tx.send(Message::Start).unwrap();
    })
    .unwrap();

    let stop_tx = tx.clone();
    tray.add_menu_item("Stop", move || {
        stop_tx.send(Message::Stop).unwrap();
    })
    .unwrap();

    let quit_tx = tx.clone();
    tray.add_menu_item("Quit", move || {
        println!("Quit");
        quit_tx.send(Message::Quit).unwrap();
    })
    .unwrap();

    let mut sender = ColorSenderTask::new(config);

    loop {
        match rx.recv() {
            Ok(Message::Quit) => break,
            Ok(Message::Start) => {
                sender.start();
            }
            Ok(Message::Stop) => {
                sender.stop();
            }
            _ => {}
        }
    }
}
