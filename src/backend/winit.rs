use std::time::Duration;

use anyhow::{anyhow, Result};
use smithay::{
    backend::{
        renderer::{
            damage::{OutputDamageTracker, RenderOutputResult},
            gles::GlesRenderer,
        },
        winit::{self, WinitError, WinitEvent, WinitEventLoop, WinitGraphicsBackend},
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::calloop::{
        timer::{TimeoutAction, Timer},
        LoopHandle,
    },
    utils::Transform,
};

use crate::{
    render::{shader::OutlineShader, CLEAR_COLOR},
    state::{CalloopData, State},
};

pub struct Winit {
    backend: WinitGraphicsBackend<GlesRenderer>,
    damage_tracker: OutputDamageTracker,
    output: Output,
}

impl Winit {
    pub fn init(data: &mut CalloopData) {
        let output = &data.backend.winit().output;
        let _global = output.create_global::<State>(&data.state.display_handle);
        data.state.shell.workspaces.map_output(output);
    }

    pub fn new(event_loop: LoopHandle<'static, CalloopData>) -> Self {
        let (mut backend, mut winit) =
            winit::init::<GlesRenderer>().expect("Failed to initialize backend");

        let mode = Mode { size: backend.window_size().physical_size, refresh: 60_000 };

        let output = Output::new(
            "winit".to_string(),
            PhysicalProperties {
                size: (0, 0).into(),
                subpixel: Subpixel::Unknown,
                make: "Smithay".into(),
                model: "Winit".into(),
            },
        );
        output.change_current_state(
            Some(mode),
            Some(Transform::Flipped180),
            None,
            Some((0, 0).into()),
        );
        output.set_preferred(mode);

        let damage_tracker = OutputDamageTracker::from_output(&output);

        OutlineShader::compile(backend.renderer());

        let timer = Timer::immediate();
        event_loop
            .insert_source(timer, move |_, _, data| {
                data.backend.winit().dispatch(&mut data.state, &mut winit);
                TimeoutAction::ToDuration(Duration::from_secs_f32(1. / 60.))
            })
            .map_err(|_| anyhow!("Failed to initialize backend source"))
            .unwrap();

        Self { backend, damage_tracker, output }
    }

    pub fn dispatch(&mut self, state: &mut State, winit: &mut WinitEventLoop) {
        if let Err(WinitError::WindowClosed) = winit.dispatch_new_events(|event| match event {
            WinitEvent::Resized { size, .. } => {
                state.shell.workspaces.change_output_mode(Mode { size, refresh: 60_000 })
            }
            WinitEvent::Input(event) => state.handle_input(event),
            WinitEvent::Refresh => self.render(state).unwrap(),
            _ => (),
        }) {
            state.is_running = false;
        }
    }

    pub fn render(&mut self, state: &mut State) -> Result<()> {
        let backend = &mut self.backend;
        let elements = state.shell.workspaces.render_elements(
            backend.renderer(),
            &mut self.damage_tracker,
            state.get_focus().as_ref(),
            &state.config,
        )?;
        backend.bind()?;
        let age = backend.buffer_age().unwrap_or_default();
        let res =
            self.damage_tracker.render_output(backend.renderer(), age, &elements, CLEAR_COLOR);
        if let Ok(RenderOutputResult { damage, .. }) = res {
            self.backend.submit(damage.as_deref())?;
        }

        self.backend.window().request_redraw();

        state.shell.workspaces.send_frames(state.start_time.elapsed());
        state.shell.workspaces.refresh();

        Ok(())
    }
}
