use chrono::{DateTime, Local};
use cosmic::{
    app::Core,
    iced::{self, futures::SinkExt, Alignment, Length, Subscription},
    theme,
    widget, Element, Task,
};
use std::collections::VecDeque;

const APP_ID: &str = "com.github.igris.ClipManager";
const MAX_HISTORY: usize = 50;
const NOTIFICATION_ID: &str = "41042";
const PANEL_PREVIEW_CHARS: usize = 14;
const POPUP_PREVIEW_CHARS: usize = 72;

#[derive(Clone)]
struct HistoryEntry {
    text: String,
    copied_at: DateTime<Local>,
}

pub struct AppModel {
    core: Core,
    popup: Option<iced::window::Id>,
    history: VecDeque<HistoryEntry>,
    current: String,
    search: String,
}

impl Default for AppModel {
    fn default() -> Self {
        Self {
            core: Core::default(),
            popup: None,
            history: VecDeque::new(),
            current: String::new(),
            search: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(iced::window::Id),
    ClipChanged(String),
    CopyEntry(usize),
    DeleteEntry(usize),
    ClearHistory,
    SearchChanged(String),
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<cosmic::Action<Self::Message>>) {
        (Self { core, ..Default::default() }, Task::none())
    }

    fn on_close_requested(&self, id: iced::window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let suggested = self.core.applet.suggested_size(false);
        let (major_padding, minor_padding) = self.core.applet.suggested_padding(false);
        let (horizontal_padding, vertical_padding) = if self.core.applet.is_horizontal() {
            (major_padding, minor_padding)
        } else {
            (minor_padding, major_padding)
        };

        let icon = widget::icon::from_name("edit-paste-symbolic")
            .size(suggested.1.saturating_sub(4));
        let preview = panel_preview(&self.current);
        let row = widget::row::with_children(vec![
            icon.into(),
            widget::text::body(preview)
                .align_y(iced::alignment::Vertical::Center)
                .into(),
        ])
        .spacing(6)
        .align_y(Alignment::Center);

        widget::button::custom(
            widget::container(row)
                .center_y(Length::Fixed(f32::from(suggested.1 + 2 * vertical_padding))),
        )
            .padding([0, horizontal_padding])
            .class(theme::Button::AppletIcon)
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: iced::window::Id) -> Element<'_, Self::Message> {
        let query = self.search.trim().to_lowercase();
        let mut list = widget::list::list_column();

        let search = widget::text_input::search_input("Search clipboard history", &self.search)
            .on_input(Message::SearchChanged)
            .padding(12);

        let header = widget::row::with_children(vec![
            widget::text::heading("Clipboard History").into(),
            widget::Space::new().width(Length::Fill).into(),
            widget::button::text("Clear")
                .on_press(Message::ClearHistory)
                .into(),
        ])
        .align_y(Alignment::Center);

        list = list.add(search);
        list = list.add(header);
        list = list.add(widget::divider::horizontal::default());

        let mut visible_entries = 0usize;
        let mut last_heading = String::new();

        for (index, entry) in self.history.iter().enumerate() {
            if !query.is_empty() && !entry.text.to_lowercase().contains(&query) {
                continue;
            }

            let heading = section_heading(entry.copied_at);
            if heading != last_heading {
                list = list.add(widget::text::caption(heading.clone()));
                last_heading = heading;
            }

            let text = widget::text::body(entry_preview(&entry.text)).width(Length::Fill);
            let actions = widget::row::with_children(vec![
                widget::button::icon(widget::icon::from_name("edit-copy-symbolic").size(16))
                    .on_press(Message::CopyEntry(index))
                    .into(),
                widget::button::icon(widget::icon::from_name("user-trash-symbolic").size(16))
                    .on_press(Message::DeleteEntry(index))
                    .into(),
            ])
            .spacing(4)
            .align_y(Alignment::Center);

            let row = widget::row::with_children(vec![text.into(), actions.into()])
                .spacing(12)
                .align_y(Alignment::Center);

            list = list.add(widget::container(row).padding([8, 4]));
            visible_entries += 1;
        }

        if visible_entries == 0 {
            let empty = if self.search.trim().is_empty() {
                "Nothing copied yet"
            } else {
                "No clipboard entries match this search"
            };
            list = list.add(widget::text::body(empty));
        }

        let content = widget::scrollable(
            widget::container(list)
                .padding([12, 16])
                .width(Length::Fill),
        )
        .height(Length::Shrink);

        self.core.applet.popup_container(content).into()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        clip_sub().map(Message::ClipChanged)
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::TogglePopup => {
                if let Some(id) = self.popup.take() {
                    return cosmic::iced::platform_specific::shell::commands::popup::destroy_popup(id);
                }

                let new_id = iced::window::Id::unique();
                self.popup.replace(new_id);

                let mut settings = self.core.applet.get_popup_settings(
                    self.core.main_window_id().unwrap(),
                    new_id,
                    None,
                    None,
                    None,
                );
                settings.positioner.size_limits = iced::Limits::NONE
                    .min_width(360.0)
                    .max_width(520.0)
                    .min_height(280.0)
                    .max_height(720.0);

                return cosmic::iced::platform_specific::shell::commands::popup::get_popup(settings);
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::ClipChanged(text) => {
                let cleaned = compact_text(&text);
                if cleaned.is_empty() || cleaned == self.current {
                    return Task::none();
                }

                self.current = cleaned.clone();
                self.history.retain(|entry| entry.text != cleaned);
                self.history.push_front(HistoryEntry {
                    text: cleaned,
                    copied_at: Local::now(),
                });

                while self.history.len() > MAX_HISTORY {
                    self.history.pop_back();
                }
                return notify_task("Copied to clipboard", &entry_preview(&self.current));
            }
            Message::CopyEntry(i) => {
                if let Some(entry) = self.history.get(i) {
                    let entry = entry.text.clone();
                    return Task::perform(
                        async move {
                            let _ = tokio::process::Command::new("wl-copy")
                                .arg(&entry)
                                .output()
                                .await;
                            entry
                        },
                        |txt| cosmic::Action::from(Message::ClipChanged(txt)),
                    );
                }
            }
            Message::DeleteEntry(i) => {
                if let Some(removed) = self.history.remove(i) {
                    if removed.text != self.current {
                        return Task::none();
                    }

                    self.current = self
                        .history
                        .front()
                        .map(|entry| entry.text.clone())
                        .unwrap_or_default();
                }
            }
            Message::ClearHistory => {
                self.history.clear();
                self.current.clear();
            }
            Message::SearchChanged(value) => self.search = value,
        }

        Task::none()
    }
}

