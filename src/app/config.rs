use web_sys::js_sys::Math::random;

#[derive(Clone, Copy)]
pub struct Config {
    pub inner_range: f32,
    pub outer_range: f32,
    pub alpha: f32,
    pub acc_matrix: [f32; 25],
}

impl Default for Config {
    fn default() -> Self {
        let mut acc_matrix: [f32; 25] = [0.0; 25];
        for i in 0..25 {
            acc_matrix[i] = (random() * 2.0 - 1.0) as f32 * 0.4;
        }
        Self {
            inner_range: 4.0,
            outer_range: 50.0,
            alpha: 0.8,
            acc_matrix,
        }
    }
}

impl Config {
    pub fn new() -> Config {
        Self::default()
    }
}
