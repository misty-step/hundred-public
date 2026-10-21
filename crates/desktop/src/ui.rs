use anyhow::Result;
use chrono::{Duration as Days, Local};
use gpui::{prelude::*, *};
use hundred_client::Store;
use hundred_core::{Change, Event, Exercise, GOAL, active_sets, pigment, progress, remaining};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

const BG: u32 = 0x181c24;
const SURFACE: u32 = 0x232935;
const TEXT: u32 = 0xe1e6ef;
const MUTED: u32 = 0x9daac0;
const BORDER: u32 = 0x46536a;
const COLOURS: [u32; 3] = [0xeca6b1, 0xe9c27b, 0xb4b9f0];

actions!(
    hundred,
    [
        Next,
        Previous,
        PushUps,
        Crunches,
        Squats,
        Save,
        Increase,
        Decrease,
        Undo,
        Sync,
        History,
        Back,
        Quit,
        SelectNumber,
        PasteNumber
    ]
);

enum Command {
    Add(Exercise, u16),
    Undo(Option<String>),
    Sync,
    SyncDone(Result<(), String>),
    Refresh,
}
struct Update {
    events: Vec<Event>,
    pending: usize,
    /// Only a command's result; a periodic refresh must not replace the
    /// window's newer local notice (such as a refused rep count).
    notice: Option<String>,
    sync_label: String,
    saved: bool,
}

