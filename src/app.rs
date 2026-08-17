use chrono::{DateTime, Duration, Local};
use cosmic::{
    app::Core,
    iced::{self, futures::SinkExt, Alignment, Length, Subscription},
    theme,
    widget, Element, Task,
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const APP_ID: &str = "com.github.igris.ClipManager";
const MAX_HISTORY: usize = 10000;
const MAX_AGE_HOURS: i64 = 48;
const NOTIFICATION_ID: &str = "41042";
const PANEL_PREVIEW_CHARS: usize = 14;
const POPUP_PREVIEW_CHARS: usize = 120;
const POPUP_WIDTH: f32 = 920.0;
const POPUP_HEIGHT: f32 = 640.0;
const MAX_HTML_BYTES: usize = 1024 * 1024;

// --- Glass design tokens ---
const RADIUS_POPUP: f32 = 24.0;
const RADIUS_CARD: f32 = 16.0;
const RADIUS_BTN: f32 = 16.0;
const OPACITY_CARD: f32 = 0.78;
const OPACITY_CARD_HOVER: f32 = 0.82;
const OPACITY_SECTION: f32 = 0.60;
const OPACITY_METADATA: f32 = 0.70;
const CARD_LUM_DARKEN: f32 = 0.92;
const CARD_LUM_HOVER: f32 = 0.96;
const BORDER_GLASS: f32 = 0.08;
const BORDER_GLASS_STRONG: f32 = 0.10;
const BORDER_DIVIDER: f32 = 0.06;
const SPACING_OUTER: f32 = 16.0;
const SPACING_CARDS: u16 = 12;
const SPACING_SEARCH: f32 = 42.0;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    text: String,
    #[serde(default)]
    html: Option<String>,
    kind: EntryKind,
    copied_at: DateTime<Local>,
    pinned: bool,
}

#[derive(Clone, Debug)]
pub struct ClipData {
    text: String,
    html: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryKind {
    Text,
    Url,
    Command,
    Code,
    Image,
    File,
    Color,
    Email,
}

impl EntryKind {
    fn label(self) -> &'static str {
        match self {
            EntryKind::Text => "Text",
            EntryKind::Url => "Link",
            EntryKind::Command => "Command",
            EntryKind::Code => "Code",
            EntryKind::Image => "Image",
            EntryKind::File => "File",
            EntryKind::Color => "Color",
            EntryKind::Email => "Email",
        }
    }
}

pub struct AppModel {
    core: Core,
    popup: Option<iced::window::Id>,
    history: VecDeque<HistoryEntry>,
    current: String,
    search: String,
    private_mode: bool,
    confirm_clear: bool,
}

