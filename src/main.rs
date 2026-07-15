mod app;
mod storage;

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<app::AppModel>(())
}
