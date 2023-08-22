use std::{
    os::fd::{AsRawFd, OwnedFd},
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use smithay::{
    backend::{
        input::{InputEvent, KeyboardKeyEvent},
        renderer::{
            element::surface::{render_elements_from_surface_tree, WaylandSurfaceRenderElement},
            gles::GlesRenderer,
            utils::{draw_render_elements, on_commit_buffer_handler},
            Frame, Renderer,
        },
        winit::{self, WinitError, WinitEvent, WinitEventLoop, WinitGraphicsBackend},
    },
    delegate_compositor, delegate_data_device, delegate_seat, delegate_shm, delegate_xdg_shell,
    input::{keyboard::FilterResult, pointer::CursorImageStatus, Seat, SeatHandler, SeatState},
    reexports::{
        calloop::{
            generic::Generic,
            timer::{TimeoutAction, Timer},
            EventLoop, Interest, Mode, PostAction,
        },
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::{wl_buffer, wl_seat, wl_surface::WlSurface},
            Client, Display, DisplayHandle,
        },
    },
    utils::{Rectangle, Serial, Transform},
    wayland::{
        buffer::BufferHandler,
        compositor::{
            with_surface_tree_downward, CompositorClientState, CompositorHandler, CompositorState,
            SurfaceAttributes, TraversalAction,
        },
        data_device::{
            ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
        },
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
        },
        shm::{ShmHandler, ShmState},
        socket::ListeningSocketSource,
    },
};
use tracing::{error, info, warn};
use tracing_subscriber::{filter::Directive, prelude::*, EnvFilter};

const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

struct State {
    is_running: bool,
    start_time: Instant,

    compositor_state: CompositorState,
    xdg_shell_state: XdgShellState,
    shm_state: ShmState,
    seat_state: SeatState<Self>,
    data_device_state: DataDeviceState,

    seat: Seat<Self>,
}

impl State {
    fn new(dh: &DisplayHandle) -> Self {
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

impl BufferHandler for State {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Activated);
        });
        surface.send_configure();
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {
        // TODO Handle popup creation here
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {
        // TODO Handle popup grab here
    }
}

impl DataDeviceHandler for State {
    type SelectionUserData = ();
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl ClientDndGrabHandler for State {}
impl ServerDndGrabHandler for State {
    fn send(&mut self, _mime_type: String, _fd: OwnedFd, _seat: Seat<Self>) {}
}

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
    }
}

impl ShmHandler for State {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl SeatHandler for State {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&WlSurface>) {}
    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: CursorImageStatus) {}
}

#[derive(Default)]
struct ClientState {
    compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

struct CalloopData {
    display: Display<State>,
    state: State,
}

fn main() -> Result<()> {
    let level = if cfg!(debug_assertions) { "debug" } else { "warn" };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level))
        .add_directive(Directive::from_str("calloop=error").unwrap())
        .add_directive(Directive::from_str(&format!("smithay={level}")).unwrap())
        .add_directive(Directive::from_str(&format!("{PKG_NAME}={level}")).unwrap());
    let fmt_layer = tracing_subscriber::fmt::layer().compact();
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(filter)
        .init();
    log_panics::init();

    info!("{PKG_NAME} {PKG_VERSION}");

    let mut event_loop = EventLoop::try_new().context("Failed to initialize event loop")?;

    let signal = event_loop.get_signal();
    let handle = event_loop.handle();

    let display = Display::new()?;

    let (mut backend, mut winit) =
        winit::init::<GlesRenderer>().map_err(|_| anyhow!("Failed to initialize backend"))?;

    let timer = Timer::immediate();
    handle
        .insert_source(timer, move |_, _, data| {
            backend_dispatch(&mut backend, &mut winit, data).unwrap();
            TimeoutAction::ToDuration(Duration::from_secs_f32(1. / 60.))
        })
        .map_err(|_| anyhow!("Failed to initialize backend source"))?;

    let source = ListeningSocketSource::new_auto()?;
    let socket_name = source.socket_name();
    std::env::set_var("WAYLAND_DISPLAY", socket_name);
    info!("Listening on {socket_name:?}");

    let state = State::new(&display.handle());
    let mut data = CalloopData { display, state };

    handle
        .insert_source(source, |client_stream, _, data: &mut CalloopData| {
            if let Err(err) = data
                .display
                .handle()
                .insert_client(client_stream, Arc::new(ClientState::default()))
            {
                warn!(?err, "Failed to add wayland client");
            }
        })
        .with_context(|| "Failed to initialize the wayland socket source")?;

    handle
        .insert_source(
            Generic::new(
                data.display.backend().poll_fd().as_raw_fd(),
                Interest::READ,
                Mode::Level,
            ),
            |_, _, data: &mut CalloopData| {
                data.display
                    .dispatch_clients(&mut data.state)
                    .map(|_| PostAction::Continue)
                    .map_err(|err| {
                        error!(?err, "Failed to dispatch wayland client");
                        data.state.is_running = false;
                        err
                    })
            },
        )
        .context("Failed to initialize the wayland event source")?;

    std::process::Command::new("alacritty").spawn()?;

    let timeout = None;
    event_loop.run(timeout, &mut data, move |data| {
        if !data.state.is_running {
            signal.stop();
            signal.wakeup();
            return;
        }

        let _ = data.display.flush_clients();
    })?;

    Ok(())
}