impl Default for AppModel {
    fn default() -> Self {
        Self {
            core: Core::default(),
            popup: None,
            history: VecDeque::new(),
            current: String::new(),
            search: String::new(),
            private_mode: false,
            confirm_clear: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(iced::window::Id),
    ClipChanged(ClipData),
    ActivateEntry(usize),
    TogglePin(usize),
    DeleteEntry(usize),
    ClearHistory,
    SearchChanged(String),
    TogglePrivateMode,
    HistoryLoaded(Vec<HistoryEntry>),
    PruneTimer,
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
        let app = Self { core, ..Default::default() };
        let task = Task::perform(
            async { crate::storage::load_history().await },
            |entries| cosmic::Action::from(Message::HistoryLoaded(entries)),
        );
        (app, task)
    }

    fn on_close_requested(&self, id: iced::window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let suggested = self.core.applet.suggested_size(false);
        let icon = widget::icon::from_name("edit-paste-symbolic")
            .size(suggested.1.saturating_sub(4));

        let preview = widget::text::caption(panel_preview(&self.current)).size(12);
        let content = widget::row::with_children(vec![icon.into(), preview.into()])
            .spacing(6)
            .align_y(Alignment::Center);

        let btn = self.core
            .applet
            .button_from_element(content, true)
            .width(Length::Shrink)
            .on_press(Message::TogglePopup);

        self.core.applet.autosize_window(btn).into()
    }

    fn view_window(&self, _id: iced::window::Id) -> Element<'_, Self::Message> {
        let filtered_entries = self.filtered_entries();

        let search = widget::text_input::text_input("Type here to search...", &self.search)
            .on_input(Message::SearchChanged)
            .padding([12, 16])
            .size(14)
            .width(Length::Fill)
            .style(theme::TextInput::Search);

        let search_bar = widget::container(
            widget::row::with_children(vec![
                widget::icon::from_name("system-search-symbolic").size(18).into(),
                search.into(),
            ])
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fixed(SPACING_SEARCH))
        .padding([0, 20])
        .style(search_shell_style);

        let mut list_children: Vec<Element<'_, Message>> = Vec::new();
        if filtered_entries.is_empty() {
            list_children.push(self.empty_state());
        } else {
            let has_pinned = filtered_entries.iter().any(|(_, e)| e.pinned);
            let has_normal = filtered_entries.iter().any(|(_, e)| !e.pinned);

            let mut added_pinned = false;
            let mut added_normal = false;

            for (index, entry) in &filtered_entries {
                if has_pinned && entry.pinned && !added_pinned {
                    list_children.push(section_header("    PINNED"));
                    added_pinned = true;
                }
                if has_normal && !entry.pinned && !added_normal {
                    if added_pinned {
                        let div = widget::container(
                            widget::Space::new().width(Length::Fill).height(Length::Fixed(1.0)),
                        )
                        .width(Length::Fill)
                        .padding([0, 12])
                        .style(divider_style);
                        list_children.push(div.into());
                    }
                    list_children.push(section_header("    RECENT"));
                    added_normal = true;
                }
                list_children.push(self.history_row(entry, *index));
            }
        }

        let scrollable = widget::scrollable(
            widget::column::with_children(list_children).spacing(SPACING_CARDS),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .class(theme::style::iced::Scrollable::Minimal);

        let content = widget::column::with_children(vec![
            search_bar.into(),
            widget::Space::new().height(Length::Fixed(SPACING_OUTER)).into(),
            scrollable.into(),
            self.footer(filtered_entries.len()),
        ])
        .spacing(0);

        self.core
            .applet
            .popup_container(
                widget::container(content)
                    .padding(SPACING_OUTER)
                    .width(Length::Fixed(POPUP_WIDTH))
                    .height(Length::Fixed(POPUP_HEIGHT))
                    .style(popup_style),
            )
            .into()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            clip_sub().map(Message::ClipChanged),
            prune_sub().map(|_| Message::PruneTimer),
        ])
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::TogglePopup => {
                if let Some(id) = self.popup.take() {
                    return cosmic::iced::platform_specific::shell::commands::popup::destroy_popup(id);
                }

                let new_id = iced::window::Id::unique();
                self.popup.replace(new_id);

                let Some(main_window_id) = self.core.main_window_id() else {
                    tracing::error!("no main window id; cannot open popup");
                    self.popup = None;
                    return Task::none();
                };

                let mut settings = self.core.applet.get_popup_settings(
                    main_window_id,
                    new_id,
                    Some((POPUP_WIDTH as u32, POPUP_HEIGHT as u32)),
                    None,
                    None,
                );
                settings.positioner.size_limits = iced::Limits::NONE
                    .min_width(POPUP_WIDTH)
                    .max_width(POPUP_WIDTH)
                    .min_height(POPUP_HEIGHT)
                    .max_height(POPUP_HEIGHT);

                let mut tasks = vec![
                    cosmic::iced::platform_specific::shell::commands::popup::get_popup(settings),
                ];
                tasks.push(iced::window::enable_blur(new_id));
                return Task::batch(tasks);
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                    self.confirm_clear = false;
                }
            }
            Message::ClipChanged(data) => {
                let cleaned = data.text.trim().to_string();
                if cleaned.is_empty() || cleaned == self.current {
                    return Task::none();
                }

                self.current = cleaned.clone();

                if self.private_mode {
                    return Task::none();
                }

                let kind = detect_kind(&cleaned);
                let pinned = self
                    .history
                    .iter()
                    .find(|entry| entry.text == cleaned)
                    .map(|entry| entry.pinned)
                    .unwrap_or(false);

                self.history.retain(|entry| entry.text != cleaned);
                let html = data.html.clone();
                self.history.push_front(HistoryEntry {
                    text: cleaned,
                    html,
                    kind,
                    copied_at: Local::now(),
                    pinned,
                });

                self.prune_expired();

                while self.history.len() > MAX_HISTORY {
                    if let Some(idx) = self.history.iter().rposition(|e| !e.pinned) {
                        self.history.remove(idx);
                    } else {
                        break;
                    }
                }

                let current_text = self.current.clone();
                let current_html = data.html;
                let claim_clipboard = Task::perform(
                    async move {
                        wl_copy(&current_text, current_html.as_deref()).await;
                    },
                    |_| cosmic::Action::None,
                );

                return Task::batch(vec![
                    self.schedule_save(),
                    notify_task("Copied to clipboard", &entry_preview(&self.current), "edit-paste-symbolic"),
                    claim_clipboard,
                ]);
            }
            Message::ActivateEntry(i) => {
                if let Some(entry) = self.history.get(i) {
                    let entry_text = entry.text.clone();
                    let entry_html = entry.html.clone();
                    self.current = entry_text.clone();
                    return Task::perform(
                        async move {
                            wl_copy(&entry_text, entry_html.as_deref()).await;
                        },
                        |_| cosmic::Action::None,
                    );
                }
            }
            Message::TogglePin(i) => {
                if let Some(entry) = self.history.get(i) {
                    let new_pinned = !entry.pinned;
                    let (summary, icon) = if new_pinned {
                        ("Pinned to clipboard", "cpin-filled-symbolic")
                    } else {
                        ("Unpinned from clipboard", "cpin-outline-symbolic")
                    };
                    let body = entry_preview(&entry.text);
                    if let Some(entry) = self.history.get_mut(i) {
                        entry.pinned = new_pinned;
                    }
                    return Task::batch(vec![
                        self.schedule_save(),
                        notify_task(summary, &body, icon),
                    ]);
                }
                return self.schedule_save();
            }
            Message::DeleteEntry(i) => {
                let is_pinned = self.history.get(i).is_some_and(|e| e.pinned);
                if !is_pinned {
                    if let Some(removed) = self.history.remove(i) {
                        if removed.text == self.current {
                            self.current = self
                                .history
                                .front()
                                .map(|entry| entry.text.clone())
                                .unwrap_or_default();
                        }
                    }
                }
                return self.schedule_save();
            }
            Message::ClearHistory => {
                if self.confirm_clear {
                    self.history.retain(|e| e.pinned);
                    if !self.history.iter().any(|e| e.text == self.current) {
                        self.current.clear();
                    }
                    self.confirm_clear = false;
                    return self.schedule_save();
                } else {
                    self.confirm_clear = true;
                }
            }
            Message::PruneTimer => {
                if self.prune_expired() {
                    return self.schedule_save();
                }
            }
            Message::SearchChanged(value) => {
                self.search = value;
                self.confirm_clear = false;
            }
            Message::TogglePrivateMode => {
                self.private_mode = !self.private_mode;
            }
            Message::HistoryLoaded(entries) => {
                self.history = VecDeque::from(entries);
                let needs_save = self.prune_expired();
                self.current = self
                    .history
                    .front()
                    .map(|e| e.text.clone())
                    .unwrap_or_default();
                if needs_save {
                    return self.schedule_save();
                }
            }
        }

        Task::none()
    }
}

