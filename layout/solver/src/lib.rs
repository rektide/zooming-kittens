//! Pure, bounded secondary-axis allocation using Taffy.
//!
//! The caller supplies content/tile semantics and member order. Integer extents
//! conserve `floor(extent)` after explicit gaps; ties go to earlier members.

use std::fmt;

use serde::{Deserialize, Serialize};
use taffy::prelude::*;

/// Largest supported span. Every integer up to this value is exact in Taffy's
/// f32 coordinate representation. Desktop-sized extents are well below it.
pub const MAX_EXTENT: u32 = 1 << 24;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gaps {
    /// `extent` already excludes all native gaps, borders and decorations.
    #[default]
    AlreadyExcluded,
    /// `extent` includes this many pixels between adjacent members, not edges.
    Between { pixels: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Member {
    /// Positive baseline weight, not a flex basis added to the desired share.
    pub weight: f64,
    pub minimum: u32,
    pub maximum: Option<u32>,
}

impl Member {
    pub fn new(weight: f64) -> Self {
        Self {
            weight,
            minimum: 0,
            maximum: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Emphasis {
    pub index: usize,
    /// Strictly between zero and one. Siblings share the remainder according to
    /// their baseline weights; one member always receives the whole budget.
    pub share: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Request<'a> {
    pub extent: f64,
    pub gaps: Gaps,
    pub members: &'a [Member],
    pub emphasis: Option<Emphasis>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Allocation {
    /// Integer extents in precisely the caller's member order.
    pub extents: Vec<u32>,
    pub content_budget: u32,
    pub gap_budget: u32,
    /// `content_budget + gap_budget == floor(request.extent)`.
    pub total_budget: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Infeasible {
    GapsExceedBudget,
    MinimaExceedBudget { required: u64, available: u32 },
    MaximaBelowBudget { capacity: u64, available: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Error {
    InvalidExtent,
    EmptyMembers,
    InvalidWeight {
        index: usize,
    },
    InvalidBounds {
        index: usize,
    },
    InvalidEmphasis,
    Infeasible(Infeasible),
    Engine(String),
    /// Taffy produced nonfinite or nonconserving output. No partial allocation
    /// is returned to the native executor.
    InvalidEngineOutput,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "layout allocation failed: {self:?}")
    }
}

impl std::error::Error for Error {}

/// Solve one affected group using zero-basis Flexbox, explicit bounds and no
/// Taffy rounding. Apply a separate bounded largest-remainder integer pass.
pub fn allocate(request: Request<'_>) -> Result<Allocation, Error> {
    if !request.extent.is_finite() || request.extent < 0.0 || request.extent > f64::from(MAX_EXTENT)
    {
        return Err(Error::InvalidExtent);
    }
    if request.members.is_empty() {
        return Err(Error::EmptyMembers);
    }
    for (index, member) in request.members.iter().enumerate() {
        if !member.weight.is_finite() || member.weight <= 0.0 {
            return Err(Error::InvalidWeight { index });
        }
        if member.maximum.is_some_and(|max| max < member.minimum) {
            return Err(Error::InvalidBounds { index });
        }
    }
    if let Some(emphasis) = request.emphasis
        && (emphasis.index >= request.members.len()
            || !emphasis.share.is_finite()
            || emphasis.share <= 0.0
            || emphasis.share >= 1.0)
    {
        return Err(Error::InvalidEmphasis);
    }

    let total_budget = request.extent.floor() as u32;
    let gap_budget = match request.gaps {
        Gaps::AlreadyExcluded => 0,
        Gaps::Between { pixels } => {
            let count = u32::try_from(request.members.len() - 1)
                .map_err(|_| Error::Infeasible(Infeasible::GapsExceedBudget))?;
            pixels
                .checked_mul(count)
                .ok_or(Error::Infeasible(Infeasible::GapsExceedBudget))?
        }
    };
    let content_budget = total_budget
        .checked_sub(gap_budget)
        .ok_or(Error::Infeasible(Infeasible::GapsExceedBudget))?;
    let minima: u64 = request.members.iter().map(|m| u64::from(m.minimum)).sum();
    if minima > u64::from(content_budget) {
        return Err(Error::Infeasible(Infeasible::MinimaExceedBudget {
            required: minima,
            available: content_budget,
        }));
    }
    // A missing maximum has enough capacity for the complete content budget.
    let maxima: u64 = request
        .members
        .iter()
        .map(|m| u64::from(m.maximum.unwrap_or(content_budget).min(content_budget)))
        .sum();
    if maxima < u64::from(content_budget) {
        return Err(Error::Infeasible(Infeasible::MaximaBelowBudget {
            capacity: maxima,
            available: content_budget,
        }));
    }

    let shares = shares(request.members, request.emphasis);
    let smallest_share = shares.iter().copied().fold(f64::INFINITY, f64::min);
    let mut tree: TaffyTree<()> = TaffyTree::new();
    tree.disable_rounding();
    let mut children = Vec::with_capacity(request.members.len());
    for (member, share) in request.members.iter().zip(shares) {
        // Every grow is >= 1, including the last unfrozen item after bounds
        // saturate. CSS deliberately underfills when remaining grow sums < 1.
        let grow = (share / smallest_share) as f32;
        if !grow.is_finite() || grow <= 0.0 {
            return Err(Error::InvalidEngineOutput);
        }
        children.push(
            tree.new_leaf(Style {
                flex_basis: length(0.0_f32),
                flex_grow: grow,
                flex_shrink: 1.0,
                min_size: Size {
                    width: length(0.0_f32),
                    height: length(member.minimum as f32),
                },
                max_size: Size {
                    width: auto(),
                    height: length(
                        member.maximum.unwrap_or(content_budget).min(content_budget) as f32
                    ),
                },
                ..Default::default()
            })
            .map_err(|error| Error::Engine(error.to_string()))?,
        );
    }
    let gap = match request.gaps {
        Gaps::AlreadyExcluded => 0,
        Gaps::Between { pixels } => pixels,
    };
    let root = tree
        .new_with_children(
            Style {
                flex_direction: FlexDirection::Column,
                size: Size {
                    width: length(1.0_f32),
                    height: length(total_budget as f32),
                },
                gap: Size {
                    width: length(0.0_f32),
                    height: length(gap as f32),
                },
                ..Default::default()
            },
            &children,
        )
        .map_err(|error| Error::Engine(error.to_string()))?;
    tree.compute_layout(root, Size::MAX_CONTENT)
        .map_err(|error| Error::Engine(error.to_string()))?;
    let floats = children
        .iter()
        .map(|&node| {
            tree.layout(node)
                .map(|layout| f64::from(layout.size.height))
                .map_err(|error| Error::Engine(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let extents = round(&floats, request.members, content_budget)?;
    Ok(Allocation {
        extents,
        content_budget,
        gap_budget,
        total_budget,
    })
}

fn shares(members: &[Member], emphasis: Option<Emphasis>) -> Vec<f64> {
    if members.len() == 1 {
        return vec![1.0];
    }
    // Scaling before summation avoids overflow with finite, very large weights.
    let largest = members
        .iter()
        .enumerate()
        .filter(|(index, _)| emphasis.is_none_or(|e| *index != e.index))
        .map(|(_, member)| member.weight)
        .fold(0.0, f64::max);
    let total: f64 = members
        .iter()
        .enumerate()
        .filter(|(index, _)| emphasis.is_none_or(|e| *index != e.index))
        .map(|(_, member)| member.weight / largest)
        .sum();
    members
        .iter()
        .enumerate()
        .map(|(index, member)| match emphasis {
            Some(e) if e.index == index => e.share,
            Some(e) => (1.0 - e.share) * (member.weight / largest) / total,
            None => (member.weight / largest) / total,
        })
        .collect()
}

fn round(floats: &[f64], members: &[Member], budget: u32) -> Result<Vec<u32>, Error> {
    if floats
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(Error::InvalidEngineOutput);
    }
    // Permit f32 arithmetic error, not arbitrary underfill/overflow from a bad
    // engine configuration. Bounds remain exact in the integer output.
    let tolerance = (f64::from(budget) * f64::from(f32::EPSILON) * 8.0).max(0.01);
    if (floats.iter().sum::<f64>() - f64::from(budget)).abs() > tolerance {
        return Err(Error::InvalidEngineOutput);
    }
    let mut output: Vec<u32> = floats
        .iter()
        .zip(members)
        .map(|(value, member)| {
            (value.floor() as u32)
                .max(member.minimum)
                .min(member.maximum.unwrap_or(budget))
        })
        .collect();
    let mut order: Vec<usize> = (0..members.len()).collect();
    order.sort_by(|&a, &b| {
        floats[b]
            .fract()
            .total_cmp(&floats[a].fract())
            .then_with(|| a.cmp(&b))
    });
    let mut total: u64 = output.iter().map(|value| u64::from(*value)).sum();
    while total < u64::from(budget) {
        let previous = total;
        for &index in &order {
            if output[index] < members[index].maximum.unwrap_or(budget) {
                output[index] += 1;
                total += 1;
                if total == u64::from(budget) {
                    break;
                }
            }
        }
        if previous == total {
            return Err(Error::InvalidEngineOutput);
        }
    }
    while total > u64::from(budget) {
        let previous = total;
        // In the exceptional f32-overflow case remove from the smallest
        // remainders, still preferring earlier members for exact ties.
        let mut removal = order.clone();
        removal.sort_by(|&a, &b| {
            floats[a]
                .fract()
                .total_cmp(&floats[b].fract())
                .then_with(|| a.cmp(&b))
        });
        for index in removal {
            if output[index] > members[index].minimum {
                output[index] -= 1;
                total -= 1;
                if total == u64::from(budget) {
                    break;
                }
            }
        }
        if previous == total {
            return Err(Error::InvalidEngineOutput);
        }
    }
    Ok(output)
}
