use std::{borrow::BorrowMut, os::fd::AsRawFd, str::FromStr, sync::Arc};

use anyhow::{Context, Result};
use smithay::{
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, PostAction},
        wayland_server::{backend::ClientData, Display},
    },
    wayland::socket::ListeningSocketSource,
};
use tracing::{error, info, warn};
use tracing_subscriber::{filter::Directive, prelude::*, EnvFilter};

const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

struct State {
    is_running: bool,
}

struct ClientState;

impl ClientData for ClientState {}

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

    let source = ListeningSocketSource::new_auto()?;
    let socket_name = source.socket_name();
    info!("Listening on {socket_name:?}");

    let state = State { is_running: true };
    let mut data = CalloopData { display, state };

    handle
        .insert_source(source, |client_stream, _, data: &mut CalloopData| {
            if let Err(err) = data
                .display
                .handle()
                .insert_client(client_stream, Arc::new(ClientState))
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
                smithay::reexports::calloop::Mode::Level,
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

    let timeout = None;
    event_loop.run(timeout, &mut data, move |data| {
        if !data.state.is_running {
            signal.stop();
            signal.wakeup();
            return;
        }

        let display = data.display.borrow_mut();
        let _ = display.flush_clients();
    })?;

    Ok(())
}
