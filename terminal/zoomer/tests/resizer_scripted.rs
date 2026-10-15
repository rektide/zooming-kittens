//! Scripted end-to-end tests for the zoomer policy over a fake command
//! transport. No live kitty instance is contacted: the sink records every
//! adjustment and replays scripted results, so the tests observe exactly what
//! the policy would send and how its tracked state reacts to failures.

use std::collections::VecDeque;
use std::sync::Mutex;

use futures::executor::block_on;
use futures::stream::iter;
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
