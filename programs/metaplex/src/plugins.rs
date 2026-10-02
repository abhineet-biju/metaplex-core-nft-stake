use anchor_lang::prelude::*;
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_asset_plugin, fetch_collection_plugin,
    instructions::UpdateCollectionPluginV1CpiBuilder,
    types::{
        Attributes, FreezeDelegate, Key as CoreKey, Plugin, PluginAuthority, PluginType,
        UpdateAuthority,
    },
};

use crate::{error::ErrorCode, StakeConfig, CONFIG_SEED, TOTAL_STAKED};

pub fn validate_asset(asset: &AccountInfo, user: Pubkey, collection: Pubkey) -> Result<()> {
    let data = BaseAssetV1::from_bytes(&asset.try_borrow_data()?)
        .map_err(|_| error!(ErrorCode::InvalidCoreAccount))?;
    require!(data.key == CoreKey::AssetV1, ErrorCode::InvalidCoreAccount);
    require_keys_eq!(data.owner, user, ErrorCode::InvalidOwner);
    require!(
        data.update_authority == UpdateAuthority::Collection(collection),
        ErrorCode::InvalidCollection
    );
    Ok(())
}

pub fn validate_staked(asset: &AccountInfo, config: Pubkey) -> Result<()> {
    let (authority, freeze, _) =
        fetch_asset_plugin::<FreezeDelegate>(asset, PluginType::FreezeDelegate)
            .map_err(|_| error!(ErrorCode::NotStaked))?;
    require!(
        freeze.frozen && authority == PluginAuthority::Address { address: config },
        ErrorCode::NotStaked
    );
    Ok(())
}

pub fn collection_attributes(collection: &AccountInfo) -> Result<Option<Attributes>> {
    match fetch_collection_plugin::<Attributes>(collection, PluginType::Attributes) {
        Ok((_, attributes, _)) => Ok(Some(attributes)),
        Err(error)
            if error.to_string() == mpl_core::errors::MplCoreError::PluginNotFound.to_string() =>
        {
            Ok(None)
        }
        Err(_) => err!(ErrorCode::InvalidAttributes),
    }
}

pub fn update_total_staked<'info>(
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    config_account: &AccountInfo<'info>,
    config: &StakeConfig,
    system_program: &AccountInfo<'info>,
    core_program: &AccountInfo<'info>,
    increment: bool,
) -> Result<()> {
    let data = BaseCollectionV1::from_bytes(&collection.try_borrow_data()?)
        .map_err(|_| error!(ErrorCode::InvalidCoreAccount))?;
    require!(
        data.key == CoreKey::CollectionV1,
        ErrorCode::InvalidCoreAccount
    );
    let mut attributes = collection_attributes(collection)?.ok_or(ErrorCode::InvalidAttributes)?;
    let mut counters = attributes
        .attribute_list
        .iter_mut()
        .filter(|attr| attr.key == TOTAL_STAKED);
    let counter = counters.next().ok_or(ErrorCode::InvalidAttributes)?;
    require!(counters.next().is_none(), ErrorCode::InvalidAttributes);
    let count = counter
        .value
        .parse::<u64>()
        .map_err(|_| error!(ErrorCode::InvalidAttributes))?;
    let count = if increment {
        count.checked_add(1)
    } else {
        count.checked_sub(1)
    }
    .ok_or(ErrorCode::ArithmeticOverflow)?;
    counter.value = count.to_string();

    let bump = [config.bump];
    let signer_seeds: &[&[&[u8]]] = &[&[CONFIG_SEED, config.collection.as_ref(), &bump]];
    UpdateCollectionPluginV1CpiBuilder::new(core_program)
        .collection(collection)
        .payer(payer)
        .authority(Some(config_account))
        .system_program(system_program)
        .plugin(Plugin::Attributes(attributes))
        .invoke_signed(signer_seeds)?;
    Ok(())
}
