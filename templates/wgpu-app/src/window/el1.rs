pub use winit::{EventsLoop as EventLoop, Window, WindowBuilder};

use super::{Event, WindowEvent};

pub fn window_size(window: &Window) -> (u32, u32) {
    // so, DPI detection on Android is currently unsupported, and
    // even when that changes, it's not like it'd get backported to
    // EL1... so, let's take it easy and use the logical size as-is!
    window.get_inner_size().unwrap().into()
}

#[derive(Debug)]
pub enum ControlFlow {
    Poll,
    Exit,
}

fn conv_event(event: winit::Event) -> Option<Event> {
    fn window_event(event: WindowEvent) -> Option<Event> {
        Some(Event::WindowEvent(event))
    }

    match event {
        winit::Event::WindowEvent { event, .. } => match event {
            winit::WindowEvent::CloseRequested => window_event(WindowEvent::CloseRequested),
            winit::WindowEvent::Resized(logical_size) => {
                // see comment in `window_size`
                let (width, height) = logical_size.into();
                window_event(WindowEvent::Resized { width, height })
            }
            _ => None,
        },
        _ => None,
    }
}

pub fn run_event_loop(
    mut event_loop: EventLoop,
    mut f: impl FnMut(Event, &mut ControlFlow) + 'static,
) {
    let mut control_flow = ControlFlow::Poll;
    while !matches!(control_flow, ControlFlow::Exit) {
        event_loop.poll_events(|event| {
            if let Some(event) = conv_event(event) {
                f(event, &mut control_flow);
            }
        });
        // just unconditionally redraw
        f(Event::RedrawRequested, &mut control_flow);
    }
}
