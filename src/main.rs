mod core;
mod i18n;
mod ui;

use core::app::Dalhin;

fn main() -> iced::Result {
    iced::application("Dalhin", Dalhin::update, Dalhin::view)
        .theme(core::app::theme)
        .run_with(Dalhin::new)
}