impl AppModel {
    fn prune_expired(&mut self) -> bool {
        let cutoff = Local::now() - Duration::hours(MAX_AGE_HOURS);
        let before = self.history.len();

        let unpinned_before = self.history.iter().filter(|e| !e.pinned).count();
        let to_remove = self.history.iter().filter(|e| !e.pinned && e.copied_at <= cutoff).count();

        if unpinned_before > 0 && to_remove as f64 / unpinned_before as f64 > 0.9 {
            tracing::warn!(
                "prune skipped — would remove {to_remove}/{unpinned_before} unpinned entries (>{:.0}%), cutoff={cutoff}",
                0.9 * 100.0,
            );
            return false;
        }

        self.history.retain(|entry| entry.pinned || entry.copied_at > cutoff);

        let removed = before - self.history.len();
        if removed > 0 {
            tracing::info!("pruned {removed} expired entries (kept {})", self.history.len());
            if !self.history.front().map(|e| e.text == self.current).unwrap_or(false) {
                self.current = self.history.front().map(|e| e.text.clone()).unwrap_or_default();
            }
            true
        } else {
            false
        }
    }

    fn schedule_save(&self) -> Task<cosmic::Action<Message>> {
        let entries: Vec<HistoryEntry> = self.history.iter().cloned().collect();
        Task::perform(
            async move { crate::storage::save_history(&entries).await },
            |_| cosmic::Action::None,
        )
    }

