use std::time::Instant;

use smithay::{
    input::{Seat, SeatState},
    reexports::wayland_server::{
        backend::{ClientData, ClientId, DisconnectReason},
        Display, DisplayHandle,
    },
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        data_device::DataDeviceState,
        shell::xdg::XdgShellState,
        shm::ShmState,
    },
};

use crate::PKG_NAME;

pub struct State {
    pub is_running: bool,
    pub start_time: Instant,

    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub seat_state: SeatState<Self>,
    pub data_device_state: DataDeviceState,

    pub seat: Seat<Self>,
}

impl State {
    pub fn new(dh: &DisplayHandle) -> Self {
        let is_running = true;
        let start_time = Instant::now();

        let compositor_state = CompositorState::new::<State>(dh);
        let xdg_shell_state = XdgShellState::new::<State>(dh);
        let shm_state = ShmState::new::<State>(dh, vec![]);
        let mut seat_state = SeatState::new();
        let data_device_state = DataDeviceState::new::<State>(dh);

        let mut seat = seat_state.new_wl_seat(dh, PKG_NAME);

        let _ = seat.add_keyboard(Default::default(), 180, 60);

        Self {
            is_running,
            start_time,

            compositor_state,
            xdg_shell_state,
            shm_state,
            seat_state,
            data_device_state,

            seat,
        }
    }
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

pub struct CalloopData {
    pub display: Display<State>,
    pub state: State,
}