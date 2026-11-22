//! الرسم عبر wgpu: خط أنابيب للعالم (مع عمق وضباب بسيط) وآخر للـ HUD (مستطيلات شفافة بلا عمق).
use crate::world::{Mesh, Vertex};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const SKY: wgpu::Color = wgpu::Color { r: 0.45, g: 0.68, b: 0.92, a: 1.0 };
const HUD_CAPACITY: usize = 60_000; // رؤوس

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct HudVertex {
    pub pos: [f32; 2],
    pub color: [f32; 4],
}

const WORLD_WGSL: &str = r#"
struct Globals { view_proj: mat4x4<f32> };
@group(0) @binding(0) var<uniform> g: Globals;
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) dist: f32,
};
@vertex
fn vs_main(@location(0) p: vec3<f32>, @location(1) c: vec3<f32>) -> VsOut {
    var o: VsOut;
    o.pos = g.view_proj * vec4<f32>(p, 1.0);
    o.color = c;
    o.dist = o.pos.w;
    return o;
}
@fragment
fn fs_main(i: VsOut) -> @location(0) vec4<f32> {
    let fog = clamp((i.dist - 70.0) / 90.0, 0.0, 1.0);
    let sky = vec3<f32>(0.45, 0.68, 0.92);
    return vec4<f32>(mix(i.color, sky, fog), 1.0);
}
"#;

const HUD_WGSL: &str = r#"
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) color: vec4<f32> };
@vertex
fn vs_main(@location(0) p: vec2<f32>, @location(1) c: vec4<f32>) -> VsOut {
    var o: VsOut;
    o.pos = vec4<f32>(p, 0.0, 1.0);
    o.color = c;
    return o;
}
@fragment
fn fs_main(i: VsOut) -> @location(0) vec4<f32> { return i.color; }
"#;

const WORLD_ATTRS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
const HUD_ATTRS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    world_pipeline: wgpu::RenderPipeline,
    hud_pipeline: wgpu::RenderPipeline,
    globals_buf: wgpu::Buffer,
    globals_bind: wgpu::BindGroup,
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    index_count: u32,
    hud_buf: wgpu::Buffer,
    hud_count: u32,
    depth: wgpu::TextureView,
}

fn make_depth(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

impl Renderer {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat, size: (u32, u32), mesh: &Mesh) -> Renderer {
        let globals_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let globals_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals bind"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals_buf.as_entire_binding() }],
        });

        let world_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("world"),
            source: wgpu::ShaderSource::Wgsl(WORLD_WGSL.into()),
        });
        let hud_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hud"),
            source: wgpu::ShaderSource::Wgsl(HUD_WGSL.into()),
        });
        let world_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("world layout"),
            bind_group_layouts: &[&globals_layout],
            push_constant_ranges: &[],
        });
        let hud_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("hud layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let world_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("world"),
            layout: Some(&world_layout),
            vertex: wgpu::VertexState {
                module: &world_shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &WORLD_ATTRS,
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &world_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let hud_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hud"),
            layout: Some(&hud_layout),
            vertex: wgpu::VertexState {
                module: &hud_shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<HudVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &HUD_ATTRS,
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &hud_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("world vertices"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("world indices"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let hud_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hud"),
            size: (HUD_CAPACITY * std::mem::size_of::<HudVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let depth = make_depth(&device, size.0, size.1);
        Renderer {
            device,
            queue,
            world_pipeline,
            hud_pipeline,
            globals_buf,
            globals_bind,
            vbuf,
            ibuf,
            index_count: mesh.indices.len() as u32,
            hud_buf,
            hud_count: 0,
            depth,
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.depth = make_depth(&self.device, w, h);
    }

    pub fn update(&mut self, view_proj: [[f32; 4]; 4], hud: &[HudVertex]) {
        self.queue.write_buffer(&self.globals_buf, 0, bytemuck::bytes_of(&Globals { view_proj }));
        let n = hud.len().min(HUD_CAPACITY) / 6 * 6;
        self.hud_count = n as u32;
        if n > 0 {
            self.queue.write_buffer(&self.hud_buf, 0, bytemuck::cast_slice(&hud[..n]));
        }
    }

    pub fn render(&self, target: &wgpu::TextureView) {
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(SKY), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.world_pipeline);
            pass.set_bind_group(0, &self.globals_bind, &[]);
            pass.set_vertex_buffer(0, self.vbuf.slice(..));
            pass.set_index_buffer(self.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.index_count, 0, 0..1);
            if self.hud_count > 0 {
                pass.set_pipeline(&self.hud_pipeline);
                pass.set_vertex_buffer(0, self.hud_buf.slice(..));
                pass.draw(0..self.hud_count, 0..1);
            }
        }
        self.queue.submit(Some(enc.finish()));
    }
}
