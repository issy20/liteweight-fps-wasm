use super::layout::FrameLayout;
use super::{text::TextMesh, text_render::TextRenderer};
use crate::{Scene, Vertex};
use glam::Mat4;
use std::borrow::Cow;
use wgpu::util::DeviceExt;

fn aligned_indices(indices: &[u16]) -> Cow<'_, [u16]> {
    if indices.len().is_multiple_of(2) {
        Cow::Borrowed(indices)
    } else {
        let mut padded = Vec::with_capacity(indices.len() + 1);
        padded.extend_from_slice(indices);
        padded.push(*indices.last().expect("odd index list is non-empty"));
        Cow::Owned(padded)
    }
}

struct Targets {
    color: wgpu::Texture,
    color_view: wgpu::TextureView,
    _depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    blit_bind: wgpu::BindGroup,
}
impl Targets {
    fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, width: u32, height: u32) -> Self {
        let descriptor = wgpu::TextureDescriptor {
            label: Some("M1 color"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        };
        let color = device.create_texture(&descriptor);
        let color_view = color.create_view(&Default::default());
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("M1 depth"),
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            ..descriptor
        });
        let depth_view = depth.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let blit_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("M1 presentation"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        Self {
            color,
            color_view,
            _depth: depth,
            depth_view,
            blit_bind,
        }
    }
}

