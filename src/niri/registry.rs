use crate::config::Verbosity;
use niri_ipc::socket::Socket;
use niri_ipc::{Event, Request, Response};
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt, wrappers::UnboundedReceiverStream};

use crate::niri::types::{NiriEvent, WindowInfo};

pub struct NiriRegistry {
    socket: Option<Socket>,
    event_tx: mpsc::UnboundedSender<NiriEvent>,
    event_rx: Option<mpsc::UnboundedReceiver<NiriEvent>>,
    verbosity: Verbosity,
}

impl NiriRegistry {
    pub async fn new_with_verbosity(
        verbosity: Verbosity,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut socket = Socket::connect()?;
        let reply = socket.send(Request::EventStream)?;

        match reply {
            Ok(Response::Handled) => {}
            _ => {
                return Err("Failed to get event stream".into());
            }
        }

        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let mut registry = Self {
            socket: Some(socket),
            event_tx,
            event_rx: Some(event_rx),
            verbosity,
        };

        registry.start_event_listener().await;

        Ok(registry)
    }

    pub fn into_events(mut self) -> impl Stream<Item = NiriEvent> + Send + Unpin {
        UnboundedReceiverStream::new(self.event_rx.take().unwrap())
    }

    pub fn focus_events(self) -> impl Stream<Item = NiriEvent> + Send + Unpin {
        self.into_events()
            .filter(|event| matches!(event, NiriEvent::Focus { .. }))
    }

    pub fn blur_events(self) -> impl Stream<Item = NiriEvent> + Send + Unpin {
        self.into_events()
            .filter(|event| matches!(event, NiriEvent::Blur { .. }))
    }

    pub fn windows_matching<P>(self, predicate: P) -> impl Stream<Item = NiriEvent> + Send + Unpin
    where
        P: Fn(&WindowInfo) -> bool + Send + Sync,
    {
        self.into_events().filter(move |event| {
            if let Some(window) = event.window() {
                (predicate)(window)
            } else {
                false
            }
        })
    }

    pub fn window_events(self) -> impl Stream<Item = NiriEvent> + Send + Unpin {
        self.into_events()
            .filter(|event| matches!(event, NiriEvent::Focus { .. } | NiriEvent::Blur { .. }))
    }

    pub fn filter_map<F, R>(self, f: F) -> impl Stream<Item = R> + Send + Unpin
    where
        F: Fn(NiriEvent) -> Option<R> + Send + Sync,
    {
        self.into_events().filter_map(f)
    }

    async fn start_event_listener(&mut self) {
        let socket = self.socket.take().unwrap();
        let mut read_event = socket.read_events();
        let tx = self.event_tx.clone();
        let verbosity = self.verbosity;

        tokio::spawn(async move {
            let mut last_focused_window_id: Option<u64> = None;

            while let Ok(event) = read_event() {
                if verbosity.log_all_events() {
                    eprintln!("Niri event: {:?}", event);
                }

                match event {
                    Event::WindowFocusTimestampChanged { id, .. } => {
                        if verbosity.log_window_events() {
                            eprintln!("WindowFocusTimestampChanged: id={}", id);
                        }

                        if let Some(window_info) = Self::get_window_info(id).await {
                            if verbosity.log_window_events() {
                                eprintln!(
                                    "Focus event: window_id={}, app_id={:?}, pid={:?}",
                                    id, window_info.app_id, window_info.pid
                                );
                            }

                            if let Some(prev_id) = last_focused_window_id {
                                if prev_id != id {
                                    if let Some(prev_window_info) = Self::get_window_info(prev_id).await {
                                        if verbosity.log_window_events() {
                                            eprintln!(
                                                "Blur event: window_id={}, app_id={:?}, pid={:?}",
                                                prev_id, prev_window_info.app_id, prev_window_info.pid
                                            );
                                        }
                                        let niri_event = NiriEvent::Blur {
                                            window_id: prev_id,
                                            window: prev_window_info,
                                        };
                                        if let Err(_) = tx.send(niri_event) {
                                            break;
                                        }
                                    }
                                }
                            }

                            let niri_event = NiriEvent::Focus {
                                window_id: id,
                                window: window_info,
                            };

                            if let Err(_) = tx.send(niri_event) {
                                break;
                            }

                            last_focused_window_id = Some(id);
                        }
                    }
                    Event::WindowFocusChanged { id: None } => {
                        last_focused_window_id = None;
                    }
                    Event::WindowsChanged { windows } => {
                        if last_focused_window_id.is_none() {
                            if let Some(focused_window) = windows.iter().find(|w| w.is_focused) {
                                if verbosity.log_window_events() {
                                    eprintln!(
                                        "Initial focus detected: window_id={}, app_id={:?}, pid={:?}",
                                        focused_window.id, focused_window.app_id, focused_window.pid
                                    );
                                }

                                let window_info = WindowInfo::from_niri_window(focused_window);
                                let niri_event = NiriEvent::Focus {
                                    window_id: focused_window.id,
                                    window: window_info,
                                };

                                if let Err(_) = tx.send(niri_event) {
                                    break;
                                }

                                last_focused_window_id = Some(focused_window.id);
                            }
                        }
                    }
                    _ => continue,
                }
            }
        });
    }

    async fn get_window_info(window_id: u64) -> Option<WindowInfo> {
        let mut socket = Socket::connect().ok()?;
        let reply = socket.send(Request::Windows).ok()?;
        let windows = match reply {
            Ok(Response::Windows(windows)) => windows,
            _ => return None,
        };

        windows
            .iter()
            .find(|w| w.id == window_id)
            .map(WindowInfo::from_niri_window)
    }
}
