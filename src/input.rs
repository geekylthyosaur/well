use std::process::Command;

use anyhow::Result;
use smithay::{
    backend::input::{
        AbsolutePositionEvent, ButtonState, Event, InputBackend, InputEvent, KeyState,
        KeyboardKeyEvent, PointerButtonEvent,
    },
    input::{
        keyboard::FilterResult,
        pointer::{ButtonEvent, MotionEvent},
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::SERIAL_COUNTER,
};
use tracing::{debug, error};

use crate::{
    config::{Action, Pattern},
    state::State,
};

impl State {
    pub fn handle_input<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            InputEvent::Keyboard { event } => {
                let action = self.action_from_event::<I>(event);
                if let Err(err) = self.process_action(action) {
                    error!(?err);
                }
            }
            InputEvent::PointerMotionAbsolute { event, .. } => {
                let output_geo = self.workspaces.output_geometry().unwrap();

                let pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();

                let serial = SERIAL_COUNTER.next_serial();

                let pointer = self.seat.get_pointer().unwrap();

                let under = self.workspaces.surface_under(pos);

                if let Some((window, _loc)) = self
                    .workspaces
                    .active()
                    .window_under(pointer.current_location())
                    .map(|(w, l)| (w.clone(), l))
                {
                    let serial = SERIAL_COUNTER.next_serial();
                    let keyboard = self.seat.get_keyboard().unwrap();
                    keyboard.set_focus(self, Some(window.toplevel().wl_surface().clone()), serial);
                }

                pointer.motion(
                    self,
                    under,
                    &MotionEvent {
                        location: pos,
                        serial,
                        time: event.time_msec(),
                    },
                );
            }
            InputEvent::PointerButton { event, .. } => {
                let pointer = self.seat.get_pointer().unwrap();
                let keyboard = self.seat.get_keyboard().unwrap();

                let serial = SERIAL_COUNTER.next_serial();

                let button = event.button_code();

                let button_state = event.state();

                if ButtonState::Pressed == button_state && !pointer.is_grabbed() {
                    if let Some((window, _loc)) = self
                        .workspaces
                        .active()
                        .window_under(pointer.current_location())
                        .map(|(w, l)| (w.clone(), l))
                    {
                        self.workspaces.active_mut().raise_window(&window, true);
                        keyboard.set_focus(
                            self,
                            Some(window.toplevel().wl_surface().clone()),
                            serial,
                        );
                        self.workspaces.active().windows().for_each(|window| {
                            window.toplevel().send_pending_configure();
                        });
                    } else {
                        self.workspaces.active().windows().for_each(|window| {
                            window.set_activated(false);
                            window.toplevel().send_pending_configure();
                        });
                        keyboard.set_focus(self, Option::<WlSurface>::None, serial);
                    }
                };

                pointer.button(
                    self,
                    &ButtonEvent {
                        button,
                        state: button_state,
                        serial,
                        time: event.time_msec(),
                    },
                );
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
            Some(Action::SwitchToWorkspace(n)) => self.workspaces.activate(n),
            _ => (),
        }
        Ok(())
    }
}
