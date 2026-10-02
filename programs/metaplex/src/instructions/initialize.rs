use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};
use mpl_core::{
    accounts::BaseCollectionV1,
    instructions::{
        AddCollectionPluginV1CpiBuilder, ApproveCollectionPluginAuthorityV1CpiBuilder,
        UpdateCollectionPluginV1CpiBuilder,
    },
    types::{Attribute, Attributes, Key as CoreKey, Plugin, PluginAuthority, PluginType},
};

use crate::{
    error::ErrorCode, plugins::collection_attributes, StakeConfig, CONFIG_SEED, REWARDS_SEED,
    REWARD_DECIMALS, TOTAL_STAKED,
};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: Core ownership, discriminator and update authority are checked in the handler.
    #[account(mut, owner = mpl_core::ID)]
    pub collection: UncheckedAccount<'info>,
    #[account(
        init,
        payer = admin,
        seeds = [CONFIG_SEED, collection.key().as_ref()],
        bump,
        space = StakeConfig::DISCRIMINATOR.len() + StakeConfig::INIT_SPACE,
    )]
    pub config: Account<'info, StakeConfig>,
    #[account(
        init,
        payer = admin,
        seeds = [REWARDS_SEED, config.key().as_ref()],
        bump,
        mint::decimals = REWARD_DECIMALS,
        mint::authority = config,
    )]
    pub rewards_mint: Account<'info, Mint>,
    /// CHECK: The address and executable constraints select the Core program.
    #[account(address = mpl_core::ID, executable)]
    pub core_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn handler(
        &mut self,
        rewards_per_second: u64,
        burn_bonus: u64,
        bumps: &InitializeBumps,
    ) -> Result<()> {
        require!(
            rewards_per_second > 0 && burn_bonus > 0,
            ErrorCode::InvalidRewards
        );
        let collection = BaseCollectionV1::from_bytes(&self.collection.try_borrow_data()?)
            .map_err(|_| error!(ErrorCode::InvalidCoreAccount))?;
        require!(
            collection.key == CoreKey::CollectionV1,
            ErrorCode::InvalidCoreAccount
        );
        require_keys_eq!(
            collection.update_authority,
            self.admin.key(),
            ErrorCode::InvalidAuthority
        );

        self.config.set_inner(StakeConfig {
            collection: self.collection.key(),
            rewards_per_second,
            burn_bonus,
            bump: bumps.config,
            rewards_bump: bumps.rewards_mint,
        });
        let existing = collection_attributes(&self.collection)?;
        let mut attributes = existing.clone().unwrap_or(Attributes {
            attribute_list: vec![],
        });
        require!(
            !attributes
                .attribute_list
                .iter()
                .any(|attr| attr.key == TOTAL_STAKED),
            ErrorCode::InvalidAttributes
        );
        attributes.attribute_list.push(Attribute {
            key: TOTAL_STAKED.into(),
            value: "0".into(),
        });
        let authority = PluginAuthority::Address {
            address: self.config.key(),
        };

        if existing.is_some() {
            UpdateCollectionPluginV1CpiBuilder::new(&self.core_program)
                .collection(&self.collection)
                .payer(&self.admin.to_account_info())
                .authority(Some(&self.admin.to_account_info()))
                .system_program(&self.system_program.to_account_info())
                .plugin(Plugin::Attributes(attributes))
                .invoke()?;
            ApproveCollectionPluginAuthorityV1CpiBuilder::new(&self.core_program)
                .collection(&self.collection)
                .payer(&self.admin.to_account_info())
                .authority(Some(&self.admin.to_account_info()))
                .system_program(&self.system_program.to_account_info())
                .plugin_type(PluginType::Attributes)
                .new_authority(authority)
                .invoke()?;
        } else {
            AddCollectionPluginV1CpiBuilder::new(&self.core_program)
                .collection(&self.collection)
                .payer(&self.admin.to_account_info())
                .authority(Some(&self.admin.to_account_info()))
                .system_program(&self.system_program.to_account_info())
                .plugin(Plugin::Attributes(attributes))
                .init_authority(authority)
                .invoke()?;
        }
        Ok(())
    }
}
