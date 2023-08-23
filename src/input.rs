use std::process::Command;

use anyhow::Result;
use smithay::{
    backend::input::{Event, InputBackend, InputEvent, KeyState, KeyboardKeyEvent},
    input::keyboard::FilterResult,
    utils::SERIAL_COUNTER,
};
use tracing::{debug, error};

use crate::{
    config::{Action, Pattern},
    state::State,
};

impl State {
    pub fn handle_input<I: InputBackend>(&mut self, event: InputEvent<I>) {
        let keyboard = self.seat.get_keyboard().unwrap();
        match event {
            InputEvent::Keyboard { event } => {
                let action = self.action_from_event::<I>(event);
                if let Err(err) = self.process_action(action) {
                    error!(?err);
                }
            }
            InputEvent::PointerMotionAbsolute { .. } => {
                if let Some(surface) = self
                    .xdg_shell_state
                    .toplevel_surfaces()
                    .iter()
                    .next()
                    .cloned()
                {
                    let surface = surface.wl_surface().clone();
                    keyboard.set_focus(self, Some(surface), 0.into());
                };
            }
            _ => {}
        }
    }

    fn action_from_event<I: InputBackend>(&mut self, event: I::KeyboardKeyEvent) -> Option<Action> {
        let code = event.key_code();
        let state = event.state();
        let serial = SERIAL_COUNTER.next_serial();
        let time = Event::time_msec(&event);
        let keyboard = self.seat.get_keyboard().unwrap();

        keyboard.input(
            self,
            code,
            state,
            serial,
            time,
            |data, modifiers, handle| {
                let key = handle.modified_sym();

                if state == KeyState::Pressed {
                    let pattern = Pattern {
                        modifiers: (*modifiers).into(),
                        key,
                    };
                    debug!(?pattern);
                    let action = data.config.bindings.0.get(&pattern).cloned();
                    debug!(?action);
                    action
                        .map(FilterResult::Intercept)
                        .unwrap_or(FilterResult::Forward)
                } else {
                    FilterResult::Forward
                }
            },
        )
    }

    fn process_action(&mut self, action: Option<Action>) -> Result<()> {
        match action {
            Some(Action::Exit) => self.is_running = false,
            Some(Action::Spawn(cmd)) => {
                Command::new(cmd).spawn()?;
            }
            _ => (),
        }
        Ok(())
    }
}
