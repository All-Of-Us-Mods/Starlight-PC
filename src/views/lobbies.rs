//! Public lobby browser. Polls the optional `/x-api/games` endpoint (see
//! `hpllp013.yaml`) on every region the user has enabled in Among Us'
//! `regionInfo.json`, aggregates the active games, and lets the user copy a
//! join code or launch straight into a lobby — picking an existing profile or
//! a temporary one, with the lobby's required mods installed automatically.

use std::collections::HashMap;
use std::time::Duration;

use gpui::{prelude::FluentBuilder as _, *};
use log::warn;

use crate::backend::api::{self, Game};
use crate::backend::error::{AppError, AppResult};
use crate::backend::events::{self, BackendEvent};
use crate::backend::services::mod_install_service::{self, InstallModInput};
use crate::backend::services::profile_service::{self, ProfileEntry};
use crate::backend::services::{launch_service, region_service};
use crate::backend::state::game_runtime::{self, GameStatePayload};
use crate::theme::{Theme, ThemeExt};
use crate::views::{modal_overlay, page_root, section_label};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::skeleton::Skeleton;
use gpui_component::{Disableable, Icon, IconName, Sizable};

/// How often the lobby list re-polls every enabled region.
const REFRESH_INTERVAL_SECS: u64 = 12;

pub struct LobbiesView {
    state: LoadState,
    /// Profiles offered in the launch dialog; refreshed alongside the lobbies.
    profiles: Vec<ProfileEntry>,
    launch_dialog: Option<LaunchDialog>,
    notice: Option<String>,
    error: Option<String>,
    /// True while a poll is in flight (drives the header spinner without
    /// flashing the list back to skeletons).
    refreshing: bool,
    /// Temporary profiles created for "Temporary profile" launches, pending
    /// deletion once their game exits. Value is whether we've yet observed the
    /// profile with a running instance (so we don't delete it before its game
    /// even started).
    temp_cleanup: HashMap<String, bool>,
    /// The auto-refresh loop; dropped (and thus cancelled) with the view.
    _refresh: Task<()>,
}

enum LoadState {
    Loading,
    Loaded(Vec<LobbyRow>),
    /// `regionInfo.json` could not be read (e.g. not on Windows).
    Unsupported,
}

#[derive(Clone)]
struct LobbyRow {
    game: Game,
    /// Display name for the lobby's region (from the server's own region list,
    /// falling back to the enabled region's name).
    region_label: String,
    /// Server origin of the enabled region this lobby was found on, used to
    /// point Among Us at the right region before launching.
    server_url: String,
}

struct LaunchDialog {
    lobby: LobbyRow,
    target: LaunchTarget,
    busy: bool,
    error: Option<String>,
}

#[derive(Clone, PartialEq)]
enum LaunchTarget {
    Existing(String),
    Temporary,
}

