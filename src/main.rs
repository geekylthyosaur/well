use std::sync::Arc;

use anyhow::{Context, Result};
use smithay::{
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, Mode, PostAction},
        wayland_server::Display,
    },
    wayland::socket::ListeningSocketSource,
};
use tracing::{error, info, warn};

use crate::{
    backend::BackendState,
    state::{CalloopData, ClientState, State},
};

mod backend;
mod config;
mod handlers;
mod input;
mod logger;
mod render;
mod shell;
mod state;

const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    logger::init();

    let display = Display::new()?;

    let mut event_loop = EventLoop::try_new().context("Failed to initialize event loop")?;
    let signal = event_loop.get_signal();
    let handle = event_loop.handle();

    let state = State::new(&display.handle(), handle.clone());
    let backend = BackendState::new(handle.clone());
    let mut data = CalloopData { backend, state };
    BackendState::init(&mut data);

    let source = ListeningSocketSource::new_auto()?;
    let socket_name = source.socket_name().to_os_string();

    handle
        .insert_source(source, |client_stream, _, data: &mut CalloopData| {
            if let Err(err) = data
                .state
                .display_handle
                .insert_client(client_stream, Arc::new(ClientState::default()))
            {
                warn!(?err, "Failed to add wayland client");
            }
        })
        .with_context(|| "Failed to initialize the wayland socket source")?;

    let display_source = Generic::new(display, Interest::READ, Mode::Level);
    handle
        .insert_source(display_source, |_, display, data: &mut CalloopData| {
            unsafe { display.get_mut().dispatch_clients(&mut data.state) }
                .map(|_| PostAction::Continue)
                .map_err(|err| {
                    error!(?err, "Failed to dispatch wayland client");
                    data.state.is_running = false;
                    err
                })
        })
        .context("Failed to initialize the wayland event source")?;

    std::env::set_var("WAYLAND_DISPLAY", socket_name.as_os_str());
    info!("Listening on {socket_name:?}");

    let timeout = None;
    event_loop.run(timeout, &mut data, move |data| {
        if !data.state.is_running {
            signal.stop();
            signal.wakeup();
            return;
        }

        let _ = data.state.display_handle.flush_clients();
    })?;

    Ok(())
}
