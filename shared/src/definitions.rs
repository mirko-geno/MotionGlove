use embassy_time::Duration;

pub const WIFI_NETWORK: &str            = "MotionGlove-Network";
pub const WIFI_PASSWORD: &str           = "Password123";
pub const TCP_CHANNEL: u8               = 5;
pub const TCP_ENDPOINT: u16             = 50124;
pub const SOCKET_TIMEOUT: Duration      = Duration::from_secs(15);
pub const DONGLE_IP: &str               = "192.168.0.10";
pub const GLOVE_IP: &str                = "192.168.0.12";
pub const CHANNEL_SIZE: usize           = 1;
pub const READ_FREQ: u64                = 1000;
pub const DELTA_TIME: f32               = 1.0 / READ_FREQ as f32;
pub const MOUSE_POLL_MS: u8             = 1;
pub const ROLL_SENS: f32                = 30.0; // Pixel movement per roll angle
pub const PITCH_SENS: f32               = 50.0; // Pixel movement per pitch angle
pub const WHEEL_SENS: f32               = 20.0;
pub const PAN_SENS: f32                 = 20.0;
pub const DEAD_ZONE: f32                = 2.5;