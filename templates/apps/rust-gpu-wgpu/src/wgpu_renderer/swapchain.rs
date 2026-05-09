use anyhow::Context;
use std::sync::Arc;
use wgpu::{Adapter, CurrentSurfaceTexture, Device, Instance, Surface, TextureFormat, TextureView};
use winit::dpi::PhysicalSize;
use winit::window::Window;

pub struct MySwapchainManager<'a> {
    instance: Instance,
    adapter: Adapter,
    device: Device,
    window: Arc<Window>,
    surface: Surface<'a>,
    format: TextureFormat,

    // state below
    active: Option<ActiveConfiguration>,
    should_recreate: bool,
}

pub struct ActiveConfiguration {
    size: PhysicalSize<u32>,
}

impl<'a> MySwapchainManager<'a> {
    pub fn new(
        instance: Instance,
        adapter: Adapter,
        device: Device,
        window: Arc<Window>,
        surface: Surface<'a>,
    ) -> Self {
        let caps = surface.get_capabilities(&adapter);
        Self {
            instance,
            adapter,
            device,
            window,
            surface,
            format: caps.formats[0],
            active: None,
            should_recreate: true,
        }
    }

    #[inline]
    pub fn should_recreate(&mut self) {
        self.should_recreate = true;
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }

    pub fn render(
        &mut self,
        f: impl FnOnce(TextureView) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let size = self.window.inner_size();
        if let Some(active) = &self.active {
            if active.size != size {
                self.should_recreate();
            }
        } else {
            self.should_recreate();
        }

        if self.should_recreate {
            self.should_recreate = false;
            self.configure_surface(size)?;
        }

        match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(surface_texture) => {
                let output_view =
                    surface_texture
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor {
                            format: Some(self.format),
                            ..wgpu::TextureViewDescriptor::default()
                        });
                f(output_view)?;
                surface_texture.present();
            }
            CurrentSurfaceTexture::Occluded | CurrentSurfaceTexture::Timeout => (),
            CurrentSurfaceTexture::Suboptimal(_) | CurrentSurfaceTexture::Outdated => {
                self.should_recreate();
            }
            CurrentSurfaceTexture::Validation => {
                anyhow::bail!("Validation error during surface texture acquisition")
            }
            CurrentSurfaceTexture::Lost => {
                self.surface = self.instance.create_surface(self.window.clone())?;
                self.should_recreate();
            }
        };
        Ok(())
    }

    fn configure_surface(&mut self, size: PhysicalSize<u32>) -> anyhow::Result<()> {
        let mut surface_config = self
            .surface
            .get_default_config(&self.adapter, size.width, size.height)
            .with_context(|| {
                format!(
                    "Incompatible adapter for surface, returned capabilities: {:?}",
                    self.surface.get_capabilities(&self.adapter)
                )
            })?;

        // force srgb surface format
        surface_config.view_formats.push(self.format);
        // limit framerate to vsync
        surface_config.present_mode = wgpu::PresentMode::AutoVsync;
        self.surface.configure(&self.device, &surface_config);

        self.active = Some(ActiveConfiguration { size });
        Ok(())
    }
}
