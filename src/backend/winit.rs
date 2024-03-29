use std::time::Duration;

use anyhow::{anyhow, Result};
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::damage::{OutputDamageTracker, RenderOutputResult};
use smithay::backend::renderer::gles::{GlesRenderer, GlesTexture};
use smithay::backend::renderer::Offscreen;
use smithay::backend::winit::{self, WinitEvent, WinitEventLoop, WinitGraphicsBackend};
use smithay::output::{Mode, Output, PhysicalProperties, Subpixel};
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::winit::platform::pump_events::PumpStatus;
use smithay::utils::{Buffer, Size, Transform};

use super::Backend;
use crate::render::element::OutputRenderElement;
use crate::render::shader::OutlineShader;
use crate::render::CLEAR_COLOR;
use crate::state::{CalloopData, State};

pub struct Winit {
    backend: WinitGraphicsBackend<GlesRenderer>,
    damage_tracker: OutputDamageTracker,
    output: Output,
}

impl Winit {
    pub fn init(data: &mut CalloopData) {
        let output = &data.backend.as_ref::<Self>().output;
        let _global = output.create_global::<State>(&data.state.display_handle);
        data.state.shell.workspaces.map_output(output);
    }

    pub fn new(event_loop: LoopHandle<'static, CalloopData>) -> Self {
        let (mut backend, mut winit) =
            winit::init::<GlesRenderer>().expect("Failed to initialize backend");

        let mode = Mode { size: backend.window_size(), refresh: 60_000 };

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
                data.backend.as_mut::<Self>().dispatch(&mut data.state, &mut winit);
                TimeoutAction::ToDuration(Duration::from_secs_f32(1. / 60.))
            })
            .map_err(|_| anyhow!("Failed to initialize backend source"))
            .unwrap();

        Self { backend, damage_tracker, output }
    }

    pub fn dispatch(&mut self, state: &mut State, winit: &mut WinitEventLoop) {
        let dispatcher = winit.dispatch_new_events(|event| match event {
            WinitEvent::Resized { size, .. } => {
                state.shell.workspaces.change_output_mode(Mode { size, refresh: 60_000 })
            }
            WinitEvent::Input(event) => state.handle_input(event),
            WinitEvent::Redraw => self.render(state).unwrap(),
            _ => (),
        });
        if let PumpStatus::Exit(_) = dispatcher {
            state.is_running = false;
        }
    }
}

impl Backend for Winit {
    fn render(&mut self, state: &mut State) -> Result<()> {
        let focus = state.get_focus();
        let elements =
            state.shell.workspaces.render_elements(self, focus.as_ref(), &state.config)?;
        let backend = &mut self.backend;
        backend.bind()?;
        let age = backend.buffer_age().unwrap_or_default();
        let renderer = backend.renderer();
        let res = self.damage_tracker.render_output(renderer, age, &elements, CLEAR_COLOR);
        if let Ok(RenderOutputResult { damage, .. }) = res {
            self.backend.submit(damage.as_deref())?;
        }

        self.backend.window().request_redraw();

        state.shell.workspaces.send_frames(state.start_time.elapsed());
        state.shell.workspaces.refresh();

        Ok(())
    }

    fn renderer(&mut self) -> &mut GlesRenderer {
        self.backend.renderer()
    }

    fn render_offscreen(
        &mut self,
        elements: &[OutputRenderElement],
        size: Size<i32, Buffer>,
    ) -> Result<Option<GlesTexture>> {
        if size.w == 0 || size.h == 0 {
            return Ok(None);
        }
        let renderer = self.backend.renderer();
        let texture = Offscreen::<GlesTexture>::create_buffer(renderer, Fourcc::Abgr8888, size)?;
        self.damage_tracker.render_output_with(
            renderer,
            texture.clone(),
            0,
            elements,
            CLEAR_COLOR,
        )?;
        Ok(Some(texture))
    }
}
