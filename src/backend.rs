use std::time::Duration;

use anyhow::Result;
use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer,
        },
        winit::{self, WinitError, WinitEvent, WinitEventLoop, WinitGraphicsBackend},
    },
    desktop::space::render_output,
    output::{Mode, Output, PhysicalProperties, Subpixel},
    utils::{Rectangle, Transform},
};

use crate::state::{CalloopData, State};

pub struct WinitBackend {
    backend: WinitGraphicsBackend<GlesRenderer>,
    damage_tracker: OutputDamageTracker,
    output: Output,
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

        state.space.map_output(&output, (0, 0));

        let damage_tracker = OutputDamageTracker::from_output(&output);

        Self {
            backend,
            damage_tracker,
            output,
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
        render_output::<_, WaylandSurfaceRenderElement<GlesRenderer>, _, _>(
            &self.output,
            self.backend.renderer(),
            1.0,
            0,
            [&state.space],
            &[],
            &mut self.damage_tracker,
            [0.6, 0.6, 0.6, 1.0],
        )?;
        self.backend.submit(Some(&[damage]))?;

        state.space.elements().for_each(|window| {
            window.send_frame(
                &self.output,
                state.start_time.elapsed(),
                Some(Duration::ZERO),
                |_, _| Some(self.output.clone()),
            )
        });

        state.space.refresh();

        Ok(())
    }
}
