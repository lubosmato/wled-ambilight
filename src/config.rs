use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum WledType {
    Rgb = 2,
    Rgbw = 3,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    readme: String,
    pub display_index: u32,
    pub gpu_index: u32,
    /// Horizontal count of LEDs including border pixels
    /// Try to match aspect ratio of your display. E.g. H/(V+2) ≈ 16/9
    /**
       ┌──────────────────────────────────────┐
       │            Horizontal LEDs           │
       ├───┬──────────────────────────────┬───┤
       │ V │                              │ V │
       │ e │                              │ e │
       │ r │                              │ r │
       │ t │           Display            │ t │
       │ i │                              │ i │
       │ c │                              │ c │
       │ a │                              │ a │
       │ l │                              │ l │
       ├───┴──────────────────────────────┴───┤
       │            Horizontal LEDs           │
       └──────────────────────────────────────┘
    */
    pub led_horizontal_count: u32,
    /// Veritcal count of LEDs NOT including border pixels - total LED count is (horizontal + vertical) * 2
    /**
       ┌──────────────────────────────────────┐
       │            Horizontal LEDs           │
       ├───┬──────────────────────────────┬───┤
       │ V │                              │ V │
       │ e │                              │ e │
       │ r │                              │ r │
       │ t │           Display            │ t │
       │ i │                              │ i │
       │ c │                              │ c │
       │ a │                              │ a │
       │ l │                              │ l │
       ├───┴──────────────────────────────┴───┤
       │            Horizontal LEDs           │
       └──────────────────────────────────────┘
    */
    pub led_vertical_count: u32,
    pub include_cursor: bool,
    pub max_fps: u32,
    pub enable_v_sync: bool,
    pub wled_type: WledType,
    pub wled_ip: String,
}

impl Config {
    pub fn load() -> Self {
        let path = Path::new("./config.toml");
        let content = fs::read_to_string(path);
        if content.is_err() {
            let content = toml::to_string(&Self::default()).expect("could not deserialize config");
            fs::write(path, content).expect("could not save default config");
        };

        let content = fs::read_to_string(path).expect("could not load config");
        toml::from_str(&content).expect("could not load config")
    }

    fn default() -> Self {
        Config {
            readme: r#"Total LED count = Horizontal + Vertical LEDs.
For best experience try matching aspect ratio of your display: H/(V+2) ≈ 16/9.
┌──────────────────────────────────────┐
│B →         Horizontal LEDs         ↓ │
├───┬──────────────────────────────┬───┤
│ E │                              │   │
│   │                              │ V │
│ V │                              │ e │
│ e │                              │ r │
│ r │                              │ t │
│ t │     Display (front screen)   │ i │
│ i │                              │ c │
│ c │                              │ a │
│ a │                              │ l │
│ l │                              │   │
├───┴──────────────────────────────┴───┤
│ ↑          Horizontal LEDs         ← │
└──────────────────────────────────────┘
With enabled V-Sync max_fps is ignored.
B is starting point (index 0), clock-wise indexing, until E (last index).
wled_type:
  "Rgbw" sends RGBW values to WLED
  "Rgb" sends RGB values to WLED
"#
            .to_string(),
            display_index: 0,
            gpu_index: 0,
            include_cursor: true,
            led_horizontal_count: 27,
            led_vertical_count: 14,
            max_fps: 60,
            enable_v_sync: true,
            wled_type: WledType::Rgbw,
            wled_ip: "192.168.0.150".to_string(),
        }
    }
}
