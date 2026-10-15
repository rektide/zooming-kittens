//! Pure zoom transition planning.
//!
//! A plan converts the policy's current per-window state, the zoom
//! configuration and one focus event into a list of remote adjustments with no
//! I/O. The resizer executes the commands and commits state; everything in
//! this module is deterministic and unit-testable.
//!
//! ## Step semantics
//!
//! Kitty's `set-font-size` remote command accepts an increment operator (`+`,
//! `-`, `*`, `/`) and a positive float size and applies `old <op> size`
//! atomically, so any intended change can be expressed as exactly one
//! command. `step_size` therefore only controls how a change is *split* into
//! intermediate commands for smoother visible transitions:
//!
//! - Additive/absolute deltas are split into `floor(|delta| / step)` full-step
//!   commands plus one exact remainder command, so the applied total equals
//!   the intended delta exactly. (The inherited code used `ceil` and
//!   truncated through a `u32` cast, which overshot targets and could send
//!   zero commands for fractional steps.)
//! - Multiplicative transitions step geometrically (`*step` when growing,
//!   `/step` when shrinking) with one exact remainder command, so the product
//!   of all commands equals the intended factor exactly.
//! - A step that cannot make progress (multiplicative `step <= 1`, which
//!   includes the default `1.0`) or a plan that would need more than
//!   [`MAX_PLAN_STEPS`] commands collapses to the single exact command. This
//!   structurally guarantees termination; the inherited loop spun forever on
//!   `*1.0`.

use crate::config::{ZoomConfig, ZoomType};

/// Upper bound on intermediate commands; larger intended changes collapse to
/// one exact command instead of an unbounded animation.
pub const MAX_PLAN_STEPS: u32 = 128;

/// Epsilon for factor comparisons, inherited from the original policy.
const FACTOR_EPSILON: f64 = 0.001;

/// One remote font adjustment, sent as a single `set-font-size` command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontAdjustment {
    pub op: FontOp,
    /// Always finite and strictly positive; direction is carried by `op`.
    pub amount: f64,
}

/// The increment operator of a kitty `set-font-size` command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl FontOp {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
        }
    }
}

impl FontAdjustment {
    pub fn new(op: FontOp, amount: f64) -> Option<Self> {
        amount.is_finite().then_some(amount).filter(|a| *a > 0.0).map(|amount| Self {
            op,
            amount,
        })
    }
}

/// The policy's per-window view used for planning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowState {
    /// Last known font size in points, if any.
    pub font: Option<f64>,
    /// Last known zoom factor relative to the configured baseline.
    pub factor: f64,
}

impl WindowState {
    pub fn with_baseline(baseline: Option<f64>) -> Self {
        Self {
            font: baseline,
            factor: 1.0,
        }
    }
}

/// Which event is being planned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanEvent {
    Focus,
    Blur,
}

/// The result of planning one event.
#[derive(Debug, Clone, PartialEq)]
pub enum ZoomPlan {
    /// Nothing to do (already at target, or no zoom configured).
    Noop,
    /// Apply the commands in order; when every one succeeds the font size is
    /// `target` and (for multiplicative zoom) the factor is `target_factor`.
    Transition {
        commands: Vec<FontAdjustment>,
        from: f64,
        target: f64,
        target_factor: Option<f64>,
    },
    /// Configuration or tracked state is unusable (e.g. nonfinite); no
    /// commands are sent and local state must not change.
    Invalid(String),
}

/// Plan the transition for one event.
///
/// `baseline` is the configured kitty.conf font size; an untracked font size
/// falls back to it (and then to the historical `12.0` default), matching the
/// inherited policy.
pub fn plan_transition(
    config: &ZoomConfig,
    event: PlanEvent,
    state: &WindowState,
    baseline: Option<f64>,
) -> ZoomPlan {
    let Some(zoom_type) = config.active_type() else {
        return ZoomPlan::Noop;
    };

    if !valid_config(config) {
        return ZoomPlan::Invalid(format!(
            "zoom configuration is invalid (nonfinite or non-positive values); \
             step_size={}",
            config.step_size
        ));
    }

    let baseline = baseline.unwrap_or(12.0);
    if !baseline.is_finite() || baseline <= 0.0 {
        return ZoomPlan::Invalid(format!("baseline font size {baseline} is unusable"));
    }

    match zoom_type {
        ZoomType::Absolute => plan_absolute(config, event, state, baseline),
        ZoomType::Additive => plan_additive(config, event, state, baseline),
        ZoomType::Multiplicative => plan_multiplicative(config, event, state, baseline),
    }
}

