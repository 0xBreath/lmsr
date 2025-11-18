#![allow(unexpected_cfgs)]
#![allow(
    deprecated,
    reason = "Anchor internally calls AccountInfo::realloc (see PR #3803)"
)]
use anchor_lang::prelude::*;

use instructions::*;
use types::*;

pub mod instructions;
pub mod state;
pub mod types;

declare_id!("JDP9AsSqpzeea8yqscvMHU7gkvC7QR16UF35hf74tAFG");

#[program]
pub mod lmsr {
    use super::*;

    /// Create a new market with N outcomes
    pub fn init_market<'info>(
        ctx: Context<'_, '_, 'info, 'info, InitMarket<'info>>,
        num_outcomes: u8,
        scale: u64,
        resolve_at: i64,
        label: FixedSizeString,
    ) -> Result<()> {
        instructions::init_market(ctx, num_outcomes, scale, resolve_at, label)
    }
}
