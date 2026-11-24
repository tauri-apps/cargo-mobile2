use crate::wgpu_renderer::render_pipeline::MyRenderPipeline;
use shaders::ShaderConstants;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::wgt::CommandEncoderDescriptor;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferUsages, Color, Device, LoadOp, Operations, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, ShaderStages, StoreOp, TextureFormat, TextureView,
};

pub struct MyRenderer {
    pub device: Device,
    pub queue: Queue,
    global_bind_group_layout: GlobalBindGroupLayout,
    pipeline: MyRenderPipeline,
}

impl MyRenderer {
    pub fn new(device: Device, queue: Queue, out_format: TextureFormat) -> anyhow::Result<Self> {
        let global_bind_group_layout = GlobalBindGroupLayout::new(&device);
        let pipeline = MyRenderPipeline::new(&device, &global_bind_group_layout, out_format)?;
        Ok(Self {
            global_bind_group_layout,
            pipeline,
            device,
            queue,
        })
    }

    pub fn render(
        &self,
        shader_constants: &ShaderConstants,
        output: TextureView,
    ) -> anyhow::Result<()> {
        let global_bind_group = self
            .global_bind_group_layout
            .create(&self.device, shader_constants);

        let mut cmd = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("main draw"),
            });

        let mut rpass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some("main renderpass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &output,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        self.pipeline.draw(&mut rpass, &global_bind_group);
        drop(rpass);

        self.queue.submit(std::iter::once(cmd.finish()));
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct GlobalBindGroupLayout(pub BindGroupLayout);

impl GlobalBindGroupLayout {
    pub fn new(device: &Device) -> Self {
        Self(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("GlobalBindGroupLayout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        }))
    }

    pub fn create(&self, device: &Device, shader_constants: &ShaderConstants) -> GlobalBindGroup {
        let shader_constants = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("ShaderConstants"),
            contents: bytemuck::bytes_of(shader_constants),
            usage: BufferUsages::STORAGE,
        });
        self.create_from_buffer(device, &shader_constants)
    }

    pub fn create_from_buffer(
        &self,
        device: &Device,
        shader_constants: &Buffer,
    ) -> GlobalBindGroup {
        GlobalBindGroup(device.create_bind_group(&BindGroupDescriptor {
            label: Some("GlobalBindGroup"),
            layout: &self.0,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: shader_constants,
                    offset: 0,
                    size: None,
                }),
            }],
        }))
    }
}

#[derive(Debug, Clone)]
pub struct GlobalBindGroup(pub BindGroup);