fn backend_dispatch(
    backend: &mut WinitGraphicsBackend<GlesRenderer>,
    winit: &mut WinitEventLoop,
    data: &mut CalloopData,
) -> Result<()> {
    let display = &mut data.display;
    let state = &mut data.state;

    let keyboard = state.seat.get_keyboard().unwrap();

    winit
        .dispatch_new_events(|event| match event {
            WinitEvent::Resized { .. } => {}
            WinitEvent::Input(event) => match event {
                InputEvent::Keyboard { event } => {
                    keyboard.input::<(), _>(
                        state,
                        event.key_code(),
                        event.state(),
                        0.into(),
                        0,
                        |_, _, _| FilterResult::Forward,
                    );
                }
                InputEvent::PointerMotionAbsolute { .. } => {
                    if let Some(surface) = state
                        .xdg_shell_state
                        .toplevel_surfaces()
                        .iter()
                        .next()
                        .cloned()
                    {
                        let surface = surface.wl_surface().clone();
                        keyboard.set_focus(state, Some(surface), 0.into());
                    };
                }
                _ => {}
            },
            _ => (),
        })
        .map_err(|err| {
            if matches!(err, WinitError::WindowClosed) {
                state.is_running = false
            }
            err
        })?;

    backend.bind()?;

    let size = backend.window_size().physical_size;
    let damage = Rectangle::from_loc_and_size((0, 0), size);

    let elements = state
        .xdg_shell_state
        .toplevel_surfaces()
        .iter()
        .flat_map(|surface| {
            render_elements_from_surface_tree(
                backend.renderer(),
                surface.wl_surface(),
                (0, 0),
                1.0,
                1.0,
            )
        })
        .collect::<Vec<WaylandSurfaceRenderElement<GlesRenderer>>>();

    let mut frame = backend
        .renderer()
        .render(size, Transform::Flipped180)
        .unwrap();
    frame.clear([0.6, 0.6, 0.6, 1.0], &[damage]).unwrap();
    draw_render_elements(&mut frame, 1.0, &elements, &[damage]).unwrap();
    // We rely on the nested compositor to do the sync for us
    let _ = frame.finish().unwrap();

    for surface in state.xdg_shell_state.toplevel_surfaces() {
        send_frames_surface_tree(
            surface.wl_surface(),
            state.start_time.elapsed().as_millis() as u32,
        );
    }

    display.dispatch_clients(state)?;
    // display.flush_clients()?;

    // It is important that all events on the display have been dispatched and flushed to clients before
    // swapping buffers because this operation may block.
    backend.submit(Some(&[damage])).unwrap();

    Ok(())
}

fn send_frames_surface_tree(surface: &WlSurface, time: u32) {
    with_surface_tree_downward(
        surface,
        (),
        |_, _, &()| TraversalAction::DoChildren(()),
        |_surf, states, &()| {
            // the surface may not have any user_data if it is a subsurface and has not
            // yet been commited
            for callback in states
                .cached_state
                .current::<SurfaceAttributes>()
                .frame_callbacks
                .drain(..)
            {
                callback.done(time);
            }
        },
        |_, _, &()| true,
    );
}

delegate_xdg_shell!(State);
delegate_compositor!(State);
delegate_shm!(State);
delegate_seat!(State);
delegate_data_device!(State);
