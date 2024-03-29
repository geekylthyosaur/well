use anyhow::Result;
use smithay::backend::input::{
    AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
    KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
};
use smithay::input::keyboard::FilterResult;
use smithay::input::pointer::{AxisFrame, ButtonEvent, MotionEvent};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::SERIAL_COUNTER;
use tracing::{debug, error};

use crate::config::Action;
use crate::state::State;

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
                let output_geo = self.shell.workspaces.output_geometry().unwrap();

                let point = event.position_transformed(output_geo.size);

                let serial = SERIAL_COUNTER.next_serial();

                let pointer = self.seat.get_pointer().unwrap();

                let under = self.shell.workspaces.current().surface_under(point);

                if let Some((window, _loc)) = self
                    .shell
                    .workspaces
                    .current()
                    .window_under(pointer.current_location())
                    .map(|(w, l)| (w.clone(), l))
                {
                    self.set_focus(Some(window));
                }

                pointer.motion(
                    self,
                    under,
                    &MotionEvent { location: point, serial, time: event.time_msec() },
                );
                pointer.frame(self);
            }
            InputEvent::PointerButton { event, .. } => {
                let pointer = self.seat.get_pointer().unwrap();
                let keyboard = self.seat.get_keyboard().unwrap();

                let serial = SERIAL_COUNTER.next_serial();

                let button = event.button_code();

                let button_state = event.state();

                if ButtonState::Pressed == button_state && !pointer.is_grabbed() {
                    if let Some((window, _loc)) = self
                        .shell
                        .workspaces
                        .current()
                        .window_under(pointer.current_location())
                        .map(|(w, l)| (w.clone(), l))
                    {
                        self.shell.workspaces.current_mut().raise_window(&window, true);
                        self.set_focus(Some(window));
                        self.shell.workspaces.current().windows().for_each(|window| {
                            window.toplevel().expect("Wayland window").send_pending_configure();
                        });
                    } else {
                        self.shell.workspaces.current().windows().for_each(|window| {
                            window.set_activated(false);
                            window.toplevel().expect("Wayland window").send_pending_configure();
                        });
                        keyboard.set_focus(self, Option::<WlSurface>::None, serial);
                    }
                };

                pointer.button(
                    self,
                    &ButtonEvent { button, state: button_state, serial, time: event.time_msec() },
                );
            }
            InputEvent::PointerAxis { event, .. } => {
                let source = event.source();

                let horizontal_amount = event
                    .amount(Axis::Horizontal)
                    .unwrap_or_else(|| event.amount(Axis::Horizontal).unwrap_or(0.0) * 3.0);
                let vertical_amount = event
                    .amount(Axis::Vertical)
                    .unwrap_or_else(|| event.amount(Axis::Vertical).unwrap_or(0.0) * 3.0);
                let horizontal_amount_discrete = event.amount(Axis::Horizontal);
                let vertical_amount_discrete = event.amount(Axis::Vertical);

                let mut frame = AxisFrame::new(event.time_msec()).source(source);
                if horizontal_amount != 0.0 {
                    frame = frame.value(Axis::Horizontal, horizontal_amount);
                    if let Some(discrete) = horizontal_amount_discrete {
                        frame = frame.v120(Axis::Horizontal, discrete as i32);
                    }
                }
                if vertical_amount != 0.0 {
                    frame = frame.value(Axis::Vertical, vertical_amount);
                    if let Some(discrete) = vertical_amount_discrete {
                        frame = frame.v120(Axis::Vertical, discrete as i32);
                    }
                }

                if source == AxisSource::Finger {
                    if event.amount(Axis::Horizontal) == Some(0.0) {
                        frame = frame.stop(Axis::Horizontal);
                    }
                    if event.amount(Axis::Vertical) == Some(0.0) {
                        frame = frame.stop(Axis::Vertical);
                    }
                }

                let pointer = self.seat.get_pointer().unwrap();
                pointer.axis(self, frame);
                pointer.frame(self);
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

        keyboard.input(self, code, state, serial, time, |data, modifiers, handle| {
            if state == KeyState::Pressed {
                let raw_syms = handle.raw_syms();
                let action = data.config.bindings.action(raw_syms, modifiers);
                if let Some(action) = action.as_ref() {
                    debug!(?action);
                }
                action.map(FilterResult::Intercept).unwrap_or(FilterResult::Forward)
            } else {
                FilterResult::Forward
            }
        })
    }

    fn process_action(&mut self, action: Option<Action>) -> Result<()> {
        match action {
            Some(Action::Exit) => self.is_running = false,
            Some(Action::Close) => {
                let window = self.get_focus();
                self.shell.close(window);
                let window = self.shell.workspaces.current().windows().next().cloned();
                self.set_focus(window);
            }
            Some(Action::Spawn(cmd)) => self.shell.spawn(cmd),
            Some(Action::SwitchToWorkspace(n)) => {
                self.shell.switch_to(n);
                let window = self.shell.workspaces.current().windows().next().cloned();
                self.set_focus(window);
            }
            Some(Action::MoveToWorkspace(n)) => {
                let window = self.get_focus();
                self.shell.move_to(window, n);
                self.set_focus(None);
            }
            Some(Action::ToggleFullscreen) => {
                let window = self.get_focus();
                self.shell.toggle_fullscreen(window.as_ref());
            }
            _ => (),
        }
        Ok(())
    }
}
