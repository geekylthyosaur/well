use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

use smithay::desktop::Window;
use smithay::utils::{Logical, Rectangle};

pub struct IsFullscreen(AtomicBool);

impl IsFullscreen {
    pub fn get(window: &Window) -> bool {
        window.user_data().get::<Self>().map(|d| d.0.load(Ordering::Relaxed)).unwrap_or_default()
    }

    pub fn set(window: &Window, is: bool) {
        if !window.user_data().insert_if_missing(|| Self(is.into())) {
            window.user_data().get::<Self>().map(|d| d.0.store(is, Ordering::Relaxed));
        }
    }
}

pub struct GeometryBeforeFullscreen(Cell<Rectangle<i32, Logical>>);

impl GeometryBeforeFullscreen {
    pub fn get(window: &Window) -> Option<Rectangle<i32, Logical>> {
        window.user_data().get::<Self>().map(|d| d.0.get())
    }

    pub fn set(window: &Window) {
        let geometry = window.geometry();
        if !window.user_data().insert_if_missing(|| Self(Cell::new(geometry))) {
            window.user_data().get::<Self>().map(|d| d.0.replace(geometry));
        }
    }
}
