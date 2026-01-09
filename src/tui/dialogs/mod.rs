//! TUI dialog components

mod confirm;
mod new_session;

pub use confirm::ConfirmDialog;
pub use new_session::{NewSessionData, NewSessionDialog};

pub enum DialogResult<T> {
    Continue,
    Cancel,
    Submit(T),
}
