use std::{os::fd::AsRawFd, sync::Arc, time::Duration};

use anyhow::{anyhow, Context, Result};
use smithay::{
    reexports::{
        calloop::{
            generic::Generic,
            timer::{TimeoutAction, Timer},
            EventLoop, Interest, Mode, PostAction,
        },
        wayland_server::Display,
    },
    wayland::socket::ListeningSocketSource,
};
use tracing::{error, info, warn};

use crate::{
    backend::WinitBackend,
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

    let state = State::new(&display.handle());
    let mut data = CalloopData { display, state };

    let mut event_loop = EventLoop::try_new().context("Failed to initialize event loop")?;

    let signal = event_loop.get_signal();
    let handle = event_loop.handle();

    let source = ListeningSocketSource::new_auto()?;
    let socket_name = source.socket_name().to_os_string();

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

    let mut backend = WinitBackend::new(&mut data);
    let timer = Timer::immediate();
    handle
        .insert_source(timer, move |_, _, data| {
            backend.dispatch(data).unwrap();
            TimeoutAction::ToDuration(Duration::from_secs_f32(1. / 60.))
        })
        .map_err(|_| anyhow!("Failed to initialize backend source"))?;

    std::env::set_var("WAYLAND_DISPLAY", socket_name.as_os_str());
    info!("Listening on {socket_name:?}");

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
