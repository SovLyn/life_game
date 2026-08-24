use egui::{Pos2, Rect, Vec2};

use super::config::Config;
use super::life_game_renderer::LifeGameRenderer;
use super::render_callback::RenderCallback;
// 定义LifeGameApp结构体，用于管理生命游戏的显示和交互
pub struct LifeGameApp {
    render: LifeGameRenderer,
    // 游戏配置，包含各种参数
    config: Config,
    surface_format: wgpu::TextureFormat,
}

impl LifeGameApp {
    // 构造函数，用于创建LifeGameApp实例
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 获取wgpu渲染状态
        let wgpu_render_state = cc
            .wgpu_render_state
            .as_ref()
            .expect("This app requires the wgpu render state");
        let surface_format = wgpu_render_state.target_format;

        let device = &wgpu_render_state.device;
        // 创建并返回LifeGameApp实例
        Self {
            render: LifeGameRenderer::new(device, &surface_format),
            config: Config::new(),
            surface_format,
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

        let callback_obj = RenderCallback {
            render_pipeline: self.render.render_pipeline.clone(),
            vertex_buffer: self.render.vertex_buffer.clone(),
            vertex_num: 500,
        };

        let callback = egui_wgpu::Callback::new_paint_callback(rect, callback_obj);
        ui.painter().add(callback);
    }
}

impl eframe::App for LifeGameApp {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        egui::Window::new("Control Panel").show(ui.ctx(), |ui| {
            ui.add(
                egui::Slider::new(&mut self.config.point_size, 0.0..=100.0).text("particle size"),
            );
        });
        egui::Frame::canvas(ui.style()).show(ui, |ui| {
            self.draw(ui);
        });
        ui.request_repaint();
    }

    fn on_exit(&mut self) {}
}
