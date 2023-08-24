use std::time::Duration;

use anyhow::Result;
use smithay::{
    backend::renderer::{
        damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
        gles::GlesRenderer,
    },
    desktop::{space::render_output, Space, Window, WindowSurfaceType},
    input::keyboard::KeyboardHandle,
    output::{Mode, Output, Scale},
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point, Rectangle, Transform},
    wayland::compositor::{get_parent, is_sync_subsurface},
};

use crate::state::State;

pub struct Workspaces {
    active: usize,
    output: Option<Output>,
    workspaces: Vec<Workspace>,
}

impl Workspaces {
    pub fn new(n: usize) -> Self {
        assert!(n > 0, "Workspaces count should be > 0");
        let output = None;
        let mut workspaces = Vec::new();
        workspaces.resize_with(n, Default::default);
        Self {
            active: 0,
            output,
            workspaces,
        }
    }

    pub fn focused_window(&self, keyboard: KeyboardHandle<State>) -> Option<&Window> {
        keyboard
            .current_focus()
            .as_ref()
            .and_then(|surface| {
                (!is_sync_subsurface(surface)).then(|| {
                    let mut root = surface.clone();
                    while let Some(parent) = get_parent(&root) {
                        root = parent;
                    }
                    self.active()
                        .windows()
                        .find(|w| w.toplevel().wl_surface() == &root)
                })
            })
            .flatten()
    }

    pub fn switch_to(&mut self, n: usize) {
        // TODO grab focus
        assert!(n > 0, "Workspace number should be > 0");
        if let Some(output) = self.output.as_ref() {
            let n = n - 1;
            let old = self.active;
            let old_loc = self.workspaces[old].output_geometry(output).unwrap().loc;
            let new = n;
            self.active = new;
            self.workspaces[old].unmap_output(output);
            self.workspaces[new].map_output(output, old_loc);
        }
    }

    pub fn move_to(&mut self, n: usize, keyboard: KeyboardHandle<State>) {
        // TODO grab focus
        assert!(n > 0, "Workspace number should be > 0");
        let n = n - 1;
        if let Some(window) = self.focused_window(keyboard).cloned() {
            self.active_mut().unmap_window(&window);
            self.workspaces[n].map_window(window, Point::default(), false);
        }
    }

    pub fn active(&self) -> &Workspace {
        &self.workspaces[self.active]
    }

    pub fn active_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active]
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
        self.active_mut().map_output(output, Point::default());
    }

    pub fn output_geometry(&self) -> Option<Rectangle<i32, Logical>> {
        self.output
            .as_ref()
            .and_then(|output| self.active().output_geometry(output))
    }

    pub fn refresh(&mut self) {
        self.active_mut().refresh();
    }

    pub fn render_output(
        &self,
        renderer: &mut GlesRenderer,
        damage_tracker: &mut OutputDamageTracker,
    ) -> Result<()> {
        if let Some(output) = self.output.as_ref() {
            render_output::<_, WaylandSurfaceRenderElement<GlesRenderer>, _, _>(
                output,
                renderer,
                1.0,
                0,
                self.workspaces.iter().map(|w| &w.space),
                &[],
                damage_tracker,
                [0.6, 0.6, 0.6, 1.0],
            )?;
        }

        Ok(())
    }

    pub fn send_frames(&self, time: Duration) {
        if let Some(output) = self.output.as_ref() {
            self.active().windows().for_each(|w| {
                w.send_frame(output, time, Some(Duration::ZERO), |_, _| None);
            })
        }
    }
}

#[derive(Default)]
pub struct Workspace {
    space: Space<Window>,
}

impl Workspace {
    pub fn map_window(
        &mut self,
        window: Window,
        location: impl Into<Point<i32, Logical>>,
        activate: bool,
    ) {
        self.space.map_element(window, location, activate);
    }

    pub fn unmap_window(&mut self, window: &Window) {
        self.space.unmap_elem(window);
    }

    pub fn windows(&self) -> impl DoubleEndedIterator<Item = &Window> {
        self.space.elements()
    }

    pub fn window_under(
        &self,
        point: impl Into<Point<f64, Logical>>,
    ) -> Option<(&Window, Point<i32, Logical>)> {
        self.space.element_under(point)
    }

    pub fn surface_under(
        &self,
        point: Point<f64, Logical>,
    ) -> Option<(WlSurface, Point<i32, Logical>)> {
        self.window_under(point).and_then(|(window, location)| {
            window
                .surface_under(point - location.to_f64(), WindowSurfaceType::ALL)
                .map(|(s, p)| (s, p + location))
        })
    }

    pub fn raise_window(&mut self, window: &Window, activate: bool) {
        self.space.raise_element(window, activate);
    }

    fn map_output<P: Into<Point<i32, Logical>>>(&mut self, output: &Output, location: P) {
        self.space.map_output(output, location);
    }

    fn unmap_output(&mut self, output: &Output) {
        self.space.unmap_output(output);
    }

    fn output_geometry(&self, output: &Output) -> Option<Rectangle<i32, Logical>> {
        self.space.output_geometry(output)
    }

    fn refresh(&mut self) {
        self.space.refresh();
    }
}
