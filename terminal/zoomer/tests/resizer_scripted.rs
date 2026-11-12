//! Scripted end-to-end tests for the zoomer policy over a fake command
//! transport. No live kitty instance is contacted: the sink records every
//! adjustment and replays scripted results, so the tests observe exactly what
//! the policy would send and how its tracked state reacts to failures.

use std::collections::VecDeque;
use std::sync::Mutex;

use futures::executor::block_on;
use futures::{StreamExt, stream::iter};
use zk_kitty::ZoomingResult;
use zk_kitty::conf_parser::set_baseline_font_size;
use zk_zoomer::{
    FocusEvent, FocusTarget, FontAdjustment, FontOp, FontPort, ZoomConfig, ZoomResizer,
};

/// Records every command and replies with scripted results; when the script
/// runs dry it answers `NotConfigured`.
struct ScriptedPort {
    calls: Mutex<Vec<(i32, FontAdjustment)>>,
    results: Mutex<VecDeque<Result<ZoomingResult, Box<dyn std::error::Error + Send + Sync>>>>,
}

impl ScriptedPort {
    fn new(results: Vec<Result<ZoomingResult, Box<dyn std::error::Error + Send + Sync>>>) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            results: Mutex::new(results.into()),
        }
    }

    fn calls(&self) -> Vec<(i32, FontAdjustment)> {
        self.calls.lock().unwrap().clone()
    }
}

impl FontPort for ScriptedPort {
    async fn adjust_font(
        &self,
        pid: i32,
        adjustment: FontAdjustment,
    ) -> Result<ZoomingResult, Box<dyn std::error::Error + Send + Sync>> {
        self.calls.lock().unwrap().push((pid, adjustment));
        self.results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Ok(ZoomingResult::NotConfigured))
    }
}

fn success() -> Result<ZoomingResult, Box<dyn std::error::Error + Send + Sync>> {
    Ok(ZoomingResult::Success {
        pid: 0,
        font_adjustment: String::new(),
    })
}

fn target(pid: i32) -> FocusTarget {
    FocusTarget {
        id: format!("window-{pid}"),
        pid: Some(pid),
    }
}

fn run_with_handle(
    port: &ScriptedPort,
    config: ZoomConfig,
    events: Vec<FocusEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    set_baseline_font_size(12.0);
    let mut resizer = ZoomResizer::with_zoom_config(RecordedPort(port), config);
    block_on(resizer.process_events(iter(events)))
}

/// Forwards to a borrowed port so the test keeps access to the recordings.
struct RecordedPort<'a>(&'a ScriptedPort);

impl FontPort for RecordedPort<'_> {
    async fn adjust_font(
        &self,
        pid: i32,
        adjustment: FontAdjustment,
    ) -> Result<ZoomingResult, Box<dyn std::error::Error + Send + Sync>> {
        self.0.adjust_font(pid, adjustment).await
    }
}

#[test]
fn additive_focus_and_blur_apply_exact_symmetric_totals() {
    let port = ScriptedPort::new((0..16).map(|_| success()).collect());
    let config = ZoomConfig {
        additive: Some(6.0),
        ..ZoomConfig::default()
    };

    run_with_handle(
        &port,
        config,
        vec![
            FocusEvent::Focus { window: target(7) },
            FocusEvent::Blur { window: target(7) },
        ],
    )
    .unwrap();

    let calls = port.calls();
    assert_eq!(calls.len(), 12, "focus +6 and blur -6 with unit steps");
    assert!(
        calls[..6]
            .iter()
            .all(|(pid, adj)| *pid == 7 && adj.op == FontOp::Add && adj.amount == 1.0)
    );
    assert!(
        calls[6..]
            .iter()
            .all(|(pid, adj)| *pid == 7 && adj.op == FontOp::Sub && adj.amount == 1.0)
    );
}

