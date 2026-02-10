use crate::config::{LongcatConfig, Verbosity};
use crate::niri::registry::NiriRegistry;
use dashmap::DashMap;
use niri_ipc::{Action, Request, Response, SizeChange, Window};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;

#[derive(Debug, Clone, Default)]
struct ColumnState {
    window_heights: HashMap<u64, f64>,
    last_height_change: Option<Instant>,
}

#[derive(Debug, Clone)]
struct ActiveColumn {
    workspace_id: u64,
    column_idx: usize,
}

pub struct Longcat {
    niri_socket: Arc<RwLock<Option<niri_ipc::socket::Socket>>>,
    config: Arc<RwLock<LongcatConfig>>,
    verbosity: Verbosity,
    column_states: Arc<DashMap<(u64, usize), ColumnState>>,
    active_column: Arc<RwLock<Option<ActiveColumn>>>,
}

impl Longcat {
    pub fn new(config: LongcatConfig, verbosity: Verbosity) -> Self {
        Self {
            niri_socket: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(config)),
            verbosity,
            column_states: Arc::new(DashMap::new()),
            active_column: Arc::new(RwLock::new(None)),
        }
    }

    async fn get_socket(&self) -> Result<niri_ipc::socket::Socket, Box<dyn std::error::Error>> {
        let mut socket_guard = self.niri_socket.write().await;
        if let Some(socket) = socket_guard.take() {
            return Ok(socket);
        }
        Ok(niri_ipc::socket::Socket::connect()?)
    }

    async fn return_socket(&self, socket: niri_ipc::socket::Socket) {
        let mut socket_guard = self.niri_socket.write().await;
        *socket_guard = Some(socket);
    }

    async fn send_action(&self, action: Action) -> Result<(), Box<dyn std::error::Error>> {
        let mut socket = self.get_socket().await?;
        let reply = socket.send(Request::Action(action));
        self.return_socket(socket).await;
        match reply {
            Ok(Ok(Response::Handled)) => Ok(()),
            Ok(Ok(other)) => Err(format!("Unexpected response: {:?}", other).into()),
            Ok(Err(e)) => Err(format!("Action error: {}", e).into()),
            Err(e) => Err(format!("Action failed: {:?}", e).into()),
        }
    }

    async fn get_windows(&self) -> Result<Vec<niri_ipc::Window>, Box<dyn std::error::Error>> {
        let mut socket = self.get_socket().await?;
        let reply = socket.send(Request::Windows);
        self.return_socket(socket).await;
        match reply {
            Ok(Ok(Response::Windows(windows))) => Ok(windows),
            Ok(Ok(other)) => Err(format!("Unexpected response: {:?}", other).into()),
            Ok(Err(e)) => Err(format!("Request error: {}", e).into()),
            Err(e) => Err(format!("Failed to get windows: {:?}", e).into()),
        }
    }

    fn get_column_key(window: &niri_ipc::Window) -> Option<(u64, usize)> {
        let workspace_id = window.workspace_id?;
        let (column_idx, _) = window.layout.pos_in_scrolling_layout?;
        Some((workspace_id, column_idx))
    }

    async fn get_column_windows(
        &self,
        workspace_id: u64,
        column_idx: usize,
    ) -> Result<Vec<niri_ipc::Window>, Box<dyn std::error::Error>> {
        let all_windows = self.get_windows().await?;
        let column_windows: Vec<_> = all_windows
            .into_iter()
            .filter(|w| {
                w.workspace_id == Some(workspace_id)
                    && w.layout
                        .pos_in_scrolling_layout
                        .map(|(col, _)| col == column_idx)
                        .unwrap_or(false)
            })
            .collect();
        Ok(column_windows)
    }

    async fn store_original_heights(
        &self,
        workspace_id: u64,
        column_idx: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let windows = self.get_column_windows(workspace_id, column_idx).await?;
        let mut state = ColumnState::default();

        for window in &windows {
            state
                .window_heights
                .insert(window.id, window.layout.window_size.1 as f64);
        }

        if self.verbosity.log_window_events() {
            eprintln!(
                "Stored original heights for {} windows in column {}",
                state.window_heights.len(),
                column_idx
            );
        }

        self.column_states
            .insert((workspace_id, column_idx), state);
        Ok(())
    }

    async fn check_manual_resize_and_update_proportions(
        &self,
        workspace_id: u64,
        column_idx: usize,
        active_window_id: u64,
    ) -> Result<(bool, Option<(f64, f64)>), Box<dyn std::error::Error>> {
        let windows = self.get_column_windows(workspace_id, column_idx).await?;
        let state_key = (workspace_id, column_idx);
        let config = self.config.read().await;

        if let Some(mut state) = self.column_states.get_mut(&state_key) {
            let now = Instant::now();

            for window in &windows {
                if let Some(&stored_height) = state.window_heights.get(&window.id) {
                    let current_height = window.layout.window_size.1 as f64;
                    let height_diff = (current_height - stored_height).abs();

                    if height_diff > 1.0 {
                        if let Some(last_change) = state.last_height_change {
                            let elapsed = now.duration_since(last_change).as_millis() as u64;

                            if elapsed >= config.resize_debounce_ms {
                                if self.verbosity.log_window_events() {
                                    eprintln!(
                                        "Manual resize detected for window {} after {}ms debounce",
                                        window.id, elapsed
                                    );
                                }

                                let total_height: f64 = windows
                                    .iter()
                                    .map(|w| w.layout.window_size.1 as f64)
                                    .sum();

                                let resized_window_id = window.id;
                                let resized_is_active = resized_window_id == active_window_id;

                                let (new_active_prop, new_other_prop) = if resized_is_active {
                                    let active_height = current_height;
                                    let other_height = total_height - active_height;
                                    (active_height / total_height, other_height / total_height)
                                } else {
                                    let other_height = current_height;
                                    let active_height = total_height - other_height;
                                    (active_height / total_height, other_height / total_height)
                                };

                                if self.verbosity.log_window_events() {
                                    eprintln!(
                                        "Updating proportions: active {:.3} -> {:.3}, other {:.3} -> {:.3}",
                                        config.active_height_proportion,
                                        new_active_prop,
                                        config.other_height_proportion,
                                        new_other_prop
                                    );
                                }

                                state.last_height_change = Some(now);
                                return Ok((true, Some((new_active_prop, new_other_prop))));
                            }
                        } else {
                            state.last_height_change = Some(now);
                        }
                    }
                }
            }
        }

        Ok((false, None))
    }

    async fn clear_column_state(&self, workspace_id: u64, column_idx: usize) {
        self.column_states.remove(&(workspace_id, column_idx));
        if self.verbosity.log_window_events() {
            eprintln!("Cleared column state for {}", column_idx);
        }
    }

    pub async fn handle_focus_gained(&self, window_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        let windows = self.get_windows().await?;
        let focused_window = windows
            .iter()
            .find(|w| w.id == window_id)
            .ok_or("Focused window not found")?;

        if let Some((workspace_id, column_idx)) = Self::get_column_key(focused_window) {
            let state_key = (workspace_id, column_idx);
            let mut active_col = self.active_column.write().await;

            let _is_new_column = active_col
                .as_ref()
                .map(|ac| ac.workspace_id != workspace_id || ac.column_idx != column_idx)
                .unwrap_or(true);

            let (manual_resize, new_proportions) = self
                .check_manual_resize_and_update_proportions(workspace_id, column_idx, window_id)
                .await?;

            if manual_resize {
                if let Some((new_active, new_other)) = new_proportions {
                    let mut config = self.config.write().await;
                    config.active_height_proportion = new_active;
                    config.other_height_proportion = new_other;
                }
                self.clear_column_state(workspace_id, column_idx).await;
                self.store_original_heights(workspace_id, column_idx).await?;
            } else if !self.column_states.contains_key(&state_key) {
                self.store_original_heights(workspace_id, column_idx).await?;
            }

            *active_col = Some(ActiveColumn {
                workspace_id,
                column_idx,
            });

            self.apply_longcat_heights(workspace_id, column_idx, window_id)
                .await?;
        }

        Ok(())
    }

    pub async fn handle_focus_lost(&self, window_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        let windows = self.get_windows().await?;
        let blurred_window = windows
            .iter()
            .find(|w| w.id == window_id)
            .ok_or("Blurred window not found")?;

        if let Some((workspace_id, column_idx)) = Self::get_column_key(blurred_window) {
            let mut active_col = self.active_column.write().await;

            let is_staying_in_column = windows.iter().any(|w| {
                w.is_focused
                    && w.layout
                        .pos_in_scrolling_layout
                        .map(|(col, _)| col == column_idx)
                        .unwrap_or(false)
            });

            if let Some(ref ac) = *active_col {
                if ac.workspace_id == workspace_id && ac.column_idx == column_idx {
                    if !is_staying_in_column {
                        self.restore_original_heights(workspace_id, column_idx)
                            .await?;
                        *active_col = None;
                    } else if self.verbosity.log_window_events() {
                        eprintln!(
                            "Focus staying in column {}:{}, skipping restore",
                            workspace_id, column_idx
                        );
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn _handle_window_layout_changed(
        &self,
        window_id: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let windows = self.get_windows().await?;
        let window = windows
            .iter()
            .find(|w| w.id == window_id)
            .ok_or("Window not found")?;

        if let Some((workspace_id, column_idx)) = Self::get_column_key(window) {
            if self.column_states.contains_key(&(workspace_id, column_idx)) {
                let (manual_resize, _) = self
                    .check_manual_resize_and_update_proportions(workspace_id, column_idx, 0)
                    .await?;
                if manual_resize {
                    self.clear_column_state(workspace_id, column_idx).await;
                }
            }
        }

        Ok(())
    }

    async fn apply_longcat_heights(
        &self,
        workspace_id: u64,
        column_idx: usize,
        active_window_id: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let windows = self.get_column_windows(workspace_id, column_idx).await?;

        if windows.len() <= 1 {
            if self.verbosity.log_window_events() {
                eprintln!("Column has {} windows, skipping longcat adjustment", windows.len());
            }
            return Ok(());
        }

        let state_key = (workspace_id, column_idx);
        if !self.column_states.contains_key(&state_key) {
            self.store_original_heights(workspace_id, column_idx).await?;
        }

        let state = self.column_states.get(&state_key).unwrap();
        let config = self.config.read().await;

        let total_original_height: f64 = state.window_heights.values().sum();
        let total_active_height = total_original_height * config.active_height_proportion;
        let total_other_height = total_original_height * config.other_height_proportion;

        let active_original_sum: f64 = windows
            .iter()
            .filter(|w| w.id == active_window_id)
            .filter_map(|w| state.window_heights.get(&w.id).copied())
            .sum();

        let other_original_sum: f64 = windows
            .iter()
            .filter(|w| w.id != active_window_id)
            .filter_map(|w| state.window_heights.get(&w.id).copied())
            .sum();

        let mut window_changes = Vec::new();
        let mut has_changes = false;

        for window in &windows {
            let is_active = window.id == active_window_id;
            let original_height = state.window_heights.get(&window.id).copied().unwrap_or(0.0);
            let current_height = window.layout.window_size.1 as f64;

            let target_height = if is_active {
                if active_original_sum > 0.0 {
                    (original_height / active_original_sum) * total_active_height
                } else {
                    original_height
                }
            } else {
                if other_original_sum > 0.0 {
                    (original_height / other_original_sum) * total_other_height
                } else {
                    original_height
                }
            };

            let change = SizeChange::SetFixed(target_height as i32);

            if (current_height - target_height).abs() > 0.1 {
                has_changes = true;
                window_changes.push((window, original_height, target_height));

                if self.verbosity.log_window_events() {
                    eprintln!(
                        "Setting window {} height: {} -> {} (active: {})",
                        window.id, current_height, target_height, is_active
                    );
                }

                self.send_action(Action::SetWindowHeight {
                    id: Some(window.id),
                    change,
                })
                .await?;
            }
        }

        if has_changes && self.verbosity.log_structured() {
            log_structured_change(
                workspace_id,
                column_idx,
                active_window_id,
                config.active_height_proportion,
                &window_changes,
            );
        }

        Ok(())
    }

    async fn restore_original_heights(
        &self,
        workspace_id: u64,
        column_idx: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state_key = (workspace_id, column_idx);
        if let Some(state) = self.column_states.get(&state_key) {
            let mut window_changes = Vec::new();

            let windows = self.get_column_windows(workspace_id, column_idx).await?;

            for window in &windows {
                if let Some(&original_height) = state.window_heights.get(&window.id) {
                    let current_height = window.layout.window_size.1 as f64;

                    if (current_height - original_height).abs() > 0.1 {
                        window_changes.push((window, original_height, original_height));

                        let change = SizeChange::SetFixed(original_height as i32);

                        if self.verbosity.log_window_events() {
                            eprintln!("Restoring window {} height to {}", window.id, original_height);
                        }

                        self.send_action(Action::SetWindowHeight {
                            id: Some(window.id),
                            change,
                        })
                        .await?;
                    }
                }
            }

            if !window_changes.is_empty() && self.verbosity.log_structured() {
                log_structured_change(
                    workspace_id,
                    column_idx,
                    0,
                    1.0,
                    &window_changes,
                );
            }
        }
        Ok(())
    }
}

fn log_structured_change(
    workspace: u64,
    column: usize,
    active: u64,
    current_proportion: f64,
    window_changes: &[(&Window, f64, f64)],
) {
    let json = json!({
        "workspace": workspace,
        "column": column,
        "active": active,
        "current_proportion": current_proportion,
        "windows": window_changes.iter().map(|(window, original, target)| {
            let mut obj = json!({
                "id": window.id,
                "original": original,
                "current": window.layout.window_size.1,
                "target": target,
            });
            if let Some(ref title) = window.title {
                obj["title"] = json!(title);
            }
            obj
        }).collect::<Vec<_>>()
    });
    eprintln!("{}", json);
}

pub async fn run_longcat(
    verbosity: Verbosity,
    config: LongcatConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    if verbosity.log_window_events() {
        eprintln!("Starting longcat");
        eprintln!("Active height proportion: {}", config.active_height_proportion);
        eprintln!("Other height proportion: {}", config.other_height_proportion);
        eprintln!("Resize debounce: {}ms", config.resize_debounce_ms);
    }

    let niri_registry = NiriRegistry::new_with_verbosity(verbosity).await?;
    let longcat = Longcat::new(config, verbosity);
    let mut events = niri_registry.into_events();

    while let Some(event) = events.next().await {
        match event {
            crate::niri::types::NiriEvent::Focus { window_id, .. } => {
                if let Err(e) = longcat.handle_focus_gained(window_id).await {
                    eprintln!("Error handling focus gained: {}", e);
                }
            }
            crate::niri::types::NiriEvent::Blur { window_id, .. } => {
                if let Err(e) = longcat.handle_focus_lost(window_id).await {
                    eprintln!("Error handling focus lost: {}", e);
                }
            }
            _ => {}
        }
    }

    Ok(())
}