/// The state after a (possibly partial) application of a plan's commands.
///
/// The resizer feeds back the suffix of commands that kitty acknowledged so
/// the committed state never claims effects that were not applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppliedEffect {
    /// Font size after the applied additive/absolute commands.
    pub font: f64,
    /// Factor after the applied multiplicative commands.
    pub factor: f64,
}

/// Compute the effect of the first `applied` commands on `state`.
///
/// `applied` is clamped to the command count. The resizer passes only the
/// commands kitty acknowledged, so the committed state never claims effects
/// that were not applied.
pub fn applied_effect(
    commands: &[FontAdjustment],
    state: &WindowState,
    applied: usize,
) -> Option<AppliedEffect> {
    let mut font = state.font?;
    let mut factor = state.factor;
    for adjustment in commands.iter().take(applied) {
        match adjustment.op {
            FontOp::Add => font += adjustment.amount,
            FontOp::Sub => font -= adjustment.amount,
            FontOp::Mul => factor *= adjustment.amount,
            FontOp::Div => factor /= adjustment.amount,
        }
    }
    if !font.is_finite() || !factor.is_finite() {
        return None;
    }
    Some(AppliedEffect { font, factor })
}

fn valid_config(config: &ZoomConfig) -> bool {
    config.step_size.is_finite()
        && config.step_size > 0.0
        && config.absolute.map(finite_positive).unwrap_or(true)
        && config.additive.map(|v| v.is_finite()).unwrap_or(true)
        && config.multiplicative.map(finite_positive).unwrap_or(true)
}

fn finite_positive(v: f64) -> bool {
    v.is_finite() && v > 0.0
}

/// The effective current font size: the tracked value when finite, otherwise
/// the baseline (already defaulted and validated by [`plan_transition`]).
///
/// A nonfinite tracked size is treated as untracked and replanned from the
/// baseline instead of poisoning every later transition.
fn effective_font(state: &WindowState, baseline: f64) -> f64 {
    state.font.filter(|v| v.is_finite()).unwrap_or(baseline)
}

fn plan_absolute(
    config: &ZoomConfig,
    event: PlanEvent,
    state: &WindowState,
    baseline: f64,
) -> ZoomPlan {
    let Some(target) = config.absolute else {
        return ZoomPlan::Noop;
    };
    let target = match event {
        // Focus moves to the configured size; blur restores the baseline.
        PlanEvent::Focus => target,
        PlanEvent::Blur => baseline,
    };

    additive_transition(effective_font(state, baseline), target, config.step_size, None)
}

fn plan_additive(
    config: &ZoomConfig,
    event: PlanEvent,
    state: &WindowState,
    baseline: f64,
) -> ZoomPlan {
    let Some(amount) = config.additive else {
        return ZoomPlan::Noop;
    };
    let current = effective_font(state, baseline);
    let signed = match event {
        PlanEvent::Focus => amount,
        PlanEvent::Blur => -amount,
    };
    additive_transition(current, current + signed, config.step_size, None)
}

fn plan_multiplicative(
    config: &ZoomConfig,
    event: PlanEvent,
    state: &WindowState,
    baseline: f64,
) -> ZoomPlan {
    let Some(configured) = config.multiplicative else {
        return ZoomPlan::Noop;
    };
    let target_factor = match event {
        PlanEvent::Focus => configured,
        PlanEvent::Blur => 1.0,
    };
    let current_factor = state.factor;
    if !current_factor.is_finite() || current_factor <= 0.0 {
        return ZoomPlan::Invalid(format!(
            "tracked zoom factor {current_factor} is unusable"
        ));
    }
    if (target_factor - current_factor).abs() <= FACTOR_EPSILON {
        return ZoomPlan::Noop;
    }

    let from_font = effective_font(state, baseline);
    let commands = factor_commands(current_factor, target_factor, config.step_size);
    // The inherited policy records the font target as baseline * factor.
    finish_transition(from_font, commands, baseline * target_factor, Some(target_factor))
}

