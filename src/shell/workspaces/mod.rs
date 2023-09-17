use std::time::Duration;

use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker,
            element::{surface::WaylandSurfaceRenderElement, AsRenderElements},
            gles::{GlesRenderer, GlesTexture},
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
    render::{OutlineShader, OutputRenderElement, RoundedElement, CLEAR_COLOR},
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
        let mut elements = vec![];
        if let Some(output) = self.output.as_ref() {
            let space = &self.current().space;
            let output_scale = output.current_scale().fractional_scale();
            let output_geometry = self.output_geometry().unwrap();
            let alpha = 1.0;

            for e in space.elements().rev() {
                let mut geometry = space.element_geometry(e).unwrap_or_default();

                let size = geometry
                    .size
                    .to_buffer(output_scale as i32, Transform::Normal);
                if size.w == 0 || size.h == 0 {
                    continue;
                }

                // TODO: wtf
                let location = (Point::from((0, output_geometry.size.h - e.geometry().size.h))
                    - e.geometry().loc)
                    .to_physical_precise_round(output_scale);
                let window_elements = e
                    .render_elements::<WaylandSurfaceRenderElement<GlesRenderer>>(
                        backend.renderer(),
                        location,
                        output_scale.into(),
                        alpha,
                    )
                    .into_iter()
                    .collect::<Vec<_>>();

                let texture = Offscreen::<GlesTexture>::create_buffer(
                    backend.renderer(),
                    smithay::backend::allocator::Fourcc::Argb8888,
                    size,
                )
                .unwrap();
                backend.renderer().bind(texture.clone()).unwrap();

                damage_tracker
                    .render_output(backend.renderer(), 0, &window_elements, CLEAR_COLOR)
                    .unwrap();

                let renderer = backend.renderer();
                let color = focus
                    .and_then(|focus| focus.eq(e).then_some(config.outline.focused_color))
                    .unwrap_or(config.outline.color);
                let radius = config.outline.radius as f32;
                let thickness = config.outline.thickness as f32;

                let program = OutlineShader::program(renderer);
                let t = thickness as i32;
                geometry.size += (t * 2, t * 2).into();
                geometry.loc -= (t, t).into();
                let element =
                    RoundedElement::new(color, geometry, program, radius, texture, thickness);
                elements.push(OutputRenderElement::RoundedWindow(element));
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
