use std::{ffi::CStr, time::Duration};

use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker,
            element::{surface::WaylandSurfaceRenderElement, AsRenderElements},
            gles::{ffi, GlesRenderer, GlesTexture},
            Bind, Offscreen,
        },
        winit::WinitGraphicsBackend,
    },
    desktop::Window,
    output::{Mode, Output, Scale},
    utils::{Logical, Point, Rectangle, Transform},
};

use crate::{
    config::Config,
    render::{OutlineShader, OutputRenderElement, CLEAR_COLOR},
};

use self::workspace::Workspace;

use super::fullscreen::{GeometryBeforeFullscreen, IsFullscreen};

mod workspace;

pub struct Workspaces {
    current: usize,
    pub output: Option<Output>,
    workspaces: Vec<Workspace>,
}

impl Workspaces {
    pub fn new(n: usize) -> Self {
        let output = None;
        let mut workspaces = Vec::new();
        workspaces.resize_with(n, Default::default);
        Self {
            current: 0,
            output,
            workspaces,
        }
    }

    pub fn switch_to(&mut self, new: usize) {
        if let Some(output) = self.output.as_ref() {
            let old = self.current;
            let old_loc = self.workspaces[old].output_geometry(output).unwrap().loc;
            self.current = new;
            self.workspaces[old].unmap_output(output);
            self.workspaces[new].map_output(output, old_loc);
        }
    }

    pub fn move_to(&mut self, window: Window, new: usize) {
        self.current_mut().unmap_window(&window);
        self.workspaces[new].map_window(window, Point::default(), false);
    }

    pub fn fullscreen(&self, window: &Window) {
        let old_geometry = window.geometry();
        window.toplevel().with_pending_state(|state| {
            state.size = self.output_geometry().map(|g| g.size);
        });
        IsFullscreen::set(window, true);
        GeometryBeforeFullscreen::set(window, old_geometry);
        window.toplevel().send_pending_configure();
    }

    pub fn unfullscreen(&mut self, window: &Window) {
        let old_geometry = GeometryBeforeFullscreen::get(window);
        window.toplevel().with_pending_state(|state| {
            state.size = old_geometry.map(|g| g.size);
        });
        if let Some(old_geometry) = old_geometry {
            self.current_mut()
                .map_window(window.to_owned(), old_geometry.loc, false);
        }
        IsFullscreen::set(window, false);
        window.toplevel().send_pending_configure();
    }

    pub fn is_fullscreen(&self, window: &Window) -> bool {
        IsFullscreen::get(window)
    }

    pub fn current(&self) -> &Workspace {
        &self.workspaces[self.current]
    }

    pub fn current_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.current]
    }

    pub fn change_output_mode(&self, new_mode: Mode) {
        if let Some(output) = self.output.as_ref() {
            output.change_current_state(Some(new_mode), None, None, None);
        }
    }

    pub fn _change_output_transform(&self, new_transform: Transform) {
        if let Some(output) = self.output.as_ref() {
            output.change_current_state(None, Some(new_transform), None, None);
        }
    }

    pub fn _change_output_scale(&self, new_scale: Scale) {
        if let Some(output) = self.output.as_ref() {
            output.change_current_state(None, None, Some(new_scale), None);
        }
    }

    pub fn _change_output_location(&self, new_location: Point<i32, Logical>) {
        if let Some(output) = self.output.as_ref() {
            output.change_current_state(None, None, None, Some(new_location));
        }
    }

    pub fn map_output(&mut self, output: &Output) {
        self.output = Some(output.clone());
        self.current_mut().map_output(output, Point::default());
    }

    pub fn output_geometry(&self) -> Option<Rectangle<i32, Logical>> {
        self.output
            .as_ref()
            .and_then(|output| self.current().output_geometry(output))
    }

    pub fn refresh(&mut self) {
        self.current_mut().refresh();
    }

    pub fn render_elements(
        &self,
        backend: &mut WinitGraphicsBackend<GlesRenderer>,
        damage_tracker: &mut OutputDamageTracker,
        focus: Option<&Window>,
        config: &Config,
    ) -> Vec<OutputRenderElement> {
        let elements = vec![];
        if let Some(output) = self.output.as_ref() {
            let space = &self.current().space;
            let scale = output.current_scale().fractional_scale();
            let alpha = 1.0;

            for e in space.elements() {
                let location = space.element_location(e).unwrap_or_default();
                let thickness = config.outline.thickness;

                let location = (location - e.geometry().loc).to_physical_precise_round(scale);
                let window_elements = e
                    .render_elements::<WaylandSurfaceRenderElement<GlesRenderer>>(
                        backend.renderer(),
                        location,
                        scale.into(),
                        alpha,
                    )
                    .into_iter()
                    .map(OutputRenderElement::from)
                    .collect::<Vec<_>>();

                let size = space.element_geometry(e).unwrap();

                let size = size.size.to_buffer(scale as i32, Transform::Normal);
                if size.w != 0 || size.h != 0 {
                    let texture = Offscreen::<GlesTexture>::create_buffer(
                        backend.renderer(),
                        smithay::backend::allocator::Fourcc::Argb8888,
                        size,
                    )
                    .unwrap();
                    let tex_id = texture.tex_id();
                    // Clone is necessary to keep tex_id valid
                    backend.renderer().bind(texture.clone()).unwrap();

                    damage_tracker
                        .render_output(backend.renderer(), 0, &window_elements, CLEAR_COLOR)
                        .unwrap();

                    backend.bind().unwrap();

                    let renderer = backend.renderer();
                    let location = space.element_location(e).unwrap_or_default();
                    let color = focus
                        .and_then(|focus| focus.eq(e).then_some(config.outline.focus_color))
                        .unwrap_or(config.outline.color);
                    let geometry = Rectangle::from_loc_and_size(location, e.geometry().size);
                    let radius = 10;

                    let shader_element =
                        OutlineShader::element(renderer, color, geometry, radius, thickness);
                    let elements = vec![OutputRenderElement::Outline(shader_element)];

                    renderer
                        .with_context(|gl| unsafe {
                            let mut program = 0;
                            gl.GetIntegerv(ffi::CURRENT_PROGRAM, &mut program);
                            let program: u32 = std::mem::transmute(program);
                            gl.ActiveTexture(ffi::TEXTURE0);
                            gl.BindTexture(ffi::TEXTURE_2D, tex_id);
                            gl.TexParameteri(
                                ffi::TEXTURE_2D,
                                ffi::TEXTURE_MIN_FILTER,
                                ffi::NEAREST as i32,
                            );
                            gl.TexParameteri(
                                ffi::TEXTURE_2D,
                                ffi::TEXTURE_MAG_FILTER,
                                ffi::NEAREST as i32,
                            );
                            let location = gl.GetUniformLocation(
                                program,
                                CStr::from_bytes_with_nul(b"tex\0").unwrap().as_ptr()
                                    as *const ffi::types::GLchar,
                            );
                            gl.Uniform1i(location, 0);
                        })
                        .unwrap();

                    self._change_output_transform(Transform::Flipped180);

                    damage_tracker
                        .render_output(renderer, 0, &elements, CLEAR_COLOR)
                        .unwrap();

                    self._change_output_transform(Transform::Normal);
                }
            }
        }
        elements
    }

    pub fn send_frames(&self, time: Duration) {
        if let Some(output) = self.output.as_ref() {
            self.current().windows().for_each(|w| {
                w.send_frame(output, time, Some(Duration::ZERO), |_, _| None);
            })
        }
    }
}
