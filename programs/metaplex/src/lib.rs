pub mod constants;
pub mod error;
pub mod instructions;
pub mod plugins;
pub mod state;

use anchor_lang::prelude::*;
pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("9Z9c67r7RFUADpWTTJxgbidobiNnHTtzLodTa6JQSBaX");

#[program]
pub mod metaplex {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        rewards_per_second: u64,
        burn_bonus: u64,
    ) -> Result<()> {
        ctx.accounts
            .handler(rewards_per_second, burn_bonus, &ctx.bumps)
    }

    pub fn stake(ctx: Context<Stake>) -> Result<()> {
        ctx.accounts.handler(&ctx.bumps)
    }

    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        ctx.accounts.handler()
    }

    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        ctx.accounts.handler()
    }
}