/// Split `target - current` into stepped additive commands with an exact sum.
fn additive_transition(
    current: f64,
    target: f64,
    step: f64,
    target_factor: Option<f64>,
) -> ZoomPlan {
    if !target.is_finite() {
        return ZoomPlan::Invalid(format!("target font size {target} is nonfinite"));
    }
    let delta = target - current;
    if delta.abs() <= f64::EPSILON {
        return ZoomPlan::Noop;
    }

    let op = if delta > 0.0 { FontOp::Add } else { FontOp::Sub };
    let commands = linear_commands(delta.abs(), step, op);
    finish_transition(current, commands, target, target_factor)
}

/// Validate the command list and report the transition.
fn finish_transition(
    from: f64,
    commands: Vec<FontAdjustment>,
    target: f64,
    target_factor: Option<f64>,
) -> ZoomPlan {
    if commands.is_empty() {
        return ZoomPlan::Noop;
    }
    if commands.iter().any(|c| !c.amount.is_finite() || c.amount <= 0.0) {
        return ZoomPlan::Invalid("planned a nonfinite or non-positive command".into());
    }
    if !target.is_finite() {
        return ZoomPlan::Invalid(format!("target font size {target} is nonfinite"));
    }
    ZoomPlan::Transition {
        commands,
        from,
        target,
        target_factor,
    }
}

/// Build `floor(total / step)` full-step commands plus one exact remainder.
fn linear_commands(total: f64, step: f64, op: FontOp) -> Vec<FontAdjustment> {
    if step.is_finite() && step > 0.0 && total / step <= f64::from(MAX_PLAN_STEPS) {
        let full = (total / step).floor();
        let mut commands = vec![FontAdjustment { op, amount: step }; full as usize];
        let remainder = total - full * step;
        if remainder > 0.0 {
            commands.push(FontAdjustment { op, amount: remainder });
        }
        return commands;
    }
    // Stepping is unsuitable or would exceed the budget: one exact command so
    // the applied sum still equals the intended change.
    vec![FontAdjustment { op, amount: total }]
}