fn controller(
    directory: PathBuf,
    server: String,
    token: String,
) -> (Sender<Command>, Receiver<Update>) {
    let (commands, incoming) = mpsc::channel();
    let (updates, outgoing) = mpsc::channel();
    let sender = commands.clone();
    thread::spawn(move || {
        let mut store = match Store::open(&directory) {
            Ok(store) => store,
            Err(error) => {
                let _ = updates.send(Update {
                    events: vec![],
                    pending: 0,
                    notice: Some(format!("Cannot open local history: {error}")),
                    sync_label: "Storage unavailable".into(),
                    saved: false,
                });
                return;
            }
        };
        let configured = !server.trim().is_empty() && !token.trim().is_empty();
        let mut sync_label = if configured {
            "Not synced yet"
        } else {
            "Local only · sync not configured"
        }
        .to_owned();
        let mut notice = if store.events().is_ok_and(|events| !events.is_empty()) {
            "Ready for your next set."
        } else {
            "Your first set starts here."
        }
        .to_owned();
        let mut notice_changed = true;
        let mut syncing = false;
        let mut last_sync = Instant::now() - Duration::from_secs(30);
        // The first pass answers at once, so the starting notice cannot arrive
        // after (and replace) a notice the window set in its first seconds.
        let mut wait = Duration::ZERO;
        loop {
            let command = incoming.recv_timeout(wait);
            wait = Duration::from_secs(3);
            let mut saved = false;
            let mut should_sync = last_sync.elapsed() >= Duration::from_secs(30);
            match command {
                Ok(Command::Add(exercise, reps)) => {
                    notice_changed = true;
                    match store.add(exercise, reps, None) {
                        Ok(event) => {
                            let totals = store.events().map(|events| {
                                progress(&events, event.day())
                                    .days
                                    .get(&event.day())
                                    .copied()
                                    .unwrap_or_default()
                            });
                            notice = totals.map_or_else(
                                |_| "Saved here.".into(),
                                |totals| {
                                    format!(
                                        "Saved {reps} {} · {} left",
                                        exercise.label().to_lowercase(),
                                        remaining(totals[exercise.index()])
                                    )
                                },
                            );
                            saved = true;
                            should_sync = true;
                        }
                        Err(error) => notice = format!("Not saved: {error}"),
                    }
                }
                Ok(Command::Undo(id)) => {
                    notice_changed = true;
                    notice = match store.undo(id.as_deref()) {
                        Ok(_) => "Set undone. Saved here.".into(),
                        Err(error) => error.to_string(),
                    };
                    should_sync = true;
                }
                Ok(Command::Sync) => should_sync = true,
                Ok(Command::SyncDone(result)) => {
                    syncing = false;
                    sync_label = match result {
                        Ok(()) => "Synced".into(),
                        Err(error) => error,
                    };
                }
                Ok(Command::Refresh) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
            if configured && should_sync && !syncing {
                syncing = true;
                last_sync = Instant::now();
                sync_label = "Syncing".into();
                let directory = directory.clone();
                let server = server.clone();
                let token = token.clone();
                let sender = sender.clone();
                thread::spawn(move || {
                    let result = Store::open(&directory)
                        .and_then(|mut store| hundred_client::sync(&mut store, &server, &token));
                    let _ =
                        sender.send(Command::SyncDone(result.map_err(|error| error.to_string())));
                });
            }
            let events = match store.events() {
                Ok(events) => events,
                Err(error) => {
                    notice = format!("Cannot read local history: {error}");
                    notice_changed = true;
                    vec![]
                }
            };
            let pending = store.pending().unwrap_or(0);
            let label = if pending > 0 && sync_label == "Synced" {
                format!("Saved here · {pending} waiting to sync")
            } else {
                sync_label.clone()
            };
            if updates
                .send(Update {
                    events,
                    pending,
                    notice: notice_changed.then(|| notice.clone()),
                    sync_label: label,
                    saved,
                })
                .is_err()
            {
                break;
            }
            notice_changed = false;
        }
    });
    (commands, outgoing)
}

struct Hundred {
    focus: FocusHandle,
    sender: Sender<Command>,
    receiver: Receiver<Update>,
    events: Vec<Event>,
    pending: usize,
    notice: String,
    sync_label: String,
    selected: usize,
    number: String,
    replace_number: bool,
    history: bool,
    saving: bool,
    poll: Task<()>,
}

impl Hundred {
    fn new(sender: Sender<Command>, receiver: Receiver<Update>, cx: &mut Context<Self>) -> Self {
        let poll = cx.spawn(async move |this, cx| {
            let mut refresh = Instant::now();
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(150))
                    .await;
                if this
                    .update(cx, |this, cx| {
                        let mut changed = false;
                        while let Ok(update) = this.receiver.try_recv() {
                            this.events = update.events;
                            this.pending = update.pending;
                            if let Some(notice) = update.notice {
                                this.notice = notice;
                            }
                            this.sync_label = update.sync_label;
                            if update.saved {
                                this.replace_number = true;
                            }
                            this.saving = false;
                            changed = true;
                        }
                        if refresh.elapsed() >= Duration::from_secs(3) {
                            let _ = this.sender.send(Command::Refresh);
                            refresh = Instant::now();
                            changed = true; // Also handles local midnight without another set.
                        }
                        if changed {
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            focus: cx.focus_handle(),
            sender,
            receiver,
            events: vec![],
            pending: 0,
            notice: "Opening local history…".into(),
            sync_label: "Loading".into(),
            selected: 0,
            number: "20".into(),
            replace_number: true,
            history: false,
            saving: false,
            poll,
        }
    }
    fn save(&mut self, _: &Save, _: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        if let Ok(reps @ 1..=1000) = self.number.parse::<u16>() {
            if self
                .sender
                .send(Command::Add(Exercise::ALL[self.selected], reps))
                .is_ok()
            {
                self.saving = true;
                self.notice = "Saving here…".into();
            } else {
                self.notice = "Storage unavailable. Not saved.".into();
            }
        } else {
            self.notice = "Enter between 1 and 1000 reps.".into();
        }
        cx.notify();
    }
    fn next(&mut self, _: &Next, _: &mut Window, cx: &mut Context<Self>) {
        self.selected = (self.selected + 1) % 3;
        cx.notify();
    }
    fn previous(&mut self, _: &Previous, _: &mut Window, cx: &mut Context<Self>) {
        self.selected = (self.selected + 2) % 3;
        cx.notify();
    }
    fn pushups(&mut self, _: &PushUps, _: &mut Window, cx: &mut Context<Self>) {
        self.selected = 0;
        cx.notify();
    }
    fn crunches(&mut self, _: &Crunches, _: &mut Window, cx: &mut Context<Self>) {
        self.selected = 1;
        cx.notify();
    }
    fn squats(&mut self, _: &Squats, _: &mut Window, cx: &mut Context<Self>) {
        self.selected = 2;
        cx.notify();
    }
    fn increase(&mut self, _: &Increase, _: &mut Window, cx: &mut Context<Self>) {
        self.adjust(1);
        cx.notify();
    }
    fn decrease(&mut self, _: &Decrease, _: &mut Window, cx: &mut Context<Self>) {
        self.adjust(-1);
        cx.notify();
    }
    fn adjust(&mut self, delta: i32) {
        self.number = (self.number.parse::<i32>().unwrap_or(20) + delta)
            .clamp(1, 1000)
            .to_string();
        self.replace_number = true;
    }
    fn undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        let _ = self.sender.send(Command::Undo(None));
        cx.notify();
    }
    fn sync(&mut self, _: &Sync, _: &mut Window, _: &mut Context<Self>) {
        let _ = self.sender.send(Command::Sync);
    }
    fn history(&mut self, _: &History, _: &mut Window, cx: &mut Context<Self>) {
        self.history = !self.history;
        cx.notify();
    }
    fn back(&mut self, _: &Back, _: &mut Window, cx: &mut Context<Self>) {
        self.history = false;
        cx.notify();
    }
    fn select_number(&mut self, _: &SelectNumber, _: &mut Window, cx: &mut Context<Self>) {
        self.replace_number = true;
        cx.notify();
    }
    fn paste_number(&mut self, _: &PasteNumber, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let text = text.trim();
            if text.len() <= 4 && text.bytes().all(|byte| byte.is_ascii_digit()) {
                self.number = text.into();
                self.replace_number = true;
            } else {
                self.notice = "Paste a number between 1 and 1000.".into();
            }
        }
        cx.notify();
    }
    fn key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if key.modifiers.control || key.modifiers.alt || key.modifiers.platform {
            return;
        }
        if key.key == "backspace" {
            if self.replace_number {
                self.number.clear();
            } else {
                self.number.pop();
            }
            self.replace_number = false;
            cx.notify();
        } else if let Some(character) = key
            .key_char
            .as_deref()
            .filter(|text| text.len() == 1 && text.bytes().all(|byte| byte.is_ascii_digit()))
        {
            if self.replace_number {
                self.number.clear();
                self.replace_number = false;
            }
            if self.number.len() < 4 {
                self.number.push_str(character);
            }
            cx.notify();
        }
    }
    fn exercise_row(&self, index: usize, total: u64, cx: &mut Context<Self>) -> Stateful<Div> {
        let selected = self.selected == index;
        let exercise = Exercise::ALL[index];
        let filled = (total.min(GOAL) / 10) as usize;
        let trace = format!("{}{}", "#".repeat(filled), "·".repeat(10 - filled));
        div()
            .id(("exercise", index))
            .h(px(86.))
            .w_full()
            .px_4()
            .py_2()
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(if selected { COLOURS[index] } else { BG }))
            .bg(rgb(if selected { SURFACE } else { BG }))
            .cursor_pointer()
            .hover(|style| style.bg(rgb(SURFACE)))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.selected = index;
                window.focus(&this.focus);
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(18.))
                            .text_color(rgb(COLOURS[index]))
                            .child(exercise.label()),
                    )
                    .child(div().text_size(px(18.)).child(format!("{total} / 100"))),
            )
            .child(
                div()
                    .mt_2()
                    .flex()
                    .justify_between()
                    .text_color(rgb(MUTED))
                    .child(div().text_color(rgb(COLOURS[index])).child(trace))
                    .child(if total >= GOAL {
                        if total == GOAL {
                            "Done".into()
                        } else {
                            format!("{} extra", total - GOAL)
                        }
                    } else {
                        format!("{} left", remaining(total))
                    }),
            )
    }
    fn art(&self, totals: [u64; 3]) -> Div {
        div()
            .flex()
            .flex_col()
            .w(px(330.))
            .items_center()
            .justify_center()
            .gap_0()
            .children(pigment(totals, 48, 18).into_iter().map(|row| {
                div()
                    .flex()
                    .h(px(14.))
                    .children(row.into_iter().map(|(glyph, colour)| {
                        div()
                            .w(px(6.5))
                            .text_size(px(11.))
                            .line_height(px(14.))
                            .text_color(rgb(colour.map_or(BORDER, |index| COLOURS[index])))
                            .child(glyph.to_string())
                    }))
            }))
            .child(
                div()
                    .mt_3()
                    .text_size(px(12.))
                    .text_color(rgb(MUTED))
                    .child(if totals.iter().all(|total| *total >= GOAL) {
                        "Today's hundred, done."
                    } else {
                        "A little more colour, every set."
                    }),
            )
    }
    fn recent(&self, cx: &mut Context<Self>) -> Div {
        let sets = active_sets(&self.events);
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_color(rgb(MUTED)).mb_1().child("Recent sets"))
            .when(sets.is_empty(), |view| {
                view.child(div().py_2().child("Your first set starts here."))
            })
            .children(sets.into_iter().rev().take(4).map(|event| {
                let Change::Set { exercise, reps } = event.change else {
                    unreachable!()
                };
                let id = event.id.clone();
                let time = chrono::DateTime::parse_from_rfc3339(&event.at).unwrap();
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(44.))
                    .child(div().child(format!(
                        "{}  +{reps:<4} {}",
                        time.format("%b %d %H:%M"),
                        exercise.label()
                    )))
                    .child(
                        button(SharedString::from(format!("undo-{id}")), "Undo", false).on_click(
                            cx.listener(move |this, _, _, _| {
                                let _ = this.sender.send(Command::Undo(Some(id.clone())));
                            }),
                        ),
                    )
            }))
    }
}

fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    primary: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(44.))
        .px_3()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(if primary { TEXT } else { BORDER }))
        .bg(rgb(if primary { TEXT } else { SURFACE }))
        .text_color(rgb(if primary { BG } else { TEXT }))
        .cursor_pointer()
        .hover(|style| style.border_color(rgb(COLOURS[2])))
        .child(label.into())
}

impl Render for Hundred {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _keep_poll_alive = &self.poll;
        let today = Local::now().date_naive();
        let p = progress(&self.events, today);
        let totals = p.days.get(&today).copied().unwrap_or_default();
        div()
            .id("hundred")
            .key_context("Hundred")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key))
            .on_action(cx.listener(Self::next))
            .on_action(cx.listener(Self::previous))
            .on_action(cx.listener(Self::pushups))
            .on_action(cx.listener(Self::crunches))
            .on_action(cx.listener(Self::squats))
            .on_action(cx.listener(Self::save))
            .on_action(cx.listener(Self::increase))
            .on_action(cx.listener(Self::decrease))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::sync))
            .on_action(cx.listener(Self::history))
            .on_action(cx.listener(Self::back))
            .on_action(cx.listener(Self::select_number))
            .on_action(cx.listener(Self::paste_number))
            .size_full()
            .overflow_y_scroll()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .font_family("JetBrains Mono")
            .text_size(px(14.))
            .p_6()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .mb_6()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_4()
                            .child(div().text_size(px(24.)).child("Hundred"))
                            .child(
                                div()
                                    .text_color(rgb(MUTED))
                                    .child(today.format("%a %d %b").to_string()),
                            ),
                    )
                    .child(
                        button(
                            "history",
                            if self.history { "Today" } else { "History" },
                            false,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.history = !this.history;
                            cx.notify();
                        })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .mb_4()
                    .child(div().text_size(px(12.)).text_color(rgb(MUTED)).child(
                        if self.pending > 0 {
                            format!("{} · {} waiting", self.sync_label, self.pending)
                        } else {
                            self.sync_label.clone()
                        },
                    ))
                    .child(
                        button("sync", "Sync", false).on_click(cx.listener(|this, _, _, _| {
                            let _ = this.sender.send(Command::Sync);
                        })),
                    ),
            )
            .when(!self.history, |view| {
                view.child(
                    div()
                        .flex()
                        .gap_4()
                        .items_center()
                        .mb_5()
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(280.))
                                .flex()
                                .flex_col()
                                .gap_2()
                                .children(
                                    (0..3).map(|index| self.exercise_row(index, totals[index], cx)),
                                ),
                        )
                        .child(self.art(totals)),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .items_center()
                        .child(div().flex_1().child(format!(
                            "Log {}",
                            Exercise::ALL[self.selected].label().to_lowercase()
                        )))
                        .child(button("minus", "−", false).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.adjust(-1);
                                cx.notify();
                            },
                        )))
                        .child(
                            div()
                                .id("number")
                                .w(px(74.))
                                .h(px(44.))
                                .rounded(px(6.))
                                .border_1()
                                .border_color(rgb(COLOURS[self.selected]))
                                .bg(rgb(if self.replace_number { BORDER } else { SURFACE }))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(22.))
                                .cursor_text()
                                .child(if self.number.is_empty() {
                                    "_".into()
                                } else {
                                    self.number.clone()
                                })
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.replace_number = true;
                                    window.focus(&this.focus);
                                    cx.notify();
                                })),
                        )
                        .child(button("plus", "+", false).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.adjust(1);
                                cx.notify();
                            },
                        )))
                        .children([10, 20, 25].into_iter().map(|reps| {
                            button(("preset", reps as usize), reps.to_string(), false).on_click(
                                cx.listener(move |this, _, _, cx| {
                                    this.number = reps.to_string();
                                    this.replace_number = true;
                                    cx.notify();
                                }),
                            )
                        }))
                        .child(
                            button(
                                "save",
                                if self.saving { "Saving" } else { "Log set ↵" },
                                true,
                            )
                            .on_click(
                                cx.listener(|this, _, window, cx| this.save(&Save, window, cx)),
                            ),
                        ),
                )
                .child(
                    div()
                        .mt_3()
                        .mb_5()
                        .text_color(rgb(COLOURS[self.selected]))
                        .child(self.notice.clone()),
                )
                .child(
                    div()
                        .border_t_1()
                        .border_color(rgb(BORDER))
                        .pt_4()
                        .mb_4()
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .text_color(rgb(MUTED))
                                .mb_3()
                                .child("The last 28 days")
                                .child(format!(
                                    "{} day streak · best {}",
                                    p.current_streak, p.best_streak
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .children((0..28).map(|index| {
                                    let day = today - Days::days(27 - index);
                                    let day_totals = p.days.get(&day).copied().unwrap_or_default();
                                    let complete = day_totals.iter().all(|total| *total >= GOAL);
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .text_color(rgb(if complete { TEXT } else { MUTED }))
                                        .child(if complete {
                                            "#"
                                        } else if day_totals.iter().any(|total| *total > 0) {
                                            "+"
                                        } else {
                                            "·"
                                        })
                                        .child(div().text_size(px(10.)).child(
                                            if index == 0 || index == 27 {
                                                day.format("%d").to_string()
                                            } else {
                                                " ".into()
                                            },
                                        ))
                                })),
                        )
                        .child(
                            div()
                                .mt_2()
                                .text_size(px(12.))
                                .text_color(rgb(MUTED))
                                .child(format!(
                                    "# complete   + started   · rest     {} completed days",
                                    p.complete_days
                                )),
                        ),
                )
                .child(self.recent(cx))
            })
            .when(self.history, |view| {
                view.child(div().text_size(px(20.)).mb_4().child("Your days"))
                    .when(p.days.is_empty(), |view| {
                        view.child("Your first set starts here.")
                    })
                    .children(p.days.iter().rev().map(|(day, totals)| {
                        div()
                            .py_3()
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .child(div().mb_2().child(format!(
                                "{}{}",
                                day.format("%a %d %b %Y"),
                                if totals.iter().all(|total| *total >= GOAL) {
                                    "   Complete"
                                } else {
                                    ""
                                }
                            )))
                            .child(div().text_color(rgb(MUTED)).child(format!(
                                "Push-ups {}    Crunches {}    Squats {}",
                                totals[0], totals[1], totals[2]
                            )))
                    }))
                    .child(div().mt_5().child(self.recent(cx)))
            })
            .child(
                div()
                    .mt_4()
                    .pt_3()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .text_color(rgb(MUTED))
                    .text_size(px(11.))
                    .child("Tab exercise   Type reps   Enter log   Ctrl+Z undo   Ctrl+R sync")
                    .child(
                        div()
                            .mt_1()
                            .child("Ctrl+1/2/3 select   Ctrl+H history   Esc today   Ctrl+Q quit"),
                    ),
            )
    }
}

