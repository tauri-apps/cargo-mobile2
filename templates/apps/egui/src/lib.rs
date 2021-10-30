use std::iter;
use std::time::Instant;

use epi::*;
use winit::event_loop::ControlFlow;
use winit::event::{Event::*};
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};

use mobile_entry_point::mobile_entry_point;
#[cfg(target_os = "android")]
use ndk_glue;

/// A custom event type for the winit app.
#[derive(Debug)]
enum Event {
    RequestRedraw,
}

static INITIAL_WIDTH: u32 = 1280;
static INITIAL_HEIGHT: u32 = 720;

/// This is the repaint signal type that egui needs for requesting a repaint from another thread.
/// It sends the custom RequestRedraw event to the winit event loop.
struct ExampleRepaintSignal(std::sync::Mutex<winit::event_loop::EventLoopProxy<Event>>);

impl epi::RepaintSignal for ExampleRepaintSignal {
    fn request_repaint(&self) {
        self.0.lock().unwrap().send_event(Event::RequestRedraw).ok();
    }
}

#[cfg(target_os = "android")]
fn init_logging() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_min_level(log::Level::Trace)
            .with_tag("{{app.name}}"),
    );
}

#[cfg(not(target_os = "android"))]
fn init_logging() {
    simple_logger::SimpleLogger::new().init().unwrap();
}

/// A simple egui + wgpu + winit based example.
#[mobile_entry_point]
fn main() {
    init_logging();
    let event_loop = winit::event_loop::EventLoop::with_user_event();
    let window = winit::window::WindowBuilder::new()
        .with_title("A fantastic window!")
        .with_inner_size(winit::dpi::LogicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT))
        .build(&event_loop)
        .unwrap();

    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

    let mut surface = if cfg!(target_os = "android") {
        None
    } else {
        Some(unsafe { instance.create_surface(&window) })
    };

    // WGPU 0.11+ support force fallback (if HW implementation not supported), set it to true or false (optional).
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: surface.as_ref(),
        force_fallback_adapter: false,
    }))
    .unwrap();

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            features: wgpu::Features::default(),
            limits: wgpu::Limits::default(),
            label: None,
        },
        None,
    ))
    .unwrap();

    let surface_format = if let Some(surface) = &surface {
        surface.get_preferred_format(&adapter).unwrap()
    } else {
        // if Surface is none, we're guaranteed to be on android
        wgpu::TextureFormat::Rgba8UnormSrgb
    };

    let mut surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: INITIAL_WIDTH,
        height: INITIAL_HEIGHT,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    if let Some(surface) = &mut surface {
        surface.configure(&device, &surface_config);
    }

    let repaint_signal = std::sync::Arc::new(ExampleRepaintSignal(std::sync::Mutex::new(
        event_loop.create_proxy(),
    )));


    // We use the egui_wgpu_backend crate as the render backend.
    let mut egui_rpass = RenderPass::new(&device, surface_format, 1);

    // Display the demo application that ships with egui.
    let mut demo_app = egui_demo_lib::WrapApp::default();

    let mut previous_frame_time = None;

    let mut state = egui_winit::State::new(&window);
    let mut ctx = egui::CtxRef::default();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        let mut redraw = || {
            if let Some(surface) = &surface {
                let output_frame = match surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(e) => {
                        eprintln!("Dropped frame with error: {}", e);
                        return;
                    }
                };
                let output_view = output_frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let egui_start = Instant::now();
                let raw_input: egui::RawInput = state.take_egui_input(&window);
                ctx.begin_frame(raw_input);
                let mut app_output = epi::backend::AppOutput::default();

                let mut frame = epi::backend::FrameBuilder {
                    info: epi::IntegrationInfo {
                        name: "egui_winit",
                        web_info: None,
                        cpu_usage: previous_frame_time,
                        native_pixels_per_point: Some(window.scale_factor() as _),
                        prefer_dark_mode: None,
                    },
                    tex_allocator: &mut egui_rpass,
                    output: &mut app_output,
                    repaint_signal: repaint_signal.clone(),
                }
                .build();

                // Draw the demo application.
                demo_app.update(&ctx, &mut frame);

                // End the UI frame. We could now handle the output and draw the UI with the backend.
                let (_output, paint_commands) = ctx.end_frame();
                let paint_jobs = ctx.tessellate(paint_commands);

                let frame_time = (Instant::now() - egui_start).as_secs_f64() as f32;
                previous_frame_time = Some(frame_time);

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("encoder"),
                });

                // Upload all resources for the GPU.
                let screen_descriptor = ScreenDescriptor {
                    physical_width: surface_config.width,
                    physical_height: surface_config.height,
                    scale_factor: window.scale_factor() as f32,
                };
                egui_rpass.update_texture(&device, &queue, &ctx.texture());
                egui_rpass.update_user_textures(&device, &queue);
                egui_rpass.update_buffers(&device, &queue, &paint_jobs, &screen_descriptor);

                // Record all render passes.
                egui_rpass
                    .execute(
                        &mut encoder,
                        &output_view,
                        &paint_jobs,
                        &screen_descriptor,
                        Some(wgpu::Color::BLACK),
                    )
                    .unwrap();
                // Submit the commands.
                queue.submit(iter::once(encoder.finish()));

                // Redraw egui
                output_frame.present();
            };
        };
        match event {
            RedrawRequested(..) | MainEventsCleared => {
                redraw();
            }
            Resumed => {
                let s = unsafe { instance.create_surface(&window) };
                surface_config.format = s.get_preferred_format(&adapter).unwrap();
                s.configure(&device, &surface_config);
                surface = Some(s);
            }
            Suspended => {
                surface = None;
            }
            WindowEvent { event, .. } => {
                match event {
                    winit::event::WindowEvent::Resized(size) => {
                        if size.width != 0 && size.height != 0 {
                            // Recreate the swap chain with the new size
                            surface_config.width = size.width;
                            surface_config.height = size.height;
                            if let Some(surface) = &surface {
                                surface.configure(&device, &surface_config);
                            }
                        }
                    },
                    winit::event::WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    },
                    _ => {
                        state.on_event(&ctx, &event);
                    }
                };
            },
            _ => (),
        }
    });
}
