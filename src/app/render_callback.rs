use egui::PaintCallbackInfo;
use egui_wgpu::{CallbackResources, CallbackTrait};

pub struct RenderCallback {
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_num: u32,
}

unsafe impl Send for RenderCallback {}
unsafe impl Sync for RenderCallback {}

impl CallbackTrait for RenderCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        _callback_resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        Vec::new()
    }

    fn paint(
        &self,
        _info: PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        _callback_resources: &CallbackResources,
    ) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.draw(0..self.vertex_num, 0..1);
    }
}