pub fn run(directory: PathBuf, server: String, token: String) -> Result<()> {
    let (sender, receiver) = controller(directory, server, token);
    let _ = sender.send(Command::Refresh);
    Application::new().run(move |cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("tab", Next, Some("Hundred")),
            KeyBinding::new("shift-tab", Previous, Some("Hundred")),
            KeyBinding::new("ctrl-1", PushUps, Some("Hundred")),
            KeyBinding::new("ctrl-2", Crunches, Some("Hundred")),
            KeyBinding::new("ctrl-3", Squats, Some("Hundred")),
            KeyBinding::new("enter", Save, Some("Hundred")),
            KeyBinding::new("up", Increase, Some("Hundred")),
            KeyBinding::new("down", Decrease, Some("Hundred")),
            KeyBinding::new("ctrl-z", Undo, Some("Hundred")),
            KeyBinding::new("ctrl-r", Sync, Some("Hundred")),
            KeyBinding::new("ctrl-h", History, Some("Hundred")),
            KeyBinding::new("escape", Back, Some("Hundred")),
            KeyBinding::new("ctrl-a", SelectNumber, Some("Hundred")),
            KeyBinding::new("ctrl-v", PasteNumber, Some("Hundred")),
            KeyBinding::new("ctrl-q", Quit, None),
        ]);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(980.), px(850.)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Hundred".into()),
                    ..Default::default()
                }),
                app_id: Some("io.mistystep.hundred".into()),
                window_min_size: Some(size(px(760.), px(640.))),
                ..Default::default()
            },
            move |window, cx| {
                let view = cx.new(|cx| Hundred::new(sender, receiver, cx));
                window.focus(&view.read(cx).focus);
                view
            },
        )
        .expect("Open the Hundred window");
        cx.activate(true);
    });
    Ok(())
}
