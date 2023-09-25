use smithay::backend::renderer::gles::{GlesRenderer, GlesTexProgram, UniformName, UniformType};

static OUTLINE_SHADER: &str = include_str!("./shader.frag");

pub struct OutlineShader;

impl OutlineShader {
    pub fn compile(renderer: &mut GlesRenderer) {
        let src = OUTLINE_SHADER;
        let additional_uniforms = &[
            UniformName::new("color", UniformType::_3f),
            UniformName::new("thickness", UniformType::_1f),
            UniformName::new("radius", UniformType::_1f),
            UniformName::new("size", UniformType::_2f),
        ];
        let program = renderer.compile_custom_texture_shader(src, additional_uniforms).unwrap();
        renderer.egl_context().user_data().insert_if_missing(|| program);
    }

    pub fn program(renderer: &mut GlesRenderer) -> GlesTexProgram {
        renderer.egl_context().user_data().get().cloned().unwrap()
    }
}
