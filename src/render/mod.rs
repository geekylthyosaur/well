use anyhow::Result;
use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            damage::OutputDamageTracker,
            gles::{GlesRenderer, GlesTexture},
            Bind, Offscreen,
        },
    },
    utils::{Buffer, Size},
};

use self::element::OutputRenderElement;

pub mod element;
pub mod shader;

pub const CLEAR_COLOR: [f32; 4] = [0.6, 0.6, 0.6, 1.0];

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
