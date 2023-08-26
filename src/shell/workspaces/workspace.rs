use smithay::{
    desktop::{Space, Window, WindowSurfaceType},
    output::Output,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point, Rectangle},
};

#[derive(Default)]
pub struct Workspace {
    pub space: Space<Window>,
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

    pub fn map_output<P: Into<Point<i32, Logical>>>(&mut self, output: &Output, location: P) {
        self.space.map_output(output, location);
    }

    pub fn unmap_output(&mut self, output: &Output) {
        self.space.unmap_output(output);
    }

    pub fn output_geometry(&self, output: &Output) -> Option<Rectangle<i32, Logical>> {
        self.space.output_geometry(output)
    }

    pub fn refresh(&mut self) {
        self.space.refresh();
    }
}
