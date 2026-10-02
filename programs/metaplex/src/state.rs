use anchor_lang::prelude::*;
use anchor_spl::token::{mint_to, MintTo};

use crate::{error::ErrorCode, CONFIG_SEED};

#[account]
#[derive(InitSpace)]
pub struct StakeConfig {
    pub collection: Pubkey,
    pub rewards_per_second: u64,
    pub burn_bonus: u64,
    pub bump: u8,
    pub rewards_bump: u8,
}

impl StakeConfig {
    pub fn mint_rewards<'info>(
        &self,
        authority: AccountInfo<'info>,
        mint: AccountInfo<'info>,
        to: AccountInfo<'info>,
        token_program: AccountInfo<'info>,
        amount: u64,
    ) -> Result<()> {
        if amount == 0 {
            return Ok(());
        }
        let bump = [self.bump];
        let signer_seeds: &[&[&[u8]]] = &[&[CONFIG_SEED, self.collection.as_ref(), &bump]];
        let accounts = MintTo {
            mint,
            to,
            authority,
        };
        let context = CpiContext::new_with_signer(token_program.key(), accounts, signer_seeds);
        mint_to(context, amount)
    }
}

#[account]
#[derive(InitSpace)]
pub struct StakeAccount {
    pub user: Pubkey,
    pub asset: Pubkey,
    pub config: Pubkey,
    pub last_claimed: i64,
    pub bump: u8,
}

impl StakeAccount {
    pub fn rewards(&self, now: i64, rate: u64) -> Result<u64> {
        let elapsed = now
            .checked_sub(self.last_claimed)
            .and_then(|seconds| u64::try_from(seconds).ok())
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        elapsed
            .checked_mul(rate)
            .ok_or_else(|| error!(ErrorCode::ArithmeticOverflow))
    }
}
