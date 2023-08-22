use std::{os::fd::AsRawFd, str::FromStr, sync::Arc, time::Duration};

use anyhow::{anyhow, Context, Result};
use smithay::{
    backend::{renderer::gles::GlesRenderer, winit},
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
use tracing_subscriber::{filter::Directive, prelude::*, EnvFilter};

use crate::{
    backend::backend_dispatch,
    state::{CalloopData, ClientState, State},
};

mod backend;
mod handlers;
mod state;

const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

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