    fn filtered_entries(&self) -> Vec<(usize, &HistoryEntry)> {
        let query = self.search.trim().to_lowercase();
        let filtered_entries: Vec<_> = self
            .history
            .iter()
            .enumerate()
            .filter(|(_, entry)| query.is_empty() || entry.text.to_lowercase().contains(&query))
            .collect();

        let mut pinned = Vec::new();
        let mut normal = Vec::new();

        for entry in filtered_entries {
            if entry.1.pinned {
                pinned.push(entry);
            } else {
                normal.push(entry);
            }
        }

        pinned.into_iter().chain(normal).collect()
    }

    fn empty_state(&self) -> Element<'_, Message> {
        let (title, caption) = if self.history.is_empty() {
            ("Nothing copied yet", "Anything you copy will appear here.")
        } else {
            ("No results", "Try a different search term.")
        };

        let icon = widget::icon::from_name("edit-paste-symbolic").size(48);
        let icon_container = widget::container(icon)
            .width(72)
            .height(72)
            .style(|theme| {
                let cosmic = theme.cosmic();
                let base: iced::Color = cosmic.background(theme.transparent).base.into();
                let divider: iced::Color = cosmic.background(theme.transparent).divider.into();
                iced::widget::container::Style {
                    background: Some(iced::Background::Color(iced::Color { a: 0.08, ..base })),
                    border: iced::Border {
                        radius: 20.0.into(),
                        width: 0.5,
                        color: divider,
                    },
                    ..Default::default()
                }
            });

        widget::container(
            widget::column::with_children(vec![
                icon_container.into(),
                widget::Space::new().height(Length::Fixed(16.0)).into(),
                widget::text::body(title).size(18).into(),
                widget::Space::new().height(Length::Fixed(6.0)).into(),
                widget::text::caption(caption).size(14).into(),
            ])
            .width(Length::Fill)
            .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([40, 0])
        .style(popup_text_style)
        .into()
    }

