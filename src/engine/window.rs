use super::renderer::SceneRenderer;
use crate::Scene;
use glam::Mat4;
use std::sync::Arc;
use winit::{dpi::PhysicalSize, window::Window};

/// ウィンドウとSurfaceのライフサイクルのみを担当する。
pub struct WindowRenderer {
    pub window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    scene: SceneRenderer,
    drawable: bool,
    pub adapter_info: String,
    pub presentation_info: String,
}
impl WindowRenderer {
    pub async fn new(
        window: Arc<Window>,
        mesh: &Scene,
        internal: (u32, u32),
    ) -> Result<Self, String> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| e.to_string())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: false,
            })
            .await
            .ok_or("対応するGPUが見つかりません")?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("M1"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits {
                        max_texture_dimension_2d: adapter
                            .limits()
                            .max_texture_dimension_2d
                            .min(8192),
                        ..wgpu::Limits::downlevel_defaults()
                    },
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| e.to_string())?;
        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("表示Surfaceを構成できません")?;
        config.format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(config.format);
        // WebGPU canvases only support `Fifo`, so the present mode is fixed.
        config.present_mode = wgpu::PresentMode::Fifo;
        let scene = SceneRenderer::new(&device, mesh, internal.0, internal.1, config.format)?;
        let adapter_info = format!("wgpu {:?}", adapter.get_info());
        let presentation_info = format!(
            "surface {:?}, present {:?}, alpha {:?}",
            config.format, config.present_mode, config.alpha_mode
        );
        let mut result = Self {
            window,
            surface,
            device,
            queue,
            config,
            scene,
            drawable: false,
            adapter_info,
            presentation_info,
        };
        result.resize(size)?;
        Ok(result)
    }
    pub fn drawable(&self) -> bool {
        self.drawable
    }
    pub fn internal_size(&self) -> (u32, u32) {
        self.scene.layout().internal_size()
    }
    pub fn resize(&mut self, size: PhysicalSize<u32>) -> Result<(), String> {
        self.scene.resize_display(size.width, size.height);
        self.drawable = false;
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let max = self.device.limits().max_texture_dimension_2d;
        if size.width > max || size.height > max {
            return Err(format!("表示寸法がGPU上限 {max}px を超えています"));
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.drawable = true;
        Ok(())
    }
    pub fn resize_follow_display_aspect(
        &mut self,
        size: PhysicalSize<u32>,
        render_scale: f32,
    ) -> Result<(), String> {
        self.resize(size)?;
        self.scene
            .resize_to_display_aspect(&self.device, size.width, size.height, render_scale)
    }
    pub fn set_internal_size(&mut self, width: u32, height: u32) -> Result<(), String> {
        self.scene.set_internal_size(&self.device, width, height)
    }
    pub fn update_extras(&mut self, world: &Scene, hud: &Scene) {
        self.scene.update_extras(&self.queue, world, hud);
    }
    pub fn update_scene(&mut self, world: &Scene) -> Result<(), String> {
        self.scene.update_scene(&self.queue, world)
    }
    pub fn update_text(&mut self, text: &super::text::TextMesh) {
        self.scene.update_text(&self.device, &self.queue, text);
    }
    pub fn prepare_text(&mut self) {
        self.scene.prepare_text(&self.device, &self.queue);
    }
    pub fn render(&mut self, matrix: Mat4) -> Result<bool, wgpu::SurfaceError> {
        if !self.drawable {
            return Ok(false);
        }
        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.scene.encode_scene(&self.queue, &mut encoder, matrix);
        self.scene.encode_extras(&self.queue, &mut encoder, matrix);
        self.scene.encode_present(&mut encoder, &view);
        self.queue.submit(Some(encoder.finish()));
        self.window.pre_present_notify();
        frame.present();
        Ok(true)
    }
}
