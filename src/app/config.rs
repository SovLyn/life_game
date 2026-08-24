#[derive(Clone, Copy)]
pub struct Config {
    pub point_size: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self { point_size: 10.0 }
    }
}

impl Config {
    pub fn new() -> Config {
        Self::default()
    }
}
