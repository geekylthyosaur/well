use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker,
            element::surface::WaylandSurfaceRenderElement,
            gles::{element::PixelShaderElement, GlesRenderer, Uniform, UniformName, UniformType},
        },
        winit::WinitGraphicsBackend,
    },
    render_elements,
    utils::{Logical, Rectangle},
};

use crate::state::State;

pub const CLEAR_COLOR: [f32; 4] = [0.6, 0.6, 0.6, 1.0];

pub static OUTLINE_SHADER: &str = include_str!("./shader.frag");

impl State {
    pub fn render_output(
        &self,
        backend: &mut WinitGraphicsBackend<GlesRenderer>,
        age: usize,
        damage_tracker: &mut OutputDamageTracker,
    ) -> () {
        let focus = self.get_focus();
        self.shell.workspaces.render_elements(
            backend,
            damage_tracker,
            focus.as_ref(),
            &self.config,
        );

        // let res = damage_tracker.render_output(backend.renderer(), age, &elements, CLEAR_COLOR);

        // res
    }
}

pub struct OutlineShader;

impl OutlineShader {
    pub fn element(
        renderer: &mut GlesRenderer,
        color: [f32; 3],
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
        let alpha = 1.0;
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
