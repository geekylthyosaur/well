use std::cell::RefCell;

use smithay::desktop::Window;
use smithay::input::Seat;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::SERIAL_COUNTER;

use crate::state::State;

struct CurrentFocus(RefCell<Option<Window>>);

impl CurrentFocus {
    fn get(seat: &Seat<State>) -> Option<Window> {
        seat.user_data().get::<Self>().and_then(|d| d.0.borrow().clone())
    }

    fn set(seat: &Seat<State>, window: Option<Window>) {
        if !seat.user_data().insert_if_missing(|| Self(RefCell::new(window.clone()))) {
            seat.user_data().get::<Self>().unwrap().0.replace(window);
        }
    }
}

impl State {
    pub fn get_focus(&self) -> Option<Window> {
        CurrentFocus::get(&self.seat)
    }

    pub fn set_focus(&mut self, window: Option<Window>) -> Option<()> {
        let surface = window.as_ref()?.toplevel()?.wl_surface();
        set_keyboard_focus(self, surface.clone());
        CurrentFocus::set(&self.seat, window);
        Some(())
    }
}

fn set_keyboard_focus(state: &mut State, surface: WlSurface) {
    let serial = SERIAL_COUNTER.next_serial();

    if let Some(handle) = state.seat.get_keyboard() {
        handle.set_focus(state, Some(surface), serial);
    }
}
