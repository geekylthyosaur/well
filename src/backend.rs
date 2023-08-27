use anyhow::Result;
use smithay::{
    backend::{
        renderer::{
            damage::{OutputDamageTracker, RenderOutputResult},
            gles::GlesRenderer,
        },
        winit::{self, WinitError, WinitEvent, WinitEventLoop, WinitGraphicsBackend},
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    utils::Transform,
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

        state.shell.workspaces.map_output(&output);

        let damage_tracker = OutputDamageTracker::from_output(&output);

        Self {
            backend,
            damage_tracker,
            winit,
        }
    }

    pub fn dispatch(&mut self, data: &mut CalloopData) -> Result<()> {
        let state = &mut data.state;

        if let Err(WinitError::WindowClosed) = self.winit.dispatch_new_events(|event| match event {
            WinitEvent::Resized { size, .. } => state.shell.workspaces.change_output_mode(Mode {
                size,
                refresh: 60_000,
            }),
            WinitEvent::Input(event) => state.handle_input(event),
            _ => (),
        }) {
            state.is_running = false;
            return Ok(());
        }

        self.backend.bind()?;
        let age = self.backend.buffer_age().unwrap_or_default();
        if let Ok(RenderOutputResult { damage, .. }) =
            state.render_output(age, &mut self.damage_tracker, self.backend.renderer())
        {
            self.backend.submit(damage.as_deref())?;
        }

        state
            .shell
            .workspaces
            .send_frames(state.start_time.elapsed());

        state.shell.workspaces.refresh();

        Ok(())
    }
}
