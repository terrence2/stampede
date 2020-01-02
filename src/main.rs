// This file is part of Arctic.
//
// Arctic is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Arctic is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Arctic.  If not, see <http://www.gnu.org/licenses/>.
mod tree;

use crate::tree::Tree;
use gpu::GPU;
use failure::Fallible;
use rand::prelude::*;
use std::mem;
use wgpu;
use winit::{
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(AsBytes, FromBytes, Copy, Clone, Debug, Default)]
pub struct Vertex {
    position: [f32; 2],
    tex_coord: [f32; 2],
}

fn main() -> Fallible<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop)?;
    let mut gpu = GPU::new(&window, Default::default())?;

    // Compute Resources
    let uni_shader = gpu.create_shader_module(include_bytes!("../target/uni_shader.comp.spirv"))?;
    let uni_shader_layout =
        gpu.device()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    wgpu::BindGroupLayoutBinding {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            dimension: wgpu::TextureViewDimension::D2,
                        },
                    },
                    wgpu::BindGroupLayoutBinding {
                        binding: 1,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                    },
                ],
            });
    let uni_shader_pipeline =
        gpu.device()
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                layout: &gpu
                    .device()
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        bind_group_layouts: &[&uni_shader_layout],
                    }),
                compute_stage: wgpu::ProgrammableStageDescriptor {
                    module: &uni_shader,
                    entry_point: "main",
                },
            });
    let instr_buffer_size = mem::size_of::<[i32; 1024]>() as wgpu::BufferAddress;
    let instr_buffer = gpu.device().create_buffer(&wgpu::BufferDescriptor {
        size: instr_buffer_size,
        usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::MAP_READ,
    });
    // TODO: make configurable
    let result_extent = wgpu::Extent3d {
        width: 1920,
        height: 1080,
        depth: 1,
    };
    let result_texture = gpu.device().create_texture(&wgpu::TextureDescriptor {
        size: result_extent,
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsage::all(),
    });
    let result_texture_view = result_texture.create_view(&wgpu::TextureViewDescriptor {
        format: wgpu::TextureFormat::Rgba32Float,
        dimension: wgpu::TextureViewDimension::D2,
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        level_count: 1, // mip level
        base_array_layer: 0,
        array_layer_count: 1,
    });
    let result_texture_sampler = gpu.device().create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        lod_min_clamp: 0f32,
        lod_max_clamp: 9_999_999f32,
        compare_function: wgpu::CompareFunction::Never,
    });
    let uni_shader_bind_group = gpu.device().create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &uni_shader_layout,
        bindings: &[
            wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&result_texture_view),
            },
            wgpu::Binding {
                binding: 1,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &instr_buffer,
                    range: 0..instr_buffer_size,
                },
            },
        ],
    });

    // Screen Resources
    let graphics_layout = gpu
        .device()
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[
                wgpu::BindGroupLayoutBinding {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        multisampled: true,
                        dimension: wgpu::TextureViewDimension::D2,
                    },
                },
                wgpu::BindGroupLayoutBinding {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler,
                },
            ],
        });
    let vert_shader = gpu.create_shader_module(include_bytes!("../target/draw.vert.spirv"))?;
    let frag_shader = gpu.create_shader_module(include_bytes!("../target/draw.frag.spirv"))?;
    let graphics_pipeline = gpu
        .device()
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &gpu
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    bind_group_layouts: &[&graphics_layout],
                }),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vert_shader,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &frag_shader,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Back,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleStrip,
            color_states: &[wgpu::ColorStateDescriptor {
                format: GPU::texture_format(),
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                format: GPU::DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_read_mask: 0,
                stencil_write_mask: 0,
            }),
            index_format: wgpu::IndexFormat::Uint32,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float2,
                        offset: 8,
                        shader_location: 1,
                    },
                ],
            }],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });
    let verts = [
        Vertex {
            position: [-1f32, -1f32],
            tex_coord: [0f32, 0f32],
        },
        Vertex {
            position: [-1f32, 1f32],
            tex_coord: [0f32, 1f32],
        },
        Vertex {
            position: [1f32, -1f32],
            tex_coord: [1f32, 0f32],
        },
        Vertex {
            position: [1f32, 1f32],
            tex_coord: [1f32, 1f32],
        },
    ];
    let vertex_buffer = gpu
        .device()
        .create_buffer_mapped(verts.len(), wgpu::BufferUsage::all())
        .fill_from_slice(&verts);
    let graphics_bind_group = gpu.device().create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &graphics_layout,
        bindings: &[
            wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&result_texture_view),
            },
            wgpu::Binding {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&result_texture_sampler),
            },
        ],
    });

    let mut rng = thread_rng();
    let tree = Tree::new(&mut rng);
    println!("tree: {}", tree.show());

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::EventsCleared => {
                // Application update code.

                // Queue a RedrawRequested event.
                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                // Redraw the application.
                //
                // It's preferrable to render in this event rather than in EventsCleared, since
                // rendering in here allows the program to gracefully handle redraws requested
                // by the OS.
                let mut frame = gpu.begin_frame().unwrap();
                {
                    let mut cpass = frame.begin_compute_pass();
                    cpass.set_pipeline(&uni_shader_pipeline);
                    cpass.set_bind_group(0, &uni_shader_bind_group, &[]);
                    cpass.dispatch(result_extent.width / 8, result_extent.height / 8, 1);
                }
                {
                    let mut rpass = frame.begin_render_pass();
                    rpass.set_pipeline(&graphics_pipeline);
                    rpass.set_bind_group(0, &graphics_bind_group, &[]);
                    rpass.set_vertex_buffers(0, &[(&vertex_buffer, 0)]);
                    rpass.draw(0..4, 0..1);
                }
                frame.finish();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                println!("The close button was pressed; stopping");
                *control_flow = ControlFlow::Exit
            }
            Event::WindowEvent {
                event: WindowEvent::Destroyed,
                ..
            } => {
                println!("The window was destroyed; stopping");
                *control_flow = ControlFlow::Exit
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                gpu.note_resize(&window);
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    },
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Q),
                                ..
                            },
                        ..
                    },
                ..
            } => *control_flow = ControlFlow::Exit,
            // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
            // dispatched any events. This is ideal for games and similar applications.
            _ => *control_flow = ControlFlow::Poll,
            // ControlFlow::Wait pauses the event loop if no events are available to process.
            // This is ideal for non-game applications that only update in response to user
            // input, and uses significantly less power/CPU time than ControlFlow::Poll.
            // _ => *control_flow = ControlFlow::Wait,
        }
    });
}