/// デバイス・シーンと行列だけを使う描画処理。ゲーム状態・入力・計測器には依存しない。
pub struct SceneRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex: wgpu::Buffer,
    index: wgpu::Buffer,
    uniform: wgpu::Buffer,
    bind: wgpu::BindGroup,
    blit_pipeline: wgpu::RenderPipeline,
    blit_layout: wgpu::BindGroupLayout,
    targets: Targets,
    count: u32,
    layout: FrameLayout,
    dynamic: DynamicMesh,
    hud: DynamicMesh,
    hud_pipeline: wgpu::RenderPipeline,
    text: Option<TextRenderer>,
}
impl SceneRenderer {
    pub fn new(
        device: &wgpu::Device,
        mesh: &Scene,
        width: u32,
        height: u32,
        output_format: wgpu::TextureFormat,
    ) -> Result<Self, String> {
        let layout_state = FrameLayout::new(width, height)?;
        Self::check_size(device, width, height)?;
        if mesh.vertices.is_empty()
            || mesh.indices.is_empty()
            || mesh
                .indices
                .iter()
                .any(|i| usize::from(*i) >= mesh.vertices.len())
        {
            return Err("シーンの頂点・インデックスが不正です".into());
        }
        let vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scene vertices"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scene indices"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../assets/shaders/scene_wall.wgsl").into(),
            ),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("M1 scene"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let hud_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("HUD"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../assets/shaders/scene.wgsl").into(),
            ),
        });
        let hud_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("HUD"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &hud_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &hud_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let dynamic = DynamicMesh::new(device, &layout);
        let hud = DynamicMesh::new(device, &layout);
        let blit_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let targets = Targets::new(device, &blit_layout, width, height);
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blit"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../assets/shaders/blit.wgsl").into()),
        });
        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&blit_layout],
            push_constant_ranges: &[],
        });
        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit"),
            layout: Some(&blit_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: output_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        Ok(Self {
            pipeline,
            vertex,
            index,
            uniform,
            bind,
            blit_pipeline,
            blit_layout,
            targets,
            count: mesh.indices.len() as u32,
            layout: layout_state,
            dynamic,
            hud,
            hud_pipeline,
            text: None,
        })
    }
    fn check_size(device: &wgpu::Device, width: u32, height: u32) -> Result<(), String> {
        let max = device.limits().max_texture_dimension_2d;
        if width > max || height > max {
            return Err(format!("GPUの最大テクスチャ寸法 {max} を超えています"));
        }
        Ok(())
    }
    pub fn layout(&self) -> &FrameLayout {
        &self.layout
    }
    pub fn update_scene(&mut self, queue: &wgpu::Queue, mesh: &Scene) -> Result<(), String> {
        if mesh.vertices.is_empty()
            || mesh.indices.is_empty()
            || mesh
                .indices
                .iter()
                .any(|index| usize::from(*index) >= mesh.vertices.len())
        {
            return Err("シーンの頂点・インデックスが不正です".into());
        }
        let upload_indices = aligned_indices(&mesh.indices);
        if bytemuck::cast_slice::<Vertex, u8>(&mesh.vertices).len() as u64 > self.vertex.size()
            || bytemuck::cast_slice::<u16, u8>(upload_indices.as_ref()).len() as u64
                > self.index.size()
        {
            return Err("更新後のシーンがGPUバッファ容量を超えています".into());
        }
        queue.write_buffer(&self.vertex, 0, bytemuck::cast_slice(&mesh.vertices));
        queue.write_buffer(
            &self.index,
            0,
            bytemuck::cast_slice(upload_indices.as_ref()),
        );
        self.count = mesh.indices.len() as u32;
        Ok(())
    }
    pub fn resize_display(&mut self, width: u32, height: u32) {
        self.layout.resize(width, height);
    }
    pub fn resize_to_display_aspect(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Result<(), String> {
        let mut next = self.layout;
        next.resize_to_display_scale(width, height, scale)
            .map_err(str::to_owned)?;
        let (internal_width, internal_height) = next.internal_size();
        Self::check_size(device, internal_width, internal_height)?;
        if self.layout.internal_size() != (internal_width, internal_height) {
            self.targets = Targets::new(device, &self.blit_layout, internal_width, internal_height);
        }
        self.layout = next;
        Ok(())
    }
    pub fn set_internal_size(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        let mut next = self.layout;
        next.set_internal(width, height)?;
        Self::check_size(device, width, height)?;
        if self.layout.internal_size() != (width, height) {
            let targets = Targets::new(device, &self.blit_layout, width, height);
            self.targets = targets;
            self.layout = next;
        }
        Ok(())
    }
    pub fn color_texture(&self) -> &wgpu::Texture {
        &self.targets.color
    }
    pub fn encode_scene(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        matrix: Mat4,
    ) {
        queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::cast_slice(&matrix.to_cols_array()),
        );
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("M1 scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.targets.color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.08,
                        g: 0.10,
                        b: 0.14,
                        a: 1.,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.targets.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.set_vertex_buffer(0, self.vertex.slice(..));
        pass.set_index_buffer(self.index.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..self.count, 0, 0..1);
    }
    /// 小規模な標的・短命エフェクトのみを固定容量のGPUバッファへ転送する。
    pub fn update_extras(&mut self, queue: &wgpu::Queue, world: &Scene, hud: &Scene) {
        self.dynamic.upload(queue, world);
        self.hud.upload(queue, hud);
    }
    pub fn prepare_text(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.text.get_or_insert_with(|| {
            TextRenderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm)
        });
    }
    pub fn update_text(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, text: &TextMesh) {
        if !text.indices.is_empty() {
            self.prepare_text(device, queue);
        }
        if let Some(renderer) = &mut self.text {
            renderer.update(queue, text);
        }
    }
    pub fn encode_extras(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        matrix: Mat4,
    ) {
        for (mesh, hud) in [(&self.dynamic, false), (&self.hud, true)] {
            if mesh.count == 0 {
                continue;
            }
            let matrix = if hud { Mat4::IDENTITY } else { matrix };
            queue.write_buffer(
                &mesh.uniform,
                0,
                bytemuck::cast_slice(&matrix.to_cols_array()),
            );
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("M3 extras"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: if hud {
                    None
                } else {
                    Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.targets.depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    })
                },
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(if hud {
                &self.hud_pipeline
            } else {
                &self.pipeline
            });
            pass.set_bind_group(0, &mesh.bind, &[]);
            pass.set_vertex_buffer(0, mesh.vertex.slice(..));
            pass.set_index_buffer(mesh.index.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..mesh.count, 0, 0..1);
        }
        if let Some(text) = &self.text {
            text.encode(encoder, &self.targets.color_view);
        }
    }
    pub fn encode_present(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let Some((x, y, w, h)) = self.layout.viewport() else {
            return;
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("M1 present"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_viewport(x as f32, y as f32, w as f32, h as f32, 0., 1.);
        pass.set_pipeline(&self.blit_pipeline);
        pass.set_bind_group(0, &self.targets.blit_bind, &[]);
        pass.draw(0..3, 0..1);
    }
}

struct DynamicMesh {
    vertex: wgpu::Buffer,
    index: wgpu::Buffer,
    uniform: wgpu::Buffer,
    bind: wgpu::BindGroup,
    count: u32,
}
impl DynamicMesh {
    fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let buffer = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let vertex = buffer(
            "dynamic vertices",
            8192 * std::mem::size_of::<Vertex>() as u64,
            wgpu::BufferUsages::VERTEX,
        );
        let index = buffer("dynamic indices", 32768 * 2, wgpu::BufferUsages::INDEX);
        let uniform = buffer("dynamic matrix", 64, wgpu::BufferUsages::UNIFORM);
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        Self {
            vertex,
            index,
            uniform,
            bind,
            count: 0,
        }
    }
    fn upload(&mut self, queue: &wgpu::Queue, mesh: &Scene) {
        assert!(mesh.vertices.len() <= 8192 && mesh.indices.len() <= 32768);
        self.count = mesh.indices.len() as u32;
        if self.count > 0 {
            let upload_indices = aligned_indices(&mesh.indices);
            queue.write_buffer(&self.vertex, 0, bytemuck::cast_slice(&mesh.vertices));
            queue.write_buffer(
                &self.index,
                0,
                bytemuck::cast_slice(upload_indices.as_ref()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::aligned_indices;
    use std::borrow::Cow;

    #[test]
    fn index_uploads_pad_odd_u16_counts_to_copy_alignment() {
        let indices = [2, 5, 7];
        assert_eq!(aligned_indices(&indices).as_ref(), &[2, 5, 7, 7]);
    }

    #[test]
    fn index_uploads_borrow_already_aligned_data() {
        let indices = [2, 5, 7, 9];
        assert!(matches!(aligned_indices(&indices), Cow::Borrowed(_)));
    }
}
