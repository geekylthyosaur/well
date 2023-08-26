use std::time::Duration;

use anyhow::Result;
use smithay::{
    backend::renderer::{
        damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
        gles::GlesRenderer,
    },
    desktop::{space::render_output, Window},
    output::{Mode, Output, Scale},
    utils::{Logical, Point, Rectangle, Transform},
};

use self::workspace::Workspace;

use super::fullscreen::{GeometryBeforeFullscreen, IsFullscreen};

mod workspace;

pub struct Workspaces {
    current: usize,
    output: Option<Output>,
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

    pub fn render_output(
        &self,
        renderer: &mut GlesRenderer,
        damage_tracker: &mut OutputDamageTracker,
    ) -> Result<()> {
        if let Some(output) = self.output.as_ref() {
            let clear_color = [0.6, 0.6, 0.6, 1.0];
            render_output::<_, WaylandSurfaceRenderElement<GlesRenderer>, _, _>(
                output,
                renderer,
                1.0,
                0,
                self.workspaces.iter().map(|w| &w.space),
                &[],
                damage_tracker,
                clear_color,
            )?;
        }

        Ok(())
    }

    pub fn send_frames(&self, time: Duration) {
        if let Some(output) = self.output.as_ref() {
            self.current().windows().for_each(|w| {
                w.send_frame(output, time, Some(Duration::ZERO), |_, _| None);
            })
        }
    }
}
