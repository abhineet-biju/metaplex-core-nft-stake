mod common;

use {
    anchor_lang::prelude::AccountInfo,
    common::{log_transaction, send, Fixture, RATE},
    mpl_core::{
        fetch_asset_plugin,
        types::{FreezeDelegate, PluginType},
    },
};

#[test]
fn unstake_pays_remaining_rewards_and_allows_restaking() {
    let mut f = Fixture::new();
    f.initialize();
    f.stake();
    f.advance(10);
    f.claim().unwrap();
    f.advance(5);
    let ix = f.unstake_ix();
    let tx = send(&mut f.svm, &f.user, &[ix], &[]).unwrap();
    assert_eq!(f.balance(), 15 * RATE);
    assert_eq!(f.total_staked(), 0);
    assert!(f.svm.get_account(&f.stake_account).is_none());
    let mut account = f.svm.get_account(&f.asset).unwrap();
    let info = AccountInfo::new(
        &f.asset,
        false,
        false,
        &mut account.lamports,
        &mut account.data,
        &account.owner,
        false,
    );
    assert!(fetch_asset_plugin::<FreezeDelegate>(&info, PluginType::FreezeDelegate).is_err());
    assert!(
        fetch_asset_plugin::<mpl_core::types::BurnDelegate>(&info, PluginType::BurnDelegate)
            .is_err()
    );
    f.svm.expire_blockhash();
    f.stake();
    f.advance(3);
    f.claim().unwrap();
    assert_eq!(f.balance(), 18 * RATE);
    assert_eq!(f.total_staked(), 1);
    log_transaction("Unstake", &tx);
}