impl LobbiesView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Subscribed up front (before any launch can happen) so the event that
        // marks a freshly-launched temp profile as running is never missed.
        let mut rx = events::subscribe();
        cx.spawn(async move |this, cx| {
            while let Ok(event) = rx.recv().await {
                let BackendEvent::GameStateChanged(payload) = event else {
                    continue;
                };
                if this
                    .update(cx, |this, _cx| this.reap_finished_temp_profiles(&payload))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

        let refresh = cx.spawn(async move |this, cx| {
            loop {
                // Bail out if the view is gone (also covered by Task drop).
                if this
                    .update(cx, |this, cx| {
                        this.refreshing = true;
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }

                let servers = cx
                    .background_executor()
                    .spawn(async { region_service::lobby_servers() })
                    .await;

                match servers {
                    Err(_) => {
                        let _ = this.update(cx, |this, cx| {
                            this.state = LoadState::Unsupported;
                            this.refreshing = false;
                            cx.notify();
                        });
                    }
                    Ok(servers) => {
                        // Poll every enabled region concurrently; a server that
                        // errors or doesn't implement the endpoint is skipped.
                        let tasks: Vec<_> = servers
                            .into_iter()
                            .map(|srv| {
                                let url = srv.url.clone();
                                let task = cx
                                    .background_executor()
                                    .spawn(async move { api::fetch_lobbies(&url) });
                                (srv, task)
                            })
                            .collect();

                        let mut rows: Vec<LobbyRow> = Vec::new();
                        for (srv, task) in tasks {
                            let Ok(result) = task.await else {
                                continue;
                            };
                            for game in result.games {
                                // Skip finished games — they can't be joined.
                                if game.status.as_deref() == Some("Ended") {
                                    continue;
                                }
                                let region_label = game
                                    .region_id
                                    .as_ref()
                                    .and_then(|id| {
                                        result
                                            .regions
                                            .iter()
                                            .find(|r| r.id.as_deref() == Some(id.as_str()))
                                    })
                                    .and_then(|r| r.name.clone())
                                    .unwrap_or_else(|| srv.region_name.clone());
                                rows.push(LobbyRow {
                                    game,
                                    region_label,
                                    server_url: srv.url.clone(),
                                });
                            }
                        }
                        // Open lobbies first, then fuller rooms first.
                        rows.sort_by(|a, b| {
                            let open = |g: &Game| u8::from(g.status.as_deref() == Some("Lobby"));
                            open(&b.game)
                                .cmp(&open(&a.game))
                                .then(b.game.player_count.cmp(&a.game.player_count))
                        });

                        let _ = this.update(cx, |this, cx| {
                            this.state = LoadState::Loaded(rows);
                            this.refreshing = false;
                            cx.notify();
                        });
                    }
                }

                // Keep the launch dialog's profile list current.
                let profiles = cx
                    .background_executor()
                    .spawn(async { profile_service::get_profiles().unwrap_or_default() })
                    .await;
                if this
                    .update(cx, |this, cx| {
                        this.profiles = profiles;
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }

                cx.background_executor()
                    .timer(Duration::from_secs(REFRESH_INTERVAL_SECS))
                    .await;
            }
        });

        Self {
            state: LoadState::Loading,
            profiles: Vec::new(),
            launch_dialog: None,
            notice: None,
            error: None,
            refreshing: false,
            temp_cleanup: HashMap::new(),
            _refresh: refresh,
        }
    }

    fn copy_code(&self, code: String, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(code));
    }

    /// Start watching a temporary profile so it's deleted once its game exits.
    /// Seeds the "seen running" flag from the current snapshot in case the
    /// launch's `GameStateChanged` event already fired before this call (the
    /// background launch thread registers the process, and may finish, before
    /// this runs on the main thread).
    fn track_temp_profile(&mut self, profile_id: String) {
        let already_running = game_runtime::current_state()
            .profile_instance_counts
            .contains_key(&profile_id);
        self.temp_cleanup.insert(profile_id, already_running);
    }

    /// Delete any temporary profile whose tracked instance count has dropped
    /// back to zero after having been seen running at least once.
    fn reap_finished_temp_profiles(&mut self, payload: &GameStatePayload) {
        let mut finished = Vec::new();
        self.temp_cleanup.retain(|id, seen_running| {
            if payload.profile_instance_counts.contains_key(id) {
                *seen_running = true;
                true
            } else if *seen_running {
                finished.push(id.clone());
                false
            } else {
                true
            }
        });
        for id in finished {
            std::thread::spawn(move || {
                if let Err(e) = profile_service::delete_profile(&id) {
                    warn!("failed to delete temporary lobby profile {id}: {e}");
                }
            });
        }
    }

    fn open_launch_dialog(&mut self, lobby: LobbyRow, cx: &mut Context<Self>) {
        // Default to the first existing profile, or a temporary one if none.
        let target = self
            .profiles
            .first()
            .map(|p| LaunchTarget::Existing(p.id.clone()))
            .unwrap_or(LaunchTarget::Temporary);
        self.launch_dialog = Some(LaunchDialog {
            lobby,
            target,
            busy: false,
            error: None,
        });
        self.notice = None;
        self.error = None;
        cx.notify();
    }

    fn submit_launch(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.launch_dialog.as_mut() else {
            return;
        };
        if dialog.busy {
            return;
        }
        dialog.busy = true;
        dialog.error = None;
        let lobby = dialog.lobby.clone();
        let target = dialog.target.clone();
        cx.notify();

        let code = lobby.game.code.clone().unwrap_or_default();
        let server_url = lobby.server_url.clone();
        let required: Vec<InstallModInput> = lobby
            .game
            .mods
            .iter()
            .filter_map(|m| {
                Some(InstallModInput {
                    mod_id: m.id.clone()?,
                    version: m.version.clone()?,
                })
            })
            .collect();

        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_executor()
                .spawn(async move { launch_into_lobby(target, required, &server_url) })
                .await;
            let _ = this.update(cx, |this, cx| {
                match outcome {
                    Ok(outcome) => {
                        this.launch_dialog = None;
                        let mut message = String::new();
                        if !code.is_empty() {
                            this.copy_code(code.clone(), cx);
                            message = format!("Code {code} copied to clipboard. ");
                        }
                        message.push_str(&outcome.summary);
                        this.notice = Some(message);
                        if let Some(temp_id) = outcome.temp_profile_id {
                            this.track_temp_profile(temp_id);
                        }
                    }
                    Err(e) => {
                        warn!("launch into lobby failed: {e}");
                        if let Some(d) = this.launch_dialog.as_mut() {
                            d.busy = false;
                            d.error = Some(e.to_string());
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn render_lobbies(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        match &self.state {
            LoadState::Loading => div()
                .flex()
                .flex_col()
                .gap_2()
                .children((0..4).map(|_| {
                    Skeleton::new()
                        .w_full()
                        .h(px(64.0))
                        .rounded_lg()
                        .into_any_element()
                }))
                .into_any_element(),
            LoadState::Unsupported => div()
                .text_sm()
                .text_color(theme.text_muted)
                .child("Could not read your Among Us regions (Windows only). Add a region on the Servers tab to browse its lobbies.")
                .into_any_element(),
            LoadState::Loaded(rows) if rows.is_empty() => div()
                .text_sm()
                .text_color(theme.text_muted)
                .child("No open lobbies found. Only servers that publish a lobby list appear here.")
                .into_any_element(),
            LoadState::Loaded(rows) => div()
                .flex()
                .flex_col()
                .gap_2()
                .children(rows.iter().enumerate().map(|(ix, row)| self.render_row(ix, row, theme, cx)))
                .into_any_element(),
        }
    }

    fn render_row(
        &self,
        ix: usize,
        row: &LobbyRow,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let game = &row.game;
        let code = game.code.clone().unwrap_or_default();
        let host = game
            .host_name
            .clone()
            .filter(|h| !h.is_empty())
            .unwrap_or_else(|| "Unknown host".to_string());
        let players = format!(
            "{}/{}",
            game.player_count.unwrap_or(0),
            game.max_players.unwrap_or(0)
        );
        let mut meta = vec![
            players,
            map_name(game.map_id).to_string(),
            row.region_label.clone(),
        ];
        if !game.mods.is_empty() {
            meta.push(format!("{} mod(s)", game.mods.len()));
        }
        let meta_line = meta.join(" · ");

        let is_open = game.status.as_deref() == Some("Lobby");
        let status_text = game.status.clone().unwrap_or_else(|| "Unknown".to_string());
        let status_color = if is_open { theme.success } else { theme.warning };

        let copy_code = code.clone();
        let row_for_launch = row.clone();

        div()
            .flex()
            .items_center()
            .gap_3()
            .px_3()
            .py_2()
            .rounded_lg()
            .bg(theme.sidebar_background)
            .border_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .font_family("ui-monospace, monospace")
                                    .font_weight(FontWeight::BOLD)
                                    .child(if code.is_empty() {
                                        "------".to_string()
                                    } else {
                                        code.clone()
                                    }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(status_color)
                                    .child(status_text),
                            )
                            .child(div().min_w_0().truncate().text_color(theme.text_muted).child(host)),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child(meta_line),
                    ),
            )
            .child(
                Button::new(SharedString::from(format!("copy-code-{ix}")))
                    .ghost()
                    .xsmall()
                    .icon(Icon::new(IconName::Copy))
                    .label("Copy")
                    .disabled(code.is_empty())
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.copy_code(copy_code.clone(), cx);
                        this.notice = Some("Code copied to clipboard.".into());
                        cx.notify();
                    })),
            )
            .child(
                Button::new(SharedString::from(format!("launch-lobby-{ix}")))
                    .primary()
                    .xsmall()
                    .icon(Icon::new(IconName::Play))
                    .label("Launch")
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.open_launch_dialog(row_for_launch.clone(), cx)
                    })),
            )
            .into_any_element()
    }

    fn render_launch_dialog(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        let dialog = self.launch_dialog.as_ref()?;
        let game = &dialog.lobby.game;
        let code = game.code.clone().unwrap_or_else(|| "------".to_string());
        let mod_count = game.mods.len();

        let mut option_rows: Vec<AnyElement> = self
            .profiles
            .iter()
            .map(|p| {
                let subtitle = if p.bepinex_installed.unwrap_or(false) {
                    "Modded profile"
                } else {
                    "BepInEx will be installed"
                };
                self.render_target_option(
                    LaunchTarget::Existing(p.id.clone()),
                    &p.name,
                    subtitle,
                    &dialog.target,
                    theme,
                    cx,
                )
            })
            .collect();
        option_rows.push(self.render_target_option(
            LaunchTarget::Temporary,
            "Temporary profile",
            "Fresh profile, deleted automatically once the game closes",
            &dialog.target,
            theme,
            cx,
        ));

        let mods_note = if mod_count == 0 {
            "No mods required.".to_string()
        } else {
            format!("{mod_count} required mod(s) will be installed if missing.")
        };

        let mut items: Vec<AnyElement> = vec![
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .child(format!("Launch into lobby {code}"))
                .into_any_element(),
            div()
                .text_xs()
                .text_color(theme.text_muted)
                .child(format!("Region: {}", dialog.lobby.region_label))
                .into_any_element(),
            section_label("Profile", theme).into_any_element(),
            div()
                .id("launch-profile-list")
                .flex()
                .flex_col()
                .gap_2()
                .max_h(px(220.0))
                .overflow_y_scroll()
                .children(option_rows)
                .into_any_element(),
            div()
                .text_xs()
                .text_color(theme.text_muted)
                .child(mods_note)
                .into_any_element(),
        ];
        if let Some(err) = &dialog.error {
            items.push(
                div()
                    .text_xs()
                    .text_color(theme.danger)
                    .child(err.clone())
                    .into_any_element(),
            );
        }
        items.push(
            div()
                .flex()
                .gap_2()
                .justify_end()
                .child(
                    Button::new("launch-cancel")
                        .label("Cancel")
                        .disabled(dialog.busy)
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.launch_dialog = None;
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("launch-confirm")
                        .primary()
                        .icon(Icon::new(IconName::Play))
                        .label(if dialog.busy {
                            "Launching…"
                        } else {
                            "Launch"
                        })
                        .disabled(dialog.busy)
                        .on_click(cx.listener(|this, _, _window, cx| this.submit_launch(cx))),
                )
                .into_any_element(),
        );

        Some(modal_overlay(theme, px(460.0), items).into_any_element())
    }

    fn render_target_option(
        &self,
        target: LaunchTarget,
        title: &str,
        subtitle: &str,
        selected: &LaunchTarget,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_selected = &target == selected;
        let id = match &target {
            LaunchTarget::Existing(pid) => format!("target-{pid}"),
            LaunchTarget::Temporary => "target-temporary".to_string(),
        };
        let border = if is_selected {
            theme.primary
        } else {
            theme.border
        };
        let pick = target.clone();
        div()
            .id(SharedString::from(id))
            .flex()
            .items_center()
            .gap_3()
            .px_3()
            .py_2()
            .rounded_lg()
            .bg(theme.background)
            .border_1()
            .border_color(border)
            .cursor_pointer()
            .hover(|s| s.bg(theme.hover))
            .on_click(cx.listener(move |this, _, _window, cx| {
                if let Some(d) = this.launch_dialog.as_mut() {
                    d.target = pick.clone();
                }
                cx.notify();
            }))
            .child(
                // Radio indicator.
                div()
                    .size(px(14.0))
                    .rounded_full()
                    .border_1()
                    .border_color(if is_selected { theme.primary } else { theme.text_muted })
                    .when(is_selected, |s| s.bg(theme.primary)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(div().truncate().font_weight(FontWeight::MEDIUM).child(title.to_string()))
                    .child(
                        div()
                            .truncate()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child(subtitle.to_string()),
                    ),
            )
            .into_any_element()
    }
}

impl Render for LobbiesView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        page_root("lobbies-page", &theme)
            .relative()
            .overflow_y_scroll()
            .gap_6()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_2xl().font_weight(FontWeight::BOLD).child("Lobbies"))
                            .when(self.refreshing, |s| {
                                s.child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.text_muted)
                                        .child("Refreshing…"),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text_muted)
                            .child("Open games on your enabled regions that publish a public lobby list. Launch in, then paste the copied code in-game."),
                    ),
            )
            .children(self.error.clone().map(|message| {
                div()
                    .rounded_md()
                    .bg(rgb(0x7f1d1d))
                    .p_3()
                    .text_color(theme.text)
                    .child(message)
            }))
            .children(
                self.notice
                    .clone()
                    .map(|message| div().text_sm().text_color(theme.success).child(message)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(section_label("Active lobbies", &theme))
                    .child(self.render_lobbies(&theme, cx)),
            )
            .children(self.render_launch_dialog(&theme, cx))
    }
}

