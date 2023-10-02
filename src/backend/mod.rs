use std::env;

use anyhow::Result;
use smithay::backend::renderer::gles::{GlesRenderer, GlesTexture};
use smithay::reexports::calloop::LoopHandle;
use smithay::utils::{Buffer, Size};

use self::winit::Winit;
use crate::render::element::OutputRenderElement;
use crate::state::{CalloopData, State};

mod winit;

pub trait Backend {
    fn render(&mut self, state: &mut State) -> Result<()>;
    fn renderer(&mut self) -> &mut GlesRenderer;
    fn render_offscreen(
        &mut self,
        elements: &[OutputRenderElement],
        size: Size<i32, Buffer>,
    ) -> Result<Option<GlesTexture>>;
}

pub enum BackendState {
    Winit(Winit),
}

impl BackendState {
    pub fn new(event_loop: LoopHandle<'static, CalloopData>) -> Self {
        if env::var_os("WAYLAND_DISPLAY").is_some() || env::var_os("DISPLAY").is_some() {
            Self::Winit(Winit::new(event_loop))
        } else {
            panic!("Standalone mode is not supported");
        }
    }

    pub fn init(data: &mut CalloopData) {
        if env::var_os("WAYLAND_DISPLAY").is_some() || env::var_os("DISPLAY").is_some() {
            Winit::init(data);
        } else {
            panic!("Standalone mode is not supported");
        }
    }

    pub fn as_ref<B>(&self) -> &B
    where
        BackendState: AsRef<B>,
    {
        AsRef::<B>::as_ref(self)
    }

    pub fn as_mut<B>(&mut self) -> &mut B
    where
        BackendState: AsMut<B>,
    {
        AsMut::<B>::as_mut(self)
    }
}

impl AsRef<Winit> for BackendState {
    fn as_ref(&self) -> &Winit {
        let Self::Winit(b) = self;
        b
    }
}

impl AsMut<Winit> for BackendState {
    fn as_mut(&mut self) -> &mut Winit {
        let Self::Winit(b) = self;
        b
    }
}
