use smithay::{
    backend::renderer::{
        damage::{Error as OutputDamageTrackerError, OutputDamageTracker, RenderOutputResult},
        element::surface::WaylandSurfaceRenderElement,
        gles::{element::PixelShaderElement, GlesRenderer, Uniform, UniformName, UniformType},
    },
    render_elements,
    utils::{Logical, Rectangle},
};

use crate::state::State;

const CLEAR_COLOR: [f32; 4] = [0.6, 0.6, 0.6, 1.0];

static OUTLINE_SHADER: &str = include_str!("./shader.frag");

impl State {
    pub fn render_output(
        &self,
        age: usize,
        damage_tracker: &mut OutputDamageTracker,
        renderer: &mut GlesRenderer,
    ) -> Result<RenderOutputResult, OutputDamageTrackerError<GlesRenderer>> {
        let elements = self.shell.workspaces.render_elements(renderer);

        damage_tracker.render_output(renderer, age, &elements, CLEAR_COLOR)
    }
}

pub struct OutlineShader;

impl OutlineShader {
    pub fn element(
        renderer: &mut GlesRenderer,
        color: [f32; 4],
        mut geometry: Rectangle<i32, Logical>,
        radius: u8,
        thickness: u8,
    ) -> PixelShaderElement {
        let shader = {
            let src = OUTLINE_SHADER;
            let additional_uniforms = &[
                UniformName::new("color", UniformType::_3f),
                UniformName::new("thickness", UniformType::_1f),
                UniformName::new("radius", UniformType::_1f),
            ];
            renderer
                .compile_custom_pixel_shader(src, additional_uniforms)
                .unwrap()
        };
        let area = {
            let t = thickness as i32;
            geometry.loc -= (t, t).into();
            geometry.size += (t * 2, t * 2).into();
            geometry
        };
        let opaque_regions = None;
        let alpha = color[3];
        let additional_uniforms = vec![
            Uniform::new(
                "color",
                [color[0] * alpha, color[1] * alpha, color[2] * alpha],
            ),
            Uniform::new("thickness", thickness as f32),
            Uniform::new("radius", radius as f32),
        ];
        let mut element =
            PixelShaderElement::new(shader, area, opaque_regions, alpha, additional_uniforms);
        element.resize(geometry, None);
        element
    }
}

render_elements! {
    pub OutputRenderElement<=GlesRenderer>;
    Window = WaylandSurfaceRenderElement<GlesRenderer>,
    Outline = PixelShaderElement,
}
