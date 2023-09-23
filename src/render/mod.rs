use anyhow::Result;
use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            damage::{OutputDamageTracker, RenderOutputResult},
            gles::{GlesRenderer, GlesTexture},
            Bind, Offscreen,
        },
        winit::WinitGraphicsBackend,
    },
    utils::{Buffer, Size},
};

use crate::state::State;

use self::element::OutputRenderElement;

pub mod element;
pub mod shader;

pub const CLEAR_COLOR: [f32; 4] = [0.6, 0.6, 0.6, 1.0];

impl State {
    pub fn render_output(
        &self,
        backend: &mut WinitGraphicsBackend<GlesRenderer>,
        age: usize,
        damage_tracker: &mut OutputDamageTracker,
    ) -> Result<RenderOutputResult> {
        let focus = self.get_focus();
        let elements = self.shell.workspaces.render_elements(
            backend.renderer(),
            damage_tracker,
            focus.as_ref(),
            &self.config,
        )?;

        backend.bind().unwrap();
        Ok(damage_tracker.render_output(backend.renderer(), age, &elements, CLEAR_COLOR)?)
    }
}

pub fn render_offscreen(
    renderer: &mut GlesRenderer,
    damage_tracker: &mut OutputDamageTracker,
    elements: &[OutputRenderElement],
    size: Size<i32, Buffer>,
) -> Result<Option<GlesTexture>> {
    if size.w == 0 || size.h == 0 {
        return Ok(None);
    }
    let texture = Offscreen::<GlesTexture>::create_buffer(renderer, Fourcc::Abgr8888, size)?;
    renderer.bind(texture.clone())?;
    damage_tracker.render_output(renderer, 0, elements, CLEAR_COLOR)?;
    Ok(Some(texture))
}