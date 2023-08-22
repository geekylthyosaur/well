use smithay::{
    backend::input::{InputBackend, InputEvent, KeyboardKeyEvent},
    input::keyboard::FilterResult,
};

use crate::state::State;

impl State {
    pub fn handle_input<I: InputBackend>(&mut self, event: InputEvent<I>) {
        let keyboard = self.seat.get_keyboard().unwrap();
        match event {
            InputEvent::Keyboard { event } => {
                keyboard.input::<(), _>(
                    self,
                    event.key_code(),
                    event.state(),
                    0.into(),
                    0,
                    |_, _, _| FilterResult::Forward,
                );
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
}
