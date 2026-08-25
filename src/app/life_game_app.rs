use eframe::Storage;
use egui::{Pos2, Rect, Vec2};

use super::frame_rate::FrameRate;

use super::data_structs::SimParams;

use super::life_game_simulator::LifeGameSimulator;

use super::config::Config;
use super::life_game_renderer::LifeGameRenderer;
use super::render_callback::RenderCallback;
// 定义LifeGameApp结构体，用于管理生命游戏的显示和交互
pub struct LifeGameApp {
    render: LifeGameRenderer,
    simulator: LifeGameSimulator,
    config: Config,
    wgpu_render_state: egui_wgpu::RenderState,
    frame_rate: FrameRate,
    last_frame_time: i64,
}

impl LifeGameApp {
    // 构造函数，用于创建LifeGameApp实例
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = if let Some(storage) = cc.storage {
            eframe::get_value(storage, "life_game_config").unwrap_or_default()
        } else {
            Config::new()
        };

        // 获取wgpu渲染状态
        let wgpu_render_state = cc
            .wgpu_render_state
            .as_ref()
            .expect("This app requires the wgpu render state");
        let surface_format = wgpu_render_state.target_format;

        let device = &wgpu_render_state.device;

        let simulator = LifeGameSimulator::new(device, config.particle_num, config.cla);

        // 创建并返回LifeGameApp实例
        Self {
            render: LifeGameRenderer::new(device, &surface_format),
            simulator,
            config,
            wgpu_render_state: wgpu_render_state.clone(),
            frame_rate: FrameRate::new(100),
            last_frame_time: chrono::offset::Utc::now().timestamp_micros(),
        }
    }

    // 绘制函数，负责处理UI绘制和交互
    fn draw(&mut self, ui: &mut egui::Ui) {
        let width = ui.available_width();
        let height = ui.available_height();
        ui.set_width(width);
        ui.set_height(height);
        let screen_size = width.min(height);
        let rect = Rect::from_center_size(
            Pos2::new(width / 2., height / 2.),
            Vec2::new(screen_size, screen_size),
        );
        let _response = ui.allocate_rect(rect, egui::Sense::drag());

        let params = self.get_params();
        let queue = &self.wgpu_render_state.queue;
        let device = &self.wgpu_render_state.device;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Particle Update Encoder"),
        });

        self.simulator.update(device, queue, &mut encoder, &params);
        // 提交计算命令，否则计算管线永远不会在 GPU 上执行，粒子位置保持不变
        queue.submit([encoder.finish()]);

        let callback_obj = RenderCallback {
            render_pipeline: self.render.render_pipeline.clone(),
            vertex_buffer: self.simulator.get_particle_buffer().clone(),
            vertex_num: self.simulator.get_particle_num(),
        };

        let callback = egui_wgpu::Callback::new_paint_callback(rect, callback_obj);
        ui.painter().add(callback);
    }

    fn get_params(&mut self) -> SimParams {
        let current_time = chrono::offset::Utc::now().timestamp_micros();
        let delta_time = (current_time - self.last_frame_time) as f32 / 1_000_000.;
        self.last_frame_time = current_time;
        self.frame_rate.add(current_time as f64 / 1_000_000.);
        SimParams {
            inner_range: self.config.inner_range,
            outer_range: self.config.outer_range,
            delta_time,
            alpha: self.config.alpha,
            acc_matrix: self.config.acc_matrix,
            ..Default::default()
        }
    }

    fn draw_ui(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::Slider::new(&mut self.config.inner_range, 1.0..=5.0).text("inner range"));
        ui.add(egui::Slider::new(&mut self.config.outer_range, 5.0..=100.0).text("outer range"));
        ui.add(egui::Slider::new(&mut self.config.alpha, 0.0..=1.0).text("alpha"));
        ui.separator();
        ui.add(egui::Slider::new(&mut self.config.particle_num, 256..=8192).text("particle num"));
        ui.add(egui::Slider::new(&mut self.config.cla, 1..=5).text("particle classes"));
        if ui.button("Renew").clicked() {
            self.simulator = LifeGameSimulator::new(
                &self.wgpu_render_state.device,
                self.config.particle_num,
                self.config.cla,
            );
        }
        ui.separator();
        ui.label(format!("fps: {}", self.frame_rate.get_fps()));
    }
}

impl eframe::App for LifeGameApp {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        egui::Window::new("Control Panel").show(ui.ctx(), |ui| {
            self.draw_ui(ui);
        });
        egui::Frame::canvas(ui.style()).show(ui, |ui| {
            self.draw(ui);
        });
        ui.request_repaint();
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        eframe::set_value(storage, "life_game_config", &self.config);
    }

    fn on_exit(&mut self) {}
}
