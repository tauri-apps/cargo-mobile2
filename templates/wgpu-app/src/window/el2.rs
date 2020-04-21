pub use winit::{
    event_loop::ControlFlow,
    window::{Window, WindowBuilder},
};

use super::{Event, WindowEvent};

pub type EventLoop = winit::event_loop::EventLoop<()>;

pub fn window_size(window: &Window) -> (u32, u32) {
    window.inner_size().into()
}

fn conv_event(event: winit::event::Event<()>) -> Option<Event> {
    fn window_event(event: WindowEvent) -> Option<Event> {
        Some(Event::WindowEvent(event))
    }

    use winit::event::Event as WinitEvent;
    use winit::event::WindowEvent as WinitWindowEvent;
    match event {
        WinitEvent::MainEventsCleared => Some(Event::MainEventsCleared),
        WinitEvent::RedrawRequested(_) => Some(Event::RedrawRequested),
        WinitEvent::WindowEvent { event, .. } => match event {
            WinitWindowEvent::CloseRequested => window_event(WindowEvent::CloseRequested),
            WinitWindowEvent::Resized(winit::dpi::PhysicalSize { width, height }) => {
                window_event(WindowEvent::Resized { width, height })
            }
            _ => None,
        },
        _ => None,
    }
}

pub fn run_event_loop(event_loop: EventLoop, mut f: impl FnMut(Event, &mut ControlFlow) + 'static) {
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        if let Some(event) = conv_event(event) {
            f(event, control_flow);
        }
    })
}