    fn footer(&self, result_count: usize) -> Element<'_, Message> {
        let divider = widget::container(
            widget::Space::new().width(Length::Fill).height(Length::Fixed(1.0)),
        )
        .width(Length::Fill)
        .style(|_| {
            iced::widget::container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(1.0, 1.0, 1.0, BORDER_DIVIDER))),
                ..Default::default()
            }
        });

        let info = widget::text::caption(format!("{} items", result_count))
            .size(13);

        let private_toggle = widget::row::with_children(vec![
            widget::text::body("Private mode").size(14).into(),
            widget::toggler(self.private_mode)
                .size(30)
                .on_toggle(|_| Message::TogglePrivateMode)
                .into(),
        ])
        .spacing(8)
        .align_y(Alignment::Center);

        let trash_icon = if self.confirm_clear {
            "dialog-warning-symbolic"
        } else {
            "user-trash-symbolic"
        };

        let inner = widget::row::with_children(vec![
            info.into(),
            widget::Space::new().width(Length::Fill).into(),
            private_toggle.into(),
            widget::Space::new().width(Length::Fixed(12.0)).into(),
            icon_button(trash_icon, false).on_press(Message::ClearHistory).into(),
        ])
        .align_y(Alignment::Center);

        widget::column::with_children(vec![
            divider.into(),
            widget::container(inner)
                .width(Length::Fill)
                .padding([16, 20])
                .into(),
        ])
        .into()
    }

    fn history_row(&self, entry: &HistoryEntry, index: usize) -> Element<'_, Message> {
        let is_active = entry.text == self.current;

        let text_column = widget::column::with_children(vec![
            widget::text::body(entry_preview(&entry.text))
                .size(15)
                .width(Length::Fill)
                .into(),
            widget::Space::new().height(Length::Fixed(4.0)).into(),
            widget::container(
                widget::row::with_children(vec![
                    widget::text::caption(entry.kind.label()).size(12).into(),
                    widget::Space::new().width(Length::Fixed(8.0)).into(),
                    widget::text::caption(time_ago(entry.copied_at)).size(12).into(),
                ])
            )
            .style(|theme| {
                let on: iced::Color = theme.cosmic().background(false).on.into();
                iced::widget::container::Style {
                    text_color: Some(iced::Color { a: OPACITY_METADATA, ..on }),
                    ..Default::default()
                }
            })
            .into(),
        ])
        .width(Length::Fill)
        .spacing(2);

        let pin_icon = if entry.pinned {
            "cpin-filled-symbolic"
        } else {
            "cpin-outline-symbolic"
        };

        let mut action_children: Vec<Element<_>> = vec![
            icon_button(pin_icon, entry.pinned).on_press(Message::TogglePin(index)).into(),
        ];
        if !entry.pinned {
            action_children.push(widget::Space::new().width(Length::Fixed(4.0)).into());
            action_children.push(icon_button("user-trash-symbolic", false).on_press(Message::DeleteEntry(index)).into());
        }
        let actions = widget::row::with_children(action_children)
            .align_y(Alignment::Center);

        let activate = widget::button::custom(
            widget::row::with_children(vec![
                text_column.into(),
            ])
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .padding([12, 16])
        .width(Length::Fill)
        .class(theme::Button::Custom {
            active: Box::new(move |focused, theme| {
                let cosmic = theme.cosmic();
                let on: iced::Color = cosmic.background(false).on.into();
                let base: iced::Color = cosmic.background(theme.transparent).base.into();
                let (r, g, b) = if is_active || focused {
                    (base.r * CARD_LUM_HOVER, base.g * CARD_LUM_HOVER, base.b * CARD_LUM_HOVER)
                } else {
                    (base.r * CARD_LUM_DARKEN, base.g * CARD_LUM_DARKEN, base.b * CARD_LUM_DARKEN)
                };
                let alpha = if is_active { OPACITY_CARD_HOVER } else { OPACITY_CARD };
                widget::button::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgba(r, g, b, alpha))),
                    border_radius: RADIUS_CARD.into(),
                    shadow_offset: iced::Vector::new(0.0, if focused || is_active { 2.0 } else { 0.0 }),
                    text_color: Some(on),
                    icon_color: Some(on),
                    ..Default::default()
                }
            }),
            disabled: Box::new(move |theme| {
                let cosmic = theme.cosmic();
                let on: iced::Color = cosmic.background(false).on.into();
                let base: iced::Color = cosmic.background(theme.transparent).base.into();
                widget::button::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgba(
                        base.r * CARD_LUM_DARKEN, base.g * CARD_LUM_DARKEN, base.b * CARD_LUM_DARKEN, OPACITY_CARD,
                    ))),
                    border_radius: RADIUS_CARD.into(),
                    text_color: Some(on),
                    icon_color: Some(on),
                    ..Default::default()
                }
            }),
            hovered: Box::new(move |_focused, theme| {
                let cosmic = theme.cosmic();
                let on: iced::Color = cosmic.background(false).on.into();
                let base: iced::Color = cosmic.background(theme.transparent).base.into();
                let (r, g, b) = if is_active {
                    (base.r * CARD_LUM_HOVER, base.g * CARD_LUM_HOVER, base.b * CARD_LUM_HOVER)
                } else {
                    (base.r * CARD_LUM_HOVER, base.g * CARD_LUM_HOVER, base.b * CARD_LUM_HOVER)
                };
                let alpha = if is_active { OPACITY_CARD_HOVER } else { OPACITY_CARD_HOVER };
                widget::button::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgba(r, g, b, alpha))),
                    border_radius: RADIUS_CARD.into(),
                    shadow_offset: iced::Vector::new(0.0, 2.0),
                    text_color: Some(on),
                    icon_color: Some(on),
                    ..Default::default()
                }
            }),
            pressed: Box::new(move |_focused, theme| {
                let cosmic = theme.cosmic();
                let on: iced::Color = cosmic.background(false).on.into();
                let base: iced::Color = cosmic.background(theme.transparent).base.into();
                widget::button::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgba(
                        base.r * CARD_LUM_HOVER, base.g * CARD_LUM_HOVER, base.b * CARD_LUM_HOVER, OPACITY_CARD_HOVER + 0.04,
                    ))),
                    border_radius: RADIUS_CARD.into(),
                    text_color: Some(on),
                    icon_color: Some(on),
                    ..Default::default()
                }
            }),
        })
        .on_press(Message::ActivateEntry(index));

        widget::container(
            widget::row::with_children(vec![
                activate.into(),
                widget::Space::new().width(Length::Fixed(8.0)).into(),
                actions.into(),
            ])
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .style(card_default)
        .into()
    }
}

