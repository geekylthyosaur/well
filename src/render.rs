use smithay::{
    backend::{
        renderer::{
            damage::{Error as RenderOutputError, OutputDamageTracker, RenderOutputResult},
            element::{surface::WaylandSurfaceRenderElement, Element, Id, RenderElement},
            gles::{
                GlesError, GlesFrame, GlesRenderer, GlesTexProgram, GlesTexture, Uniform,
                UniformName, UniformType,
            },
            utils::CommitCounter,
        },
        winit::WinitGraphicsBackend,
    },
    render_elements,
    utils::{Logical, Rectangle, Transform},
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
    ) -> Result<RenderOutputResult, RenderOutputError<GlesRenderer>> {
        let focus = self.get_focus();
        let elements = self.shell.workspaces.render_elements(
            backend,
            damage_tracker,
            focus.as_ref(),
            &self.config,
        );

        backend.bind().unwrap();
        self.shell
            .workspaces
            .change_output_transform(smithay::utils::Transform::Flipped180);
        let res = damage_tracker.render_output(backend.renderer(), age, &elements, CLEAR_COLOR);
        self.shell
            .workspaces
            .change_output_transform(smithay::utils::Transform::Normal);
        res
    }
}

pub struct OutlineShader;

impl OutlineShader {
    pub fn program(renderer: &mut GlesRenderer) -> GlesTexProgram {
        let src = OUTLINE_SHADER;
        let additional_uniforms = &[
            UniformName::new("color", UniformType::_3f),
            UniformName::new("thickness", UniformType::_1f),
            UniformName::new("radius", UniformType::_1f),
            UniformName::new("size", UniformType::_2f),
        ];
        renderer
            .compile_custom_texture_shader(src, additional_uniforms)
            .unwrap()
    }
}

render_elements! {
    pub OutputRenderElement<=GlesRenderer>;
    Window = WaylandSurfaceRenderElement<GlesRenderer>,
    RoundedWindow = RoundedElement,
}

pub struct RoundedElement {
    color: [f32; 3],
    commit_counter: CommitCounter,
    geometry: Rectangle<i32, Logical>,
    id: Id,
    program: GlesTexProgram,
    radius: f32,
    texture: GlesTexture,
    thickness: f32,
}

impl RoundedElement {
    pub fn new(
        color: [f32; 3],
        geometry: Rectangle<i32, Logical>,
        program: GlesTexProgram,
        radius: f32,
        texture: GlesTexture,
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
            thickness,
        }
    }
}

impl Element for RoundedElement {
    fn id(&self) -> &Id {
        &self.id
    }

    fn current_commit(&self) -> smithay::backend::renderer::utils::CommitCounter {
        self.commit_counter
    }

    fn src(&self) -> Rectangle<f64, smithay::utils::Buffer> {
        let scale = 1.0;
        self.geometry
            .to_f64()
            .to_buffer(scale, Transform::Normal, &self.geometry.size.to_f64())
    }

    fn geometry(
        &self,
        scale: smithay::utils::Scale<f64>,
    ) -> Rectangle<i32, smithay::utils::Physical> {
        self.geometry.to_f64().to_physical_precise_round(scale)
    }
}

impl RenderElement<GlesRenderer> for RoundedElement {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_>,
        src: Rectangle<f64, smithay::utils::Buffer>,
        dst: Rectangle<i32, smithay::utils::Physical>,
        damage: &[Rectangle<i32, smithay::utils::Physical>],
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
            smithay::utils::Transform::Normal,
            1.0,
            program,
            &additional_uniforms,
        )
    }
}
