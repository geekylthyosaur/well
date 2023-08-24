use anyhow::Result;
use smithay::{
    backend::{
        renderer::{damage::OutputDamageTracker, gles::GlesRenderer},
        winit::{self, WinitError, WinitEvent, WinitEventLoop, WinitGraphicsBackend},
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    utils::{Rectangle, Transform},
};

use crate::state::{CalloopData, State};

pub struct WinitBackend {
    backend: WinitGraphicsBackend<GlesRenderer>,
    damage_tracker: OutputDamageTracker,
    winit: WinitEventLoop,
}

impl WinitBackend {
    pub fn new(data: &mut CalloopData) -> Self {
        let display = &data.display;
        let state = &mut data.state;

        let (backend, winit) = winit::init::<GlesRenderer>().expect("Failed to initialize backend");

        let mode = Mode {
            size: backend.window_size().physical_size,
            refresh: 60_000,
        };

        let output = Output::new(
            "winit".to_string(),
            PhysicalProperties {
                size: (0, 0).into(),
                subpixel: Subpixel::Unknown,
                make: "Smithay".into(),
                model: "Winit".into(),
            },
        );
        let _global = output.create_global::<State>(&display.handle());
        output.change_current_state(
            Some(mode),
            Some(Transform::Flipped180),
            None,
            Some((0, 0).into()),
        );
        output.set_preferred(mode);

        state.workspaces.map_output(&output);

        let damage_tracker = OutputDamageTracker::from_output(&output);

        Self {
            backend,
            damage_tracker,
            winit,
        }
    }

    pub fn dispatch(&mut self, data: &mut CalloopData) -> Result<()> {
        let state = &mut data.state;

        self.winit
            .dispatch_new_events(|event| match event {
                WinitEvent::Resized { .. } => {}
                WinitEvent::Input(event) => state.handle_input(event),
                _ => (),
            })
            .map_err(|err| {
                if matches!(err, WinitError::WindowClosed) {
                    state.is_running = false
                }
                err
            })?;

        let size = self.backend.window_size().physical_size;
        let damage = Rectangle::from_loc_and_size((0, 0), size);

        self.backend.bind()?;
        state
            .workspaces
            .render_output(self.backend.renderer(), &mut self.damage_tracker)?;
        self.backend.submit(Some(&[damage]))?;

        state.workspaces.send_frames(state.start_time.elapsed());

        state.workspaces.refresh();

        Ok(())
    }
}