#[test]
fn failed_command_stops_the_plan_and_advances_state_by_the_applied_portion() {
    // +6 with step 2 plans three +2 commands. The third is refused, so the
    // tracked size must advance by 4 (to 16), not 6. The following blur
    // therefore plans -6 from 16 (three -2 commands, target 10) instead of
    // -6 from a phantom 18: honest bookkeeping, where the inherited code
    // recorded the full +6 regardless of the failure.
    let port = ScriptedPort::new(vec![
        success(),
        success(),
        Ok(ZoomingResult::ConnectionFailed),
        success(),
        success(),
        success(),
    ]);
    let config = ZoomConfig {
        additive: Some(6.0),
        step_size: 2.0,
        ..ZoomConfig::default()
    };

    run_with_handle(
        &port,
        config,
        vec![
            FocusEvent::Focus { window: target(3) },
            FocusEvent::Blur { window: target(3) },
        ],
    )
    .unwrap();

    let calls = port.calls();
    assert_eq!(
        calls.len(),
        6,
        "3rd focus command refused (still recorded), blur replans from 16"
    );
    assert!(
        calls[..3]
            .iter()
            .all(|(_, adj)| adj.op == FontOp::Add && adj.amount == 2.0)
    );
    assert!(
        calls[3..]
            .iter()
            .all(|(_, adj)| adj.op == FontOp::Sub && adj.amount == 2.0)
    );
}

#[test]
fn not_configured_first_command_leaves_state_untouched() {
    // No kitty master/socket: nothing is applied, so a later focus replans
    // the full +6 from the original size instead of compounding on a phantom
    // adjustment.
    let port = ScriptedPort::new(vec![
        Ok(ZoomingResult::NotConfigured),
        success(),
        success(),
        success(),
        success(),
        success(),
        success(),
    ]);
    let config = ZoomConfig {
        additive: Some(6.0),
        ..ZoomConfig::default()
    };

    run_with_handle(
        &port,
        config,
        vec![
            FocusEvent::Focus { window: target(9) },
            FocusEvent::Focus { window: target(9) },
        ],
    )
    .unwrap();

    let calls = port.calls();
    assert_eq!(
        calls.len(),
        7,
        "first focus sends one command, retry sends six"
    );
    assert!(
        calls[1..]
            .iter()
            .all(|(_, adj)| adj.op == FontOp::Add && adj.amount == 1.0)
    );
}

#[test]
fn multiplicative_default_step_terminates_in_single_commands() {
    // The inherited loop multiplied by the default step 1.0 forever. The
    // policy now reaches each target with one exact command.
    let port = ScriptedPort::new(vec![success(), success()]);
    let config = ZoomConfig {
        multiplicative: Some(1.5),
        ..ZoomConfig::default()
    };

    run_with_handle(
        &port,
        config,
        vec![
            FocusEvent::Focus { window: target(4) },
            FocusEvent::Blur { window: target(4) },
        ],
    )
    .unwrap();

    let calls = port.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[0],
        (
            4,
            FontAdjustment {
                op: FontOp::Mul,
                amount: 1.5
            }
        )
    );
    assert_eq!(
        calls[1],
        (
            4,
            FontAdjustment {
                op: FontOp::Div,
                amount: 1.5
            }
        )
    );
}

#[test]
fn absolute_blur_restores_the_configured_baseline() {
    let port = ScriptedPort::new((0..12).map(|_| success()).collect());
    let config = ZoomConfig {
        absolute: Some(18.0),
        ..ZoomConfig::default()
    };

    run_with_handle(
        &port,
        config,
        vec![
            FocusEvent::Focus { window: target(5) },
            FocusEvent::Blur { window: target(5) },
        ],
    )
    .unwrap();

    let calls = port.calls();
    assert_eq!(calls.len(), 12, "+6 then -6 with unit steps");
    assert!(calls[..6].iter().all(|(_, adj)| adj.op == FontOp::Add));
    assert!(calls[6..].iter().all(|(_, adj)| adj.op == FontOp::Sub));
}