fn card_default(theme: &cosmic::Theme) -> iced::widget::container::Style {
    let cosmic = theme.cosmic();
    let base: iced::Color = cosmic.background(theme.transparent).base.into();
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgba(
            base.r * CARD_LUM_DARKEN,
            base.g * CARD_LUM_DARKEN,
            base.b * CARD_LUM_DARKEN,
            OPACITY_CARD,
        ))),
        border: iced::Border {
            radius: RADIUS_CARD.into(),
            width: 0.0,
            color: iced::Color::TRANSPARENT,
        },
        ..Default::default()
    }
}

fn icon_button<'a>(icon_name: &'static str, pinned: bool) -> widget::Button<'a, Message> {
    widget::button::custom(widget::icon::from_name(icon_name).size(18))
        .class(theme::Button::Custom {
            active: Box::new(move |focused, theme| glass_icon_style(focused, pinned, theme)),
            disabled: Box::new(move |theme| glass_icon_style(false, pinned, theme)),
            hovered: Box::new(move |focused, theme| glass_icon_style(focused, pinned, theme)),
            pressed: Box::new(move |focused, theme| glass_icon_style(focused, pinned, theme)),
        })
        .padding([8, 8])
}

fn glass_icon_style(focused: bool, pinned: bool, theme: &cosmic::Theme) -> widget::button::Style {
    let on: iced::Color = theme.cosmic().background(false).on.into();
    let accent: iced::Color = theme.cosmic().accent.base.into();

    let alpha = if pinned {
        0.08
    } else if focused {
        0.06
    } else {
        0.04
    };
    let shadow_offset = if focused { 3.0 } else { 1.0 };

    widget::button::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgba(1.0, 1.0, 1.0, alpha))),
        border_radius: RADIUS_BTN.into(),
        border_width: 0.0,
        shadow_offset: iced::Vector::new(0.0, shadow_offset),
        text_color: Some(on),
        icon_color: Some(if pinned { accent } else { on }),
        ..Default::default()
    }
}

