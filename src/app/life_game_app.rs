use eframe::Storage;
use egui::{Pos2, Rect, Vec2};
use web_sys::js_sys::Math::random;

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
    current_cla: usize,
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
            render: LifeGameRenderer::new(device, &surface_format, &config.colors),
            simulator,
            config,
            wgpu_render_state: wgpu_render_state.clone(),
            frame_rate: FrameRate::new(100),
            last_frame_time: chrono::offset::Utc::now().timestamp_micros(),
            current_cla: config.cla
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
        // 每帧把最新的颜色数组写回 GPU uniform buffer，使颜色编辑实时生效
        queue.write_buffer(
            &self.render.color_array_buffer,
            0,
            bytemuck::cast_slice(&self.config.colors),
        );
        // 提交计算命令，否则计算管线永远不会在 GPU 上执行，粒子位置保持不变
        queue.submit([encoder.finish()]);

        let callback_obj = RenderCallback {
            render_pipeline: self.render.render_pipeline.clone(),
            vertex_buffer: self.simulator.get_particle_buffer().clone(),
            color_array_bind_group: self.render.color_array_bind_group.clone(),
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
        // 把 config 的紧密 [f32;25] 展开成 stride=16 的 [[f32;4];25]（.x 存值）
        let mut acc_matrix = [[0.0f32; 4]; 25];
        for (i, v) in self.config.acc_matrix.iter().enumerate() {
            acc_matrix[i][0] = *v;
        }
        SimParams {
            inner_range: self.config.inner_range,
            outer_range: self.config.outer_range,
            delta_time,
            alpha: self.config.alpha,
            acc_matrix,
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
            self.current_cla = self.config.cla;
        }

        // 每类粒子一个颜色选择器（只显示当前粒子类数个）
        ui.separator();
        if ui.button("Randomize Colors").clicked() {
            for c in self.config.colors[..self.current_cla].iter_mut() {
                c[0] = random() as f32;
                c[1] = random() as f32;
                c[2] = random() as f32;
                c[3] = 1.0;
            }
        }
        for i in 0..self.current_cla {
            let c = self.config.colors[i];
            let mut c32 = egui::Color32::from_rgba_unmultiplied(
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                (c[3] * 255.0) as u8,
            );
            ui.horizontal(|ui| {
                ui.label(format!("particle {}", i));
                if ui.color_edit_button_srgba(&mut c32).changed() {
                    self.config.colors[i] = [
                        c32.r() as f32 / 255.0,
                        c32.g() as f32 / 255.0,
                        c32.b() as f32 / 255.0,
                        c32.a() as f32 / 255.0,
                    ];
                }
            });
        }
        ui.separator();
        self.draw_matrix_ui(ui);
        ui.separator();
        ui.label(format!("fps: {}", self.frame_rate.get_fps()));
    }

    // 绘制 5×5 的相互作用矩阵：行=自己、列=对方，与 compute.wgsl 的
    // acc_matrix[p_type * 5 + other_type] 索引保持一致。
    // 有效格子数为 current_cla × current_cla，超出的行列渲染为灰色且不可交互。
    // 滚动滚轮以 0.01 为步长在 [-1, 1] 区间内修改对应矩阵项，实时写回 config。
    fn draw_matrix_ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Attraction Matrix (row = self, col = other)");
        if ui.button("Randomize Matrix").clicked() {
            self.config.randomize_matrix();
        }

        let cell = egui::vec2(46.0, 30.0);
        let gap = 4.0;
        // 行列标签色块的尺寸（行标签列宽 / 列标签行高）
        let label_size = 18.0;
        let grid_size = egui::vec2(
            5.0 * cell.x + 4.0 * gap,
            5.0 * cell.y + 4.0 * gap,
        );
        // 整体区域 = 左侧行标签列 + 顶部列标签行 + 网格本体
        let total_size = egui::vec2(
            label_size + gap + grid_size.x,
            label_size + gap + grid_size.y,
        );

        // 一次性占位整块区域，后续画在这个 rect 内
        let (grid_rect, _) = ui.allocate_exact_size(total_size, egui::Sense::hover());
        let painter = ui.painter();
        let hover_pos = ui.ctx().pointer_hover_pos();

        // 网格本体（5×5 格子）的原点：跳过标签区
        let grid_origin = grid_rect.min + egui::vec2(label_size + gap, label_size + gap);

        // 先统计本次帧滚轮的"步数"（一格滚轮 = 1 步 = 0.01）
        let mut wheel_steps = 0.0f32;
        ui.ctx().input(|i| {
            for e in &i.events {
                if let egui::Event::MouseWheel { unit, delta, .. } = e {
                    wheel_steps += match unit {
                        egui::MouseWheelUnit::Point => delta.y / 50.0,
                        egui::MouseWheelUnit::Line => delta.y,
                        egui::MouseWheelUnit::Page => delta.y * 4.0,
                    };
                }
            }
        });

        // 找出当前悬停的格子（若在有效区域内）
        let mut hovered_idx: Option<usize> = None;
        if let Some(p) = hover_pos {
            let gx = p.x - grid_origin.x;
            let gy = p.y - grid_origin.y;
            if gx >= 0.0 && gy >= 0.0 && gx < grid_size.x && gy < grid_size.y {
                let col = (gx / (cell.x + gap)).floor() as i32;
                let row = (gy / (cell.y + gap)).floor() as i32;
                if (0..5).contains(&row) && (0..5).contains(&col) {
                    let r = row as usize;
                    let c = col as usize;
                    if r < self.current_cla && c < self.current_cla {
                        hovered_idx = Some(r * 5 + c);
                    }
                }
            }
        }

        // 应用滚轮修改
        if wheel_steps != 0.0 {
            if let Some(idx) = hovered_idx {
                let v = &mut self.config.acc_matrix[idx];
                *v = (*v + wheel_steps * 0.01).clamp(-1.0, 1.0);
            }
        }

        // 绘制顶部列标签 + 左侧行标签（色块表示对应粒子类型的颜色）
        let type_color = |t: usize| -> egui::Color32 {
            let c = self.config.colors[t];
            egui::Color32::from_rgba_unmultiplied(
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                (c[3] * 255.0) as u8,
            )
        };
        for col in 0..5 {
            let r = egui::Rect::from_min_size(
                grid_origin + egui::vec2(col as f32 * (cell.x + gap), -(label_size + gap)),
                egui::vec2(cell.x, label_size),
            );
            let fill = if col < self.current_cla {
                type_color(col)
            } else {
                egui::Color32::from_gray(55)
            };
            painter.rect_filled(r, 3.0, fill);
            painter.rect_stroke(r, 3.0, egui::Stroke::new(1.0, egui::Color32::from_gray(90)), egui::StrokeKind::Inside);
        }
        for row in 0..5 {
            let r = egui::Rect::from_min_size(
                grid_origin + egui::vec2(-(label_size + gap), row as f32 * (cell.y + gap)),
                egui::vec2(label_size, cell.y),
            );
            let fill = if row < self.current_cla {
                type_color(row)
            } else {
                egui::Color32::from_gray(55)
            };
            painter.rect_filled(r, 3.0, fill);
            painter.rect_stroke(r, 3.0, egui::Stroke::new(1.0, egui::Color32::from_gray(90)), egui::StrokeKind::Inside);
        }

        // 逐个绘制格子
        for row in 0..5 {
            for col in 0..5 {
                let idx = row * 5 + col;
                let active = row < self.current_cla && col < self.current_cla;
                let hovered = hovered_idx == Some(idx);
                let val = self.config.acc_matrix[idx];

                let min = grid_origin
                    + egui::vec2(
                        col as f32 * (cell.x + gap),
                        row as f32 * (cell.y + gap),
                    );
                let cell_rect = egui::Rect::from_min_size(min, cell);

                // 热力图：按值正负/强弱插值填充底色
                // 正值（吸引）→ 暖橙红；负值（排斥）→ 冷蓝；0 → 中性深灰
                let fill = if !active {
                    egui::Color32::from_gray(55)
                } else {
                    let t = val.clamp(-1.0, 1.0).abs();
                    let neutral = 35.0f32;
                    let (tr, tg, tb) = if val >= 0.0 {
                        (210.0f32, 90.0, 40.0) // 暖
                    } else {
                        (50.0f32, 110.0, 220.0) // 冷
                    };
                    let r = neutral + (tr - neutral) * t;
                    let g = neutral + (tg - neutral) * t;
                    let b = neutral + (tb - neutral) * t;
                    egui::Color32::from_rgb(r as u8, g as u8, b as u8)
                };

                // 文字颜色按底色亮度切换，保证可读性
                let text_color = if !active {
                    egui::Color32::from_gray(130)
                } else {
                    let lum = (fill.r() as u32 + fill.g() as u32 + fill.b() as u32) / 3;
                    if lum > 130 {
                        egui::Color32::from_gray(20)
                    } else {
                        egui::Color32::WHITE
                    }
                };

                let stroke = if hovered {
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(235, 180, 60))
                } else if !active {
                    egui::Stroke::new(1.0, egui::Color32::from_gray(70))
                } else {
                    egui::Stroke::new(1.0, egui::Color32::from_gray(90))
                };

                painter.rect_filled(cell_rect, 3.0, fill);
                painter.rect_stroke(cell_rect, 3.0, stroke, egui::StrokeKind::Inside);

                let text = format!("{:+.2}", val);
                painter.text(
                    cell_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::monospace(13.0),
                    text_color,
                );
            }
        }
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