#[test]
fn invalid_config_sends_no_commands() {
    let port = ScriptedPort::new(Vec::new());
    let config = ZoomConfig {
        additive: Some(6.0),
        step_size: 0.0,
        ..ZoomConfig::default()
    };

    run_with_handle(
        &port,
        config,
        vec![
            FocusEvent::Focus { window: target(2) },
            FocusEvent::Blur { window: target(2) },
        ],
    )
    .unwrap();

    assert!(port.calls().is_empty());
}

fn run_and_stop(
    port: &ScriptedPort,
    config: ZoomConfig,
    events: Vec<FocusEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (stop, stopped) = futures::channel::oneshot::channel();
    let mut stop = Some(stop);
    let events = iter(events).chain(futures::stream::poll_fn(move |_| {
        if let Some(stop) = stop.take() {
            let _ = stop.send(());
        }
        std::task::Poll::Pending
    }));
    let mut resizer = ZoomResizer::with_zoom_config(RecordedPort(port), config);
    block_on(resizer.process_events_until(events, async {
        let _ = stopped.await;
    }))
}

#[test]
fn shutdown_restores_only_the_net_acknowledged_effect_and_original_pid() {
    let port = ScriptedPort::new((0..10).map(|_| success()).collect());
    run_and_stop(
        &port,
        ZoomConfig {
            additive: Some(3.0),
            step_size: 3.0,
            ..ZoomConfig::default()
        },
        vec![
            FocusEvent::Focus { window: target(1) },
            FocusEvent::Blur { window: target(1) },
            FocusEvent::Focus { window: target(2) },
        ],
    )
    .unwrap();
    assert_eq!(
        port.calls(),
        vec![
            (
                1,
                FontAdjustment {
                    op: FontOp::Add,
                    amount: 3.0
                }
            ),
            (
                1,
                FontAdjustment {
                    op: FontOp::Sub,
                    amount: 3.0
                }
            ),
            (
                2,
                FontAdjustment {
                    op: FontOp::Add,
                    amount: 3.0
                }
            ),
            (
                2,
                FontAdjustment {
                    op: FontOp::Sub,
                    amount: 3.0
                }
            ),
        ]
    );
}

#[test]
fn shutdown_after_partial_refusal_reverses_only_acknowledged_commands() {
    let port = ScriptedPort::new(vec![
        success(),
        success(),
        Ok(ZoomingResult::ConnectionFailed),
        success(),
    ]);
    run_and_stop(
        &port,
        ZoomConfig {
            additive: Some(6.0),
            step_size: 2.0,
            ..ZoomConfig::default()
        },
        vec![FocusEvent::Focus { window: target(7) }],
    )
    .unwrap();
    assert_eq!(
        port.calls().last(),
        Some(&(
            7,
            FontAdjustment {
                op: FontOp::Sub,
                amount: 4.0
            }
        ))
    );
    assert_eq!(port.calls().len(), 4);
}

#[test]
fn shutdown_multiplicative_inverse_and_refused_restoration_are_honest() {
    let port = ScriptedPort::new(vec![success(), success()]);
    run_and_stop(
        &port,
        ZoomConfig {
            multiplicative: Some(1.5),
            ..ZoomConfig::default()
        },
        vec![FocusEvent::Focus { window: target(7) }],
    )
    .unwrap();
    assert_eq!(
        port.calls().last(),
        Some(&(
            7,
            FontAdjustment {
                op: FontOp::Div,
                amount: 1.5
            }
        ))
    );
    let port = ScriptedPort::new(vec![success(), Ok(ZoomingResult::NotConfigured)]);
    assert!(
        run_and_stop(
            &port,
            ZoomConfig {
                additive: Some(3.0),
                step_size: 3.0,
                ..ZoomConfig::default()
            },
            vec![FocusEvent::Focus { window: target(7) }]
        )
        .is_err()
    );
}