/// Map id → Among Us map name (see `MapNames.cs`).
fn map_name(map_id: Option<u32>) -> &'static str {
    match map_id {
        Some(0) => "The Skeld",
        Some(1) => "MIRA HQ",
        Some(2) => "Polus",
        Some(3) => "Dleks",
        Some(4) => "The Airship",
        Some(5) => "The Fungle",
        _ => "Unknown map",
    }
}

/// Outcome of a successful `launch_into_lobby` call.
struct LaunchOutcome {
    /// Short summary of what happened (the caller adds the copied-code note).
    summary: String,
    /// Set when the launch used a temporary profile, so the caller can watch
    /// for its game exiting and delete it.
    temp_profile_id: Option<String>,
}

/// Resolve the target profile, install the lobby's required mods (best-effort),
/// point Among Us at the lobby's region, and launch. Blocking; run on the
/// background executor.
fn launch_into_lobby(
    target: LaunchTarget,
    required: Vec<InstallModInput>,
    server_url: &str,
) -> AppResult<LaunchOutcome> {
    let is_temp = matches!(target, LaunchTarget::Temporary);
    let profile = match target {
        LaunchTarget::Existing(id) => profile_service::get_profile_by_id(&id)?
            .ok_or_else(|| AppError::validation("The selected profile no longer exists"))?,
        LaunchTarget::Temporary => create_temp_profile()?,
    };
    let temp_profile_id = is_temp.then(|| profile.id.clone());

    if profile.bepinex_installed != Some(true) {
        profile_service::install_bepinex_for_profile(&profile.id)?;
    }

    let mut skipped = 0usize;
    if !required.is_empty() {
        let (installable, unresolved) = mod_install_service::plan_lobby_mods(&required);
        skipped = unresolved.len();
        // Skip mods already present at the exact version the lobby wants.
        let missing: Vec<InstallModInput> = installable
            .into_iter()
            .filter(|m| {
                !profile
                    .mods
                    .iter()
                    .any(|p| p.mod_id == m.mod_id && p.version == m.version)
            })
            .collect();
        if !missing.is_empty() {
            mod_install_service::install_mods_for_profile(&profile.id, &missing)?;
        }
    }

    let region_set = region_service::select_region_by_server_url(server_url).unwrap_or(false);

    // Reload so the launch sees the freshly installed BepInEx / mods.
    let profile = profile_service::get_profile_by_id(&profile.id)?
        .ok_or_else(|| AppError::validation("Profile disappeared before launch"))?;
    launch_service::launch_modded_for_profile(profile)?;

    let mut summary = if region_set {
        "Launched with the region set to this lobby.".to_string()
    } else {
        "Launched.".to_string()
    };
    if skipped > 0 {
        summary.push_str(&format!(
            " {skipped} required mod(s) weren't in the catalog and were skipped."
        ));
    }
    Ok(LaunchOutcome {
        summary,
        temp_profile_id,
    })
}

/// Create a fresh throwaway profile for a one-off lobby launch, uniquely named
/// so repeated temporary launches don't collide. The caller (`LobbiesView`)
/// deletes it once the launched game exits.
fn create_temp_profile() -> AppResult<ProfileEntry> {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    profile_service::create_profile(&format!("Temporary Lobby {millis}"))
}