/// Build stepped multiplicative commands whose product is exact.
fn factor_commands(from: f64, to: f64, step: f64) -> Vec<FontAdjustment> {
    let growing = to > from;
    let step_op = if growing { FontOp::Mul } else { FontOp::Div };
    // A multiplicative step must move the factor toward the target; `step <= 1`
    // cannot grow and cannot shrink by dividing, so it collapses to one exact
    // command below (this includes the historical default of 1.0, which
    // previously looped forever on `*1.0`).
    if !(step.is_finite() && step > 1.0) {
        let amount = if growing { to / from } else { from / to };
        return vec![FontAdjustment { op: step_op, amount }];
    }

    let mut commands = Vec::new();
    let mut current = from;
    // Bound the geometric walk; the remainder command stays exact.
    for _ in 0..MAX_PLAN_STEPS {
        let next = if growing { current * step } else { current / step };
        let overshoots = if growing { next > to } else { next < to };
        if overshoots {
            break;
        }
        commands.push(FontAdjustment { op: step_op, amount: step });
        current = next;
        if (current - to).abs() <= f64::EPSILON {
            return commands;
        }
    }
    let remainder = if growing { to / current } else { current / to };
    commands.push(FontAdjustment { op: step_op, amount: remainder });
    commands
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with(zoom: impl Into<ZoomSetting>) -> ZoomConfig {
        let mut config = ZoomConfig::default();
        match zoom.into() {
            ZoomSetting::Absolute(v) => config.absolute = Some(v),
            ZoomSetting::Additive(v) => config.additive = Some(v),
            ZoomSetting::Multiplicative(v) => config.multiplicative = Some(v),
        }
        config
    }

    enum ZoomSetting {
        Absolute(f64),
        Additive(f64),
        Multiplicative(f64),
    }

    impl From<f64> for ZoomSetting {
        fn from(v: f64) -> Self {
            ZoomSetting::Additive(v)
        }
    }

    fn state(font: f64) -> WindowState {
        WindowState {
            font: Some(font),
            factor: 1.0,
        }
    }

    fn command_sum(commands: &[FontAdjustment]) -> f64 {
        commands
            .iter()
            .map(|c| match c.op {
                FontOp::Add => c.amount,
                FontOp::Sub => -c.amount,
                FontOp::Mul | FontOp::Div => 0.0,
            })
            .sum()
    }

    fn command_factor(commands: &[FontAdjustment]) -> f64 {
        commands
            .iter()
            .fold(1.0, |acc, c| match c.op {
                FontOp::Add | FontOp::Sub => acc,
                FontOp::Mul => acc * c.amount,
                FontOp::Div => acc / c.amount,
            })
    }

    fn transition(plan: ZoomPlan) -> (Vec<FontAdjustment>, f64) {
        match plan {
            ZoomPlan::Transition {
                commands, target, ..
            } => (commands, target),
            other => panic!("expected a transition, got {other:?}"),
        }
    }

    #[test]
    fn additive_focus_steps_by_one_with_default_step() {
        let config = config_with(6.0);
        let plan = plan_transition(
            &config,
            PlanEvent::Focus,
            &state(12.0),
            Some(12.0),
        );

        let (commands, target) = transition(plan);
        assert_eq!(commands, vec![adjust(FontOp::Add, 1.0); 6]);
        assert_eq!(target, 18.0);
        assert!((command_sum(&commands) - 6.0).abs() < 1e-12);
    }

    #[test]
    fn additive_fractional_step_applies_exact_total() {
        let mut config = config_with(6.0);
        config.step_size = 0.5;
        let plan = plan_transition(&config, PlanEvent::Focus, &state(12.0), Some(12.0));

        let (commands, target) = transition(plan);
        assert_eq!(commands.len(), 12);
        assert_eq!(commands[0].amount, 0.5);
        assert!((command_sum(&commands) - 6.0).abs() < 1e-12);
        assert_eq!(target, 18.0);
    }

    #[test]
    fn additive_remainder_command_keeps_intended_total() {
        // delta 5 with step 2: two +2 steps plus one exact +1 remainder. The
        // inherited ceil() behavior sent +6 instead of +5.
        let mut config = config_with(5.0);
        config.step_size = 2.0;
        let plan = plan_transition(&config, PlanEvent::Focus, &state(12.0), Some(12.0));

        let (commands, target) = transition(plan);
        assert_eq!(
            commands,
            vec![adjust(FontOp::Add, 2.0), adjust(FontOp::Add, 2.0), adjust(FontOp::Add, 1.0)]
        );
        assert_eq!(target, 17.0);
    }

    #[test]
    fn fractional_delta_with_unit_step_is_exact() {
        // delta 2.5 with step 1: the inherited truncation sent +2 while
        // recording 2.5; the plan must sum to exactly 2.5.
        let mut config = config_with(2.5);
        config.step_size = 1.0;
        let plan = plan_transition(&config, PlanEvent::Focus, &state(12.0), Some(12.0));

        let (commands, _) = transition(plan);
        assert_eq!(commands.last().copied(), Some(adjust(FontOp::Add, 0.5)));
        assert!((command_sum(&commands) - 2.5).abs() < 1e-12);
    }

    #[test]
    fn additive_blur_subtracts_the_same_amount() {
        let config = config_with(6.0);
        let plan = plan_transition(&config, PlanEvent::Blur, &state(18.0), Some(12.0));

        let (commands, target) = transition(plan);
        assert_eq!(commands, vec![adjust(FontOp::Sub, 1.0); 6]);
        assert_eq!(target, 12.0);
    }

    #[test]
    fn absolute_focus_targets_configured_size() {
        let config = config_with(ZoomSetting::Absolute(18.0));
        let plan = plan_transition(&config, PlanEvent::Focus, &state(12.0), Some(12.0));

        let (commands, target) = transition(plan);
        assert!((command_sum(&commands) - 6.0).abs() < 1e-12);
        assert_eq!(target, 18.0);
    }

    #[test]
    fn absolute_blur_restores_configured_baseline() {
        let config = config_with(ZoomSetting::Absolute(18.0));
        let plan = plan_transition(&config, PlanEvent::Blur, &state(18.0), Some(11.5));

        let (commands, target) = transition(plan);
        assert!((command_sum(&commands) + 6.5).abs() < 1e-12);
        assert_eq!(target, 11.5);
    }

    #[test]
    fn absolute_blur_grows_back_when_baseline_is_larger() {
        let config = config_with(ZoomSetting::Absolute(10.0));
        let plan = plan_transition(&config, PlanEvent::Blur, &state(10.0), Some(14.0));

        let (commands, target) = transition(plan);
        assert!((command_sum(&commands) - 4.0).abs() < 1e-12);
        assert_eq!(target, 14.0);
    }

    #[test]
    fn multiplicative_default_step_is_one_exact_command() {
        // The inherited loop multiplied by the default step 1.0 forever.
        let config = config_with(ZoomSetting::Multiplicative(1.5));
        let plan = plan_transition(&config, PlanEvent::Focus, &state(12.0), Some(12.0));

        let (commands, target) = transition(plan);
        assert_eq!(commands, vec![adjust(FontOp::Mul, 1.5)]);
        assert_eq!(target, 18.0);
    }

    #[test]
    fn multiplicative_step_splits_geometrically_with_exact_product() {
        let mut config = config_with(ZoomSetting::Multiplicative(2.0));
        config.step_size = 1.5;
        let plan = plan_transition(&config, PlanEvent::Focus, &state(12.0), Some(12.0));

        let (commands, target) = transition(plan);
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0], adjust(FontOp::Mul, 1.5));
        assert!((command_factor(&commands) - 2.0).abs() < 1e-12);
        assert!((target - 24.0).abs() < 1e-12);
    }

    #[test]
    fn multiplicative_blur_returns_to_factor_one() {
        let config = config_with(ZoomSetting::Multiplicative(1.5));
        let zoomed = WindowState {
            font: Some(18.0),
            factor: 1.5,
        };
        let plan = plan_transition(&config, PlanEvent::Blur, &zoomed, Some(12.0));

        let (commands, target) = transition(plan);
        assert_eq!(commands, vec![adjust(FontOp::Div, 1.5)]);
        assert_eq!(target, 12.0);
        match plan_transition(&config, PlanEvent::Blur, &zoomed, Some(12.0)) {
            ZoomPlan::Transition { target_factor, .. } => {
                assert_eq!(target_factor, Some(1.0))
            }
            other => panic!("expected a transition, got {other:?}"),
        }
    }

    #[test]
    fn multiplicative_subunit_step_collapses_to_exact_command() {
        // step < 1 cannot make progress toward a larger factor; the plan must
        // still reach the target in one command instead of regressing.
        let mut config = config_with(ZoomSetting::Multiplicative(2.0));
        config.step_size = 0.9;
        let plan = plan_transition(&config, PlanEvent::Focus, &state(12.0), Some(12.0));

        let (commands, target) = transition(plan);
        assert_eq!(commands, vec![adjust(FontOp::Mul, 2.0)]);
        assert!((target - 24.0).abs() < 1e-12);
    }

    #[test]
    fn zero_delta_and_epsilon_factor_are_noops() {
        let config = config_with(ZoomSetting::Absolute(12.0));
        assert_eq!(
            plan_transition(&config, PlanEvent::Focus, &state(12.0), Some(12.0)),
            ZoomPlan::Noop
        );

        let config = config_with(ZoomSetting::Multiplicative(1.0));
        assert_eq!(
            plan_transition(&config, PlanEvent::Focus, &state(12.0), Some(12.0)),
            ZoomPlan::Noop
        );
    }

    #[test]
    fn unconfigured_zoom_is_noop() {
        let plan = plan_transition(&ZoomConfig::default(), PlanEvent::Focus, &state(12.0), Some(12.0));
        assert_eq!(plan, ZoomPlan::Noop);
    }

    #[test]
    fn invalid_step_size_and_amounts_are_rejected_before_commands() {
        let mut config = config_with(6.0);
        config.step_size = 0.0;
        assert!(matches!(
            plan_transition(&config, PlanEvent::Focus, &state(12.0), Some(12.0)),
            ZoomPlan::Invalid(_)
        ));

        config.step_size = f64::NAN;
        assert!(matches!(
            plan_transition(&config, PlanEvent::Focus, &state(12.0), Some(12.0)),
            ZoomPlan::Invalid(_)
        ));

        let config = config_with(ZoomSetting::Multiplicative(0.0));
        assert!(matches!(
            plan_transition(&config, PlanEvent::Focus, &state(12.0), Some(12.0)),
            ZoomPlan::Invalid(_)
        ));

        let config = config_with(ZoomSetting::Multiplicative(-1.5));
        assert!(matches!(
            plan_transition(&config, PlanEvent::Focus, &state(12.0), Some(12.0)),
            ZoomPlan::Invalid(_)
        ));

        let config = config_with(f64::INFINITY);
        assert!(matches!(
            plan_transition(&config, PlanEvent::Focus, &state(12.0), Some(12.0)),
            ZoomPlan::Invalid(_)
        ));
    }

    #[test]
    fn nonfinite_tracked_font_falls_back_to_baseline() {
        // A poisoned size is treated as untracked and replanned from the
        // baseline instead of failing every later transition.
        let config = config_with(6.0);
        let bad = WindowState {
            font: Some(f64::NAN),
            factor: 1.0,
        };
        let (commands, target) = transition(plan_transition(
            &config,
            PlanEvent::Focus,
            &bad,
            Some(12.0),
        ));
        assert!((command_sum(&commands) - 6.0).abs() < 1e-12);
        assert_eq!(target, 18.0);
    }

    #[test]
    fn missing_baseline_falls_back_to_twelve() {
        let config = config_with(ZoomSetting::Absolute(18.0));
        let plan = plan_transition(&config, PlanEvent::Blur, &state(18.0), None);

        let (commands, target) = transition(plan);
        assert!((command_sum(&commands) + 6.0).abs() < 1e-12);
        assert_eq!(target, 12.0);
    }

    #[test]
    fn huge_delta_collapses_to_bounded_plan() {
        let mut config = config_with(600.0);
        config.step_size = 0.5;
        let plan = plan_transition(&config, PlanEvent::Focus, &state(12.0), Some(12.0));

        let (commands, target) = transition(plan);
        assert!(commands.len() <= MAX_PLAN_STEPS as usize + 1);
        assert!((command_sum(&commands) - 600.0).abs() < 1e-9);
        assert_eq!(target, 612.0);
    }

    #[test]
    fn applied_effect_counts_only_acknowledged_commands() {
        let commands = linear_commands(6.0, 2.0, FontOp::Add);

        let effect = applied_effect(&commands, &state(12.0), 2).unwrap();
        assert_eq!(effect.font, 16.0); // two +2 commands applied

        let full = applied_effect(&commands, &state(12.0), 3).unwrap();
        assert_eq!(full.font, 18.0); // +2 +2 +2
    }

    #[test]
    fn applied_effect_is_none_without_tracked_font() {
        let commands = linear_commands(6.0, 2.0, FontOp::Add);
        let untracked = WindowState {
            font: None,
            factor: 1.0,
        };
        assert!(applied_effect(&commands, &untracked, 2).is_none());
    }

    fn adjust(op: FontOp, amount: f64) -> FontAdjustment {
        FontAdjustment::new(op, amount).unwrap()
    }
}
