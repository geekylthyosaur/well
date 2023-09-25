use std::cell::RefCell;

use smithay::{
    desktop::Window,
    utils::{Logical, Rectangle},
};

pub struct IsFullscreen(RefCell<bool>);

impl IsFullscreen {
    pub fn get(window: &Window) -> bool {
        matches!(window.user_data().get::<Self>().map(|d| *d.0.borrow()), Some(true))
    }

    pub fn set(window: &Window, is: bool) {
        if !window.user_data().insert_if_missing(|| Self(RefCell::new(is))) {
            window.user_data().get::<Self>().unwrap().0.replace(is);
        }
    }
}

pub struct GeometryBeforeFullscreen(RefCell<Rectangle<i32, Logical>>);

impl GeometryBeforeFullscreen {
    pub fn get(window: &Window) -> Option<Rectangle<i32, Logical>> {
        window.user_data().get::<Self>().map(|d| *d.0.borrow())
    }

    pub fn set(window: &Window, geometry: Rectangle<i32, Logical>) {
        if !window.user_data().insert_if_missing(|| Self(RefCell::new(geometry))) {
            window.user_data().get::<Self>().unwrap().0.replace(geometry);
        }
    }
}
