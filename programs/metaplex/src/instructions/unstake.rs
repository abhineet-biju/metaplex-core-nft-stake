use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use mpl_core::{
    instructions::{RemovePluginV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{FreezeDelegate, Plugin, PluginType},
};

use crate::{
    plugins::{update_total_staked, validate_asset, validate_staked},
    StakeAccount, StakeConfig, CONFIG_SEED, REWARDS_SEED, STAKE_SEED,
};

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(has_one = collection, seeds = [CONFIG_SEED, collection.key().as_ref()], bump = config.bump)]
    pub config: Account<'info, StakeConfig>,
    /// CHECK: Core ownership is constrained; the handler checks the NFT owner, collection and freeze delegate.
    #[account(mut, owner = mpl_core::ID)]
    pub asset: UncheckedAccount<'info>,
    /// CHECK: Bound to config; Core validates the collection during CPI.
    #[account(mut, owner = mpl_core::ID)]
    pub collection: UncheckedAccount<'info>,
    #[account(
        mut,
        close = user,
        has_one = user,
        has_one = asset,
        has_one = config,
        seeds = [STAKE_SEED, config.key().as_ref(), asset.key().as_ref()],
        bump = stake_account.bump,
    )]
    pub stake_account: Account<'info, StakeAccount>,
    #[account(
        mut,
        seeds = [REWARDS_SEED, config.key().as_ref()],
        bump = config.rewards_bump,
        mint::authority = config,
    )]
    pub rewards_mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = rewards_mint,
        associated_token::authority = user,
    )]
    pub user_rewards_ata: Account<'info, TokenAccount>,
    /// CHECK: The address and executable constraints select the Core program.
    #[account(address = mpl_core::ID, executable)]
    pub core_program: UncheckedAccount<'info>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> Unstake<'info> {
    pub fn handler(&mut self) -> Result<()> {
        validate_asset(&self.asset, self.user.key(), self.collection.key())?;
        validate_staked(&self.asset, self.config.key())?;
        let now = Clock::get()?.unix_timestamp;
        let amount = self
            .stake_account
            .rewards(now, self.config.rewards_per_second)?;
        let bump = [self.config.bump];
        let signer_seeds: &[&[&[u8]]] = &[&[CONFIG_SEED, self.config.collection.as_ref(), &bump]];
        UpdatePluginV1CpiBuilder::new(&self.core_program)
            .asset(&self.asset)
            .collection(Some(&self.collection))
            .payer(&self.user.to_account_info())
            .authority(Some(&self.config.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }))
            .invoke_signed(signer_seeds)?;
        for plugin_type in [PluginType::FreezeDelegate, PluginType::BurnDelegate] {
            RemovePluginV1CpiBuilder::new(&self.core_program)
                .asset(&self.asset)
                .collection(Some(&self.collection))
                .payer(&self.user.to_account_info())
                .authority(Some(&self.user.to_account_info()))
                .system_program(&self.system_program.to_account_info())
                .plugin_type(plugin_type)
                .invoke()?;
        }
        update_total_staked(
            &self.collection,
            &self.user.to_account_info(),
            &self.config.to_account_info(),
            &self.config,
            &self.system_program.to_account_info(),
            &self.core_program,
            false,
        )?;
        self.config.mint_rewards(
            self.config.to_account_info(),
            self.rewards_mint.to_account_info(),
            self.user_rewards_ata.to_account_info(),
            self.token_program.to_account_info(),
            amount,
        )?;
        Ok(())
    }
}
