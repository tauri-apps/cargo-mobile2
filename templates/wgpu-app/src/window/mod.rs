// winit EL2 (0.20 onward) doesn't currently support Android, so we have to use
// the last EL1 release (0.19) for now, and abstract over both versions... in
// the future, you'll be able to just use winit like normal, which'll be dope.
// (note that this is a very lazy implementation)

#[cfg(target_os = "android")]
mod el1;
#[cfg(not(target_os = "android"))]
mod el2;

#[cfg(target_os = "android")]
pub use self::el1::*;
#[cfg(not(target_os = "android"))]
pub use self::el2::*;

#[derive(Debug)]
pub enum Event {
    #[cfg(not(target_os = "android"))]
    MainEventsCleared,
    WindowEvent(WindowEvent),
    RedrawRequested,
}

#[derive(Debug)]
pub enum WindowEvent {
    Resized { width: u32, height: u32 },
    CloseRequested,
}
