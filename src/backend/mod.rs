use std::env;

use smithay::reexports::calloop::LoopHandle;

pub use self::winit::Winit;
use crate::state::CalloopData;

mod winit;

pub enum Backend {
    Winit(Winit),
}

impl Backend {
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
        Backend: AsRef<B>,
    {
        AsRef::<B>::as_ref(self)
    }

    pub fn as_mut<B>(&mut self) -> &mut B
    where
        Backend: AsMut<B>,
    {
        AsMut::<B>::as_mut(self)
    }
}

impl AsRef<Winit> for Backend {
    fn as_ref(&self) -> &Winit {
        let Self::Winit(b) = self;
        b
    }
}

impl AsMut<Winit> for Backend {
    fn as_mut(&mut self) -> &mut Winit {
        let Self::Winit(b) = self;
        b
    }
}
