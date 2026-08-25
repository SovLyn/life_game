use web_sys::js_sys::Math::random;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Config {
    pub inner_range: f32,
    pub outer_range: f32,
    pub alpha: f32,
    pub acc_matrix: [f32; 25],
    pub cla: usize, 
    pub particle_num: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            inner_range: 4.0,
            outer_range: 50.0,
            alpha: 0.8,
            acc_matrix: Self::random_matrix(),
            cla: 3,
            particle_num: 1000,
        }
    }
}

impl Config {
    pub fn new() -> Config {
        Self::default()
    }

    fn random_matrix() -> [f32; 25] {
        let mut m = [0.0f32; 25];
        for v in m.iter_mut() {
            *v = (random() * 2.0 - 1.0) as f32 * 0.4;
        }
        m
    }

    // 重新随机生成整个相互作用矩阵（值域 [-0.4, 0.4]，与默认初始化一致）
    pub fn randomize_matrix(&mut self) {
        self.acc_matrix = Self::random_matrix();
    }
}
