use std::process::Command;

use smithay::desktop::Window;
pub use workspaces::Workspaces;

mod focus;
mod fullscreen;
mod workspaces;

pub struct Shell {
    pub workspaces: Workspaces,
}

impl Shell {
    pub fn new(workspaces_count: usize) -> Self {
        assert!(workspaces_count > 0, "Workspaces count should be > 0");
        let workspaces = Workspaces::new(workspaces_count);
        Self { workspaces }
    }

    // FIXME: self
    pub fn close(&mut self, window: Option<Window>) {
        if let Some(window) = window {
            window.toplevel().expect("Wayland window").send_close();
        }
    }

    // FIXME: self
    pub fn spawn(&self, command: String) {
        std::thread::spawn(move || {
            let mut cmd = Command::new("/bin/sh");
            cmd.args(["-c", command.as_str()]);
            match cmd.spawn() {
                Ok(mut child) => {
                    let _ = child.wait();
                }
                Err(err) => tracing::error!(?err),
            }
        });
    }

    pub fn switch_to(&mut self, new: usize) {
        assert!(new > 0, "Workspace number should be > 0");
        self.workspaces.switch_to(new - 1);
    }

    pub fn move_to(&mut self, window: Option<Window>, new: usize) {
        assert!(new > 0, "Workspace number should be > 0");
        if let Some(window) = window {
            self.workspaces.move_to(window, new - 1);
        }
    }

    pub fn toggle_fullscreen(&mut self, window: Option<&Window>) {
        if let Some(window) = window {
            if self.workspaces.is_fullscreen(window) {
                self.workspaces.unfullscreen(&window.clone());
            } else {
                self.workspaces.fullscreen(window);
            }
        }
    }
}
