use anyhow::Result;
use smithay::{
    backend::{
        input::{InputEvent, KeyboardKeyEvent},
        renderer::{
            element::surface::{render_elements_from_surface_tree, WaylandSurfaceRenderElement},
            gles::GlesRenderer,
            utils::draw_render_elements,
            Frame, Renderer,
        },
        winit::{WinitError, WinitEvent, WinitEventLoop, WinitGraphicsBackend},
    },
    input::keyboard::FilterResult,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Rectangle, Transform},
    wayland::compositor::{with_surface_tree_downward, SurfaceAttributes, TraversalAction},
};

use crate::state::CalloopData;

pub fn backend_dispatch(
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
