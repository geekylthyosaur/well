use std::time::Instant;

use smithay::{
    desktop::PopupManager,
    input::{Seat, SeatState},
    reexports::{
        calloop::LoopHandle,
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            DisplayHandle,
        },
    },
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        data_device::DataDeviceState,
        shell::xdg::{decoration::XdgDecorationState, XdgShellState},
        shm::ShmState,
    },
};

use crate::{backend::Backend, config::Config, shell::Shell, PKG_NAME};

pub struct CalloopData {
    pub backend: Backend,
    pub state: State,
}

pub struct State {
    pub is_running: bool,
    pub start_time: Instant,

    pub display_handle: DisplayHandle,
    pub event_loop: LoopHandle<'static, CalloopData>,

    pub config: Config,
    pub popups: PopupManager,
    pub shell: Shell,

    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub seat_state: SeatState<Self>,
    pub data_device_state: DataDeviceState,
    pub xdg_decoration_state: XdgDecorationState,

    pub seat: Seat<Self>,
}

impl State {
    pub fn new(dh: &DisplayHandle, event_loop: LoopHandle<'static, CalloopData>) -> Self {
        let is_running = true;
        let start_time = Instant::now();

        let display_handle = dh.clone();

        let config = Config::load().unwrap();
        let popups = PopupManager::default();
        let shell = Shell::new(config.workspace_count);

        let compositor_state = CompositorState::new::<State>(dh);
        let xdg_shell_state = XdgShellState::new::<State>(dh);
        let shm_state = ShmState::new::<State>(dh, vec![]);
        let mut seat_state = SeatState::new();
        let data_device_state = DataDeviceState::new::<State>(dh);
        let xdg_decoration_state = XdgDecorationState::new::<State>(dh);

        let mut seat = seat_state.new_wl_seat(dh, PKG_NAME);

        let _ = seat.add_keyboard(Default::default(), 180, 60);
        let _ = seat.add_pointer();

        Self {
            is_running,
            start_time,

            display_handle,
            event_loop,

            config,
            popups,
            shell,

            compositor_state,
            xdg_shell_state,
            shm_state,
            seat_state,
            data_device_state,
            xdg_decoration_state,

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