fn section_header(label: &'static str) -> Element<'static, Message> {
    widget::container(
        widget::text::caption(label)
            .size(13)
            .width(Length::Fill),
    )
    .padding([18, 12, 8, 12])
    .width(Length::Fill)
    .style(|theme| {
        let on: iced::Color = theme.cosmic().background(false).on.into();
        iced::widget::container::Style {
            text_color: Some(iced::Color { a: OPACITY_SECTION, ..on }),
            ..Default::default()
        }
    })
    .into()
}

fn popup_style(theme: &cosmic::Theme) -> iced::widget::container::Style {
    let cosmic = theme.cosmic();
    let on: iced::Color = cosmic.background(false).on.into();

    let bg = if theme.transparent {
        let base: iced::Color = cosmic.background(true).base.into();
        iced::Background::Color(base)
    } else {
        let base: iced::Color = cosmic.background(false).base.into();
        iced::Background::Color(iced::Color { a: 0.95, ..base })
    };

    iced::widget::container::Style {
        background: Some(bg),
        text_color: Some(on),
        border: iced::Border {
            radius: RADIUS_POPUP.into(),
            width: 1.0,
            color: iced::Color::from_rgba(1.0, 1.0, 1.0, BORDER_GLASS),
        },
        shadow: iced::Shadow {
            color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.25),
            offset: iced::Vector::new(0.0, 16.0),
            blur_radius: 40.0,
        },
        ..Default::default()
    }
}

fn popup_text_style(theme: &cosmic::Theme) -> iced::widget::container::Style {
    let on: iced::Color = theme.cosmic().background(false).on.into();
    iced::widget::container::Style {
        text_color: Some(on),
        ..Default::default()
    }
}

fn divider_style(_theme: &cosmic::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgba(1.0, 1.0, 1.0, BORDER_DIVIDER))),
        ..Default::default()
    }
}

fn search_shell_style(theme: &cosmic::Theme) -> iced::widget::container::Style {
    let on: iced::Color = theme.cosmic().background(false).on.into();

    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgba(on.r, on.g, on.b, 0.05))),
        text_color: Some(on),
        border: iced::Border {
            radius: RADIUS_POPUP.into(),
            width: 1.0,
            color: iced::Color::from_rgba(1.0, 1.0, 1.0, BORDER_GLASS_STRONG),
        },
        ..Default::default()
    }
}


fn detect_kind(text: &str) -> EntryKind {
    let trimmed = text.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        EntryKind::Url
    } else if trimmed.contains('\n') || trimmed.contains("fn ") || trimmed.contains("let ") {
        EntryKind::Code
    } else if trimmed.starts_with("#") || trimmed.starts_with("rgb") || trimmed.starts_with("hsl") {
        EntryKind::Color
    } else if trimmed.contains('@') && trimmed.contains('.') {
        EntryKind::Email
    } else {
        EntryKind::Text
    }
}

fn clip_sub() -> Subscription<ClipData> {
    Subscription::run(|| {
        iced::stream::channel(
            100,
            |mut out: iced::futures::channel::mpsc::Sender<ClipData>| async move {
                let mut last_seen = String::new();

                loop {
                    if let Some(data) = read_clipboard().await {
                        if !data.text.is_empty() && data.text != last_seen {
                            last_seen = data.text.clone();
                            let _ = out.send(data).await;
                        }
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(450)).await;
                }
            },
        )
    })
}

