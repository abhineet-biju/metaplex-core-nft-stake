use anchor_lang::prelude::*;
use mpl_core::{
    instructions::AddPluginV1CpiBuilder,
    types::{BurnDelegate, FreezeDelegate, Plugin, PluginAuthority},
};

use crate::{
    plugins::{update_total_staked, validate_asset},
    StakeAccount, StakeConfig, CONFIG_SEED, STAKE_SEED,
};

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(has_one = collection, seeds = [CONFIG_SEED, collection.key().as_ref()], bump = config.bump)]
    pub config: Account<'info, StakeConfig>,
    /// CHECK: Core ownership is constrained; the handler checks the NFT owner and collection.
    #[account(mut, owner = mpl_core::ID)]
    pub asset: UncheckedAccount<'info>,
    /// CHECK: Bound to config; Core validates the collection during CPI.
    #[account(mut, owner = mpl_core::ID)]
    pub collection: UncheckedAccount<'info>,
    #[account(
        init,
        payer = user,
        seeds = [STAKE_SEED, config.key().as_ref(), asset.key().as_ref()],
        bump,
        space = StakeAccount::DISCRIMINATOR.len() + StakeAccount::INIT_SPACE,
    )]
    pub stake_account: Account<'info, StakeAccount>,
    /// CHECK: The address and executable constraints select the Core program.
    #[account(address = mpl_core::ID, executable)]
    pub core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Stake<'info> {
    pub fn handler(&mut self, bumps: &StakeBumps) -> Result<()> {
        validate_asset(&self.asset, self.user.key(), self.collection.key())?;
        for plugin in [
            Plugin::BurnDelegate(BurnDelegate {}),
            Plugin::FreezeDelegate(FreezeDelegate { frozen: true }),
        ] {
            AddPluginV1CpiBuilder::new(&self.core_program)
                .asset(&self.asset)
                .collection(Some(&self.collection))
                .payer(&self.user.to_account_info())
                .authority(Some(&self.user.to_account_info()))
                .system_program(&self.system_program.to_account_info())
                .plugin(plugin)
                .init_authority(PluginAuthority::Address {
                    address: self.config.key(),
                })
                .invoke()?;
        }
        self.stake_account.set_inner(StakeAccount {
            user: self.user.key(),
            asset: self.asset.key(),
            config: self.config.key(),
            last_claimed: Clock::get()?.unix_timestamp,
            bump: bumps.stake_account,
        });
        update_total_staked(
            &self.collection,
            &self.user.to_account_info(),
            &self.config.to_account_info(),
            &self.config,
            &self.system_program.to_account_info(),
            &self.core_program,
            true,
        )
    }
}