fn clip_sub() -> Subscription<String> {
    Subscription::run(|| {
        iced::stream::channel(
            100,
            |mut out: iced::futures::channel::mpsc::Sender<String>| async move {
                let mut last_seen = String::new();

                loop {
                    if let Ok(output) = tokio::process::Command::new("wl-paste")
                        .arg("--no-newline")
                        .output()
                        .await
                    {
                        if output.status.success() {
                            let text = String::from_utf8_lossy(&output.stdout).to_string();
                            if !text.is_empty() && text != last_seen {
                                last_seen = text.clone();
                                let _ = out.send(text).await;
                            }
                        }
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(450)).await;
                }
            },
        )
    })
}

fn compact_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        text.to_string()
    } else {
        format!("{}…", text.chars().take(max_chars).collect::<String>())
    }
}

fn panel_preview(text: &str) -> String {
    let compact = compact_text(text);
    if compact.is_empty() {
        "Clipboard".to_string()
    } else {
        truncate_chars(&compact, PANEL_PREVIEW_CHARS)
    }
}

fn entry_preview(text: &str) -> String {
    truncate_chars(&compact_text(text), POPUP_PREVIEW_CHARS)
}

fn section_heading(copied_at: DateTime<Local>) -> String {
    let today = Local::now().date_naive();
    let entry_day = copied_at.date_naive();

    if entry_day == today {
        format!("Today • {}", copied_at.format("%A, %b %-d"))
    } else {
        copied_at.format("%A, %b %-d").to_string()
    }
}

fn notify_task(
    summary: &'static str,
    body: &str,
) -> Task<cosmic::Action<Message>> {
    let body = body.to_string();

    Task::perform(
        async move {
            let _ = tokio::process::Command::new("notify-send")
                .arg("--replace-id")
                .arg(NOTIFICATION_ID)
                .arg("--transient")
                .arg("--expire-time=1600")
                .arg("--app-name=Clipboard Applet")
                .arg("--icon=edit-paste-symbolic")
                .arg(summary)
                .arg(body)
                .output()
                .await;
        },
        |_| cosmic::Action::None,
    )
}
