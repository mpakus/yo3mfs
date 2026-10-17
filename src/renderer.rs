use crate::i18n::{Key, Message};
use crate::prepared::Prepared;
use eframe::{egui, egui_wgpu, wgpu};
use std::{ops::Range, sync::Arc};
use wgpu::util::DeviceExt;

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    bind: wgpu::BindGroup,
}
struct GpuMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
}
struct Draw {
    mesh: usize,
    plate: Option<u32>,
    instances: Range<u32>,
}
pub struct GpuScene {
    meshes: Vec<GpuMesh>,
    instances: wgpu::Buffer,
    draws: Vec<Draw>,
    pub bytes: usize,
}

impl Renderer {
    pub fn new(state: &egui_wgpu::RenderState) -> Result<Arc<Self>, Message> {
        let device = &state.device;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("yo3mfs material"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../assets/shaders/model.wgsl").into()),
        });
        let pipeline=device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
            label:Some("yo3mfs geometry"),layout:None,
            vertex:wgpu::VertexState{module:&shader,entry_point:Some("vs_main"),compilation_options:Default::default(),buffers:&[
                Some(wgpu::VertexBufferLayout{array_stride:12,step_mode:wgpu::VertexStepMode::Vertex,attributes:&wgpu::vertex_attr_array![0=>Float32x3]}),
                Some(wgpu::VertexBufferLayout{array_stride:64,step_mode:wgpu::VertexStepMode::Instance,attributes:&wgpu::vertex_attr_array![1=>Float32x4,2=>Float32x4,3=>Float32x4,4=>Float32x4]}),
            ]},
            primitive:wgpu::PrimitiveState{cull_mode:None,..Default::default()},
            depth_stencil:Some(wgpu::DepthStencilState{format:wgpu::TextureFormat::Depth32Float,depth_write_enabled:Some(true),depth_compare:Some(wgpu::CompareFunction::Less),stencil:Default::default(),bias:Default::default()}),
            multisample:wgpu::MultisampleState { count: 4, ..Default::default() },
            fragment:Some(wgpu::FragmentState{module:&shader,entry_point:Some("fs_main"),compilation_options:Default::default(),targets:&[Some(wgpu::ColorTargetState{format:state.target_format,blend:None,write_mask:wgpu::ColorWrites::ALL})]}),
            multiview_mask:None,cache:None,
        });
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera"),
            contents: bytemuck::cast_slice(&glam::Mat4::IDENTITY.to_cols_array()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        if let Some(error) = pollster::block_on(scope.pop()) {
            return Err(Message::with(Key::GpuError, [error]));
        }
        Ok(Arc::new(Self {
            pipeline,
            uniform,
            bind,
        }))
    }
    pub fn upload(device: &wgpu::Device, prepared: &Prepared) -> Result<Arc<GpuScene>, Message> {
        let limit = device.limits().max_buffer_size;
        if prepared.gpu_bytes as u64 > 2 * 1024 * 1024 * 1024 {
            return Err(Key::GpuMemory.into());
        }
        for (mesh, vertices) in prepared.scene.meshes.iter().zip(&prepared.vertices) {
            if (vertices.len() * 12) as u64 > limit || (mesh.triangles.len() * 12) as u64 > limit {
                return Err(Key::GpuBuffer.into());
            }
        }
        if (prepared.instances.len() * 64) as u64 > limit {
            return Err(Key::GpuInstances.into());
        }
        let oom = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut meshes = Vec::new();
        for (mesh, vertices) in prepared.scene.meshes.iter().zip(&prepared.vertices) {
            meshes.push(GpuMesh {
                vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("mesh vertices"),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
                indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("mesh indices"),
                    contents: bytemuck::cast_slice(&mesh.triangles),
                    usage: wgpu::BufferUsages::INDEX,
                }),
                count: mesh.triangles.len() as u32 * 3,
            });
        }
        let matrices = prepared
            .instances
            .iter()
            .map(|i| i.matrix)
            .collect::<Vec<_>>();
        let instances = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("instances"),
            contents: bytemuck::cast_slice(&matrices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let mut draws: Vec<Draw> = Vec::new();
        for (n, i) in prepared.instances.iter().enumerate() {
            if let Some(last) = draws
                .last_mut()
                .filter(|d| d.mesh == i.mesh && d.plate == i.plate)
            {
                last.instances.end += 1;
            } else {
                draws.push(Draw {
                    mesh: i.mesh,
                    plate: i.plate,
                    instances: n as u32..n as u32 + 1,
                });
            }
        }
        let error = pollster::block_on(validation.pop());
        let memory_error = pollster::block_on(oom.pop());
        if let Some(error) = error.or(memory_error) {
            return Err(Message::with(Key::GpuError, [error]));
        }
        Ok(Arc::new(GpuScene {
            meshes,
            instances,
            draws,
            bytes: prepared.gpu_bytes,
        }))
    }
    pub fn callback(
        self: &Arc<Self>,
        rect: egui::Rect,
        scene: Arc<GpuScene>,
        matrix: glam::Mat4,
        plate: Option<u32>,
    ) -> egui::PaintCallback {
        egui_wgpu::Callback::new_paint_callback(
            rect,
            Paint {
                renderer: self.clone(),
                scene,
                matrix: matrix.to_cols_array(),
                plate,
            },
        )
    }
}
struct Paint {
    renderer: Arc<Renderer>,
    scene: Arc<GpuScene>,
    matrix: [f32; 16],
    plate: Option<u32>,
}
impl egui_wgpu::CallbackTrait for Paint {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        _resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        queue.write_buffer(
            &self.renderer.uniform,
            0,
            bytemuck::cast_slice(&self.matrix),
        );
        vec![]
    }
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        _resources: &egui_wgpu::CallbackResources,
    ) {
        pass.set_pipeline(&self.renderer.pipeline);
        pass.set_bind_group(0, &self.renderer.bind, &[]);
        pass.set_vertex_buffer(1, self.scene.instances.slice(..));
        for draw in &self.scene.draws {
            if self.plate.is_some() && draw.plate != self.plate {
                continue;
            }
            let mesh = &self.scene.meshes[draw.mesh];
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.count, 0, draw.instances.clone());
        }
    }
}