async fn read_clipboard() -> Option<ClipData> {
    let types_output = tokio::process::Command::new("wl-paste")
        .arg("--list-types")
        .output()
        .await
        .ok()?;
    if !types_output.status.success() {
        return None;
    }
    let types = String::from_utf8_lossy(&types_output.stdout);
    let has_plain = types.lines().any(|t| t.starts_with("text/plain"));
    let has_html = types.lines().any(|t| t.starts_with("text/html"));
    if !has_plain && !has_html {
        return None;
    }

    let text = if has_plain {
        let out = tokio::process::Command::new("wl-paste")
            .arg("--no-newline")
            .output()
            .await
            .ok()?;
        if !out.status.success() {
            return None;
        }
        String::from_utf8_lossy(&out.stdout).to_string()
    } else {
        String::new()
    };

    let html = if has_html {
        let out = tokio::process::Command::new("wl-paste")
            .arg("--no-newline")
            .arg("--type")
            .arg("text/html")
            .output()
            .await
            .ok()?;
        if out.status.success() && out.stdout.len() <= MAX_HTML_BYTES {
            Some(String::from_utf8_lossy(&out.stdout).to_string())
        } else {
            None
        }
    } else {
        None
    };

    Some(ClipData { text, html })
}

fn prune_sub() -> Subscription<()> {
    Subscription::run(|| {
        iced::stream::channel(
            100,
            |mut out: iced::futures::channel::mpsc::Sender<()>| async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1800)).await;
                    let _ = out.send(()).await;
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
    let collapsed = compact_text(text).replace('\n', " ");
    let preview = collapsed.trim();
    if preview.starts_with("http://") || preview.starts_with("https://") {
        let trimmed = preview.split('/').take(3).collect::<Vec<_>>().join("/");
        truncate_chars(&trimmed, POPUP_PREVIEW_CHARS)
    } else {
        truncate_chars(preview, POPUP_PREVIEW_CHARS)
    }
}

fn time_ago(copied_at: DateTime<Local>) -> String {
    let now = Local::now();
    let duration = now.signed_duration_since(copied_at);

    if duration.num_minutes() < 1 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{} minutes ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{} hours ago", duration.num_hours())
    } else {
        format!("{} days ago", duration.num_days())
    }
}

async fn wl_copy(text: &str, html: Option<&str>) {
    let mut sources = vec![wl_clipboard_rs::copy::MimeSource {
        source: wl_clipboard_rs::copy::Source::Bytes(text.as_bytes().to_vec().into()),
        mime_type: wl_clipboard_rs::copy::MimeType::Text,
    }];
    if let Some(html) = html {
        sources.push(wl_clipboard_rs::copy::MimeSource {
            source: wl_clipboard_rs::copy::Source::Bytes(html.as_bytes().to_vec().into()),
            mime_type: wl_clipboard_rs::copy::MimeType::Specific("text/html".to_string()),
        });
    }

    tokio::task::spawn_blocking(move || {
        if let Err(e) = wl_clipboard_rs::copy::copy_multi(wl_clipboard_rs::copy::Options::new(), sources)
        {
            tracing::warn!("wl-clipboard-rs copy_multi failed: {e}");
        }
    })
    .await
    .ok();
}

fn notify_task(summary: &'static str, body: &str, icon: &'static str) -> Task<cosmic::Action<Message>> {
    let body = body.to_string();

    Task::perform(
        async move {
            let _ = tokio::process::Command::new("notify-send")
                .arg("--replace-id")
                .arg(NOTIFICATION_ID)
                .arg("--transient")
                .arg("--expire-time=1600")
                .arg("--app-name=Clipboard Applet")
                .arg("--icon")
                .arg(icon)
                .arg(summary)
                .arg(body)
                .output()
                .await;
        },
        |_| cosmic::Action::None,
    )
}