#[test]
fn stop_during_a_command_settles_it_then_restores_without_draining_queued_focus() {
    struct GatedPort {
        calls: Mutex<Vec<(i32, FontAdjustment)>>,
        entered: Mutex<Option<futures::channel::oneshot::Sender<()>>>,
        release: Mutex<Option<futures::channel::oneshot::Receiver<()>>>,
    }
    impl FontPort for &GatedPort {
        async fn adjust_font(
            &self,
            pid: i32,
            adjustment: FontAdjustment,
        ) -> Result<ZoomingResult, Box<dyn std::error::Error + Send + Sync>> {
            self.calls.lock().unwrap().push((pid, adjustment));
            let release = self.release.lock().unwrap().take();
            if let Some(release) = release {
                self.entered
                    .lock()
                    .unwrap()
                    .take()
                    .unwrap()
                    .send(())
                    .unwrap();
                release.await.unwrap();
            }
            success()
        }
    }
    let (entered, entering) = futures::channel::oneshot::channel();
    let (release, released) = futures::channel::oneshot::channel();
    let (stop, stopped) = futures::channel::oneshot::channel();
    let port = GatedPort {
        calls: Mutex::new(vec![]),
        entered: Mutex::new(Some(entered)),
        release: Mutex::new(Some(released)),
    };
    let mut resizer = ZoomResizer::with_zoom_config(
        &port,
        ZoomConfig {
            additive: Some(3.0),
            ..ZoomConfig::default()
        },
    );
    let events = iter(vec![
        FocusEvent::Focus { window: target(7) },
        FocusEvent::Focus { window: target(8) },
    ]);
    let processing = resizer.process_events_until(events, async {
        stopped.await.unwrap();
    });
    let control = async {
        entering.await.unwrap();
        stop.send(()).unwrap();
        release.send(()).unwrap();
    };
    let (result, ()) = block_on(futures::future::join(processing, control));
    result.unwrap();
    assert_eq!(
        *port.calls.lock().unwrap(),
        vec![
            (
                7,
                FontAdjustment {
                    op: FontOp::Add,
                    amount: 1.0
                }
            ),
            (
                7,
                FontAdjustment {
                    op: FontOp::Sub,
                    amount: 1.0
                }
            ),
        ]
    );
}

#[test]
fn stop_between_commands_does_not_start_the_unsent_animation_tail() {
    struct StopOnAck {
        calls: Mutex<Vec<FontAdjustment>>,
        stop: Mutex<Option<futures::channel::oneshot::Sender<()>>>,
    }
    impl FontPort for &StopOnAck {
        async fn adjust_font(
            &self,
            _: i32,
            adjustment: FontAdjustment,
        ) -> Result<ZoomingResult, Box<dyn std::error::Error + Send + Sync>> {
            self.calls.lock().unwrap().push(adjustment);
            if let Some(stop) = self.stop.lock().unwrap().take() {
                stop.send(()).unwrap();
            }
            success()
        }
    }
    let (stop, stopped) = futures::channel::oneshot::channel();
    let port = StopOnAck {
        calls: Mutex::new(vec![]),
        stop: Mutex::new(Some(stop)),
    };
    let mut resizer = ZoomResizer::with_zoom_config(
        &port,
        ZoomConfig {
            additive: Some(3.0),
            ..ZoomConfig::default()
        },
    );
    block_on(resizer.process_events_until(
        iter(vec![FocusEvent::Focus { window: target(7) }]),
        async {
            stopped.await.unwrap();
        },
    ))
    .unwrap();
    assert_eq!(
        *port.calls.lock().unwrap(),
        vec![
            FontAdjustment {
                op: FontOp::Add,
                amount: 1.0
            },
            FontAdjustment {
                op: FontOp::Sub,
                amount: 1.0
            },
        ]
    );
}
