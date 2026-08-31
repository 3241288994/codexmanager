mod env;
mod lifecycle;
mod prompts;
mod startup;
mod state;
mod tray;
mod window;

pub(crate) use env::load_env_from_exe_dir;
pub(crate) use lifecycle::{handle_main_window_event, handle_run_event};
pub(crate) use startup::sync_startup_window_state;
pub(crate) use state::{
    prepare_for_forced_app_exit, set_unsaved_settings_draft_sections, CLOSE_TO_TRAY_ON_CLOSE,
    KEEP_ALIVE_FOR_LIGHTWEIGHT_CLOSE, LIGHTWEIGHT_MODE_ON_CLOSE_TO_TRAY, TRAY_AVAILABLE,
};
pub(crate) use tray::setup_tray;
pub(crate) use window::request_show_main_window;
