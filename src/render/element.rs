use anyhow::Result;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::element::{Element, Id, RenderElement};
use smithay::backend::renderer::gles::{
    GlesError, GlesFrame, GlesRenderer, GlesTexProgram, GlesTexture, Uniform,
};
use smithay::backend::renderer::utils::CommitCounter;
use smithay::render_elements;
use smithay::utils::{Buffer, Logical, Physical, Rectangle, Scale, Transform};

use crate::config::Color;

render_elements! {
    pub OutputRenderElement<=GlesRenderer>;
    Window = WaylandSurfaceRenderElement<GlesRenderer>,
    RoundedWindow = RoundedElement,
}

pub struct RoundedElement {
    color: Color,
    commit_counter: CommitCounter,
    geometry: Rectangle<i32, Logical>,
    id: Id,
    program: GlesTexProgram,
    radius: f32,
    texture: GlesTexture,
    transform: Transform,
    thickness: f32,
}

impl RoundedElement {
    pub fn new(
        color: Color,
        geometry: Rectangle<i32, Logical>,
        program: GlesTexProgram,
        radius: f32,
        texture: GlesTexture,
        transform: Transform,
        thickness: f32,
    ) -> Self {
        Self {
            color,
            commit_counter: CommitCounter::default(),
            geometry,
            id: Id::new(),
            program,
            radius,
            texture,
            transform,
            thickness,
        }
    }
}

impl Element for RoundedElement {
    fn id(&self) -> &Id {
        &self.id
    }

    fn current_commit(&self) -> CommitCounter {
        self.commit_counter
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        let scale = 1.0;
        let mut src =
            self.geometry.to_f64().to_buffer(scale, self.transform(), &self.geometry.size.to_f64());
        src.loc.x = 0.0;
        src.loc.y = 0.0;

        src
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.geometry.to_f64().to_physical_precise_round(scale)
    }

    fn transform(&self) -> Transform {
        self.transform
    }
}

impl RenderElement<GlesRenderer> for RoundedElement {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
    ) -> Result<(), GlesError> {
        let program = Some(&self.program);

        let additional_uniforms = vec![
            Uniform::new("color", self.color),
            Uniform::new("thickness", self.thickness),
            Uniform::new("radius", self.radius),
            Uniform::new("size", (dst.size.w as f32, dst.size.h as f32)),
        ];

        frame.render_texture_from_to(
            &self.texture,
            src,
            dst,
            damage,
            self.transform(),
            self.alpha(),
            program,
            &additional_uniforms,
        )
    }
}
