mod common;

use {
    common::{log_transaction, send, Fixture, BONUS, RATE},
    mpl_core::{
        accounts::BaseCollectionV1, instructions::RevokePluginAuthorityV1Builder, types::PluginType,
    },
    solana_signer::Signer,
};

#[test]
fn burn_pays_bonus_once_and_decrements_collection_counter() {
    let mut f = Fixture::new();
    f.initialize();
    f.stake();
    f.advance(10);
    f.claim().unwrap();
    f.advance(5);
    let ix = f.burn_ix();
    let tx = send(&mut f.svm, &f.user, &[ix], &[]).unwrap();
    assert_eq!(f.balance(), 15 * RATE + BONUS);
    assert_eq!(f.total_staked(), 0);
    assert!(f.svm.get_account(&f.stake_account).is_none());
    let asset = f.svm.get_account(&f.asset);
    assert!(
        asset.is_none()
            || asset.unwrap().data.first() == Some(&(mpl_core::types::Key::Uninitialized as u8))
    );
    let collection = f.svm.get_account(&f.collection).unwrap();
    assert_eq!(
        BaseCollectionV1::from_bytes(&collection.data)
            .unwrap()
            .current_size,
        0
    );
    f.svm.expire_blockhash();
    let ix = f.burn_ix();
    assert!(send(&mut f.svm, &f.user, &[ix], &[]).is_err());
    assert_eq!(f.balance(), 15 * RATE + BONUS);
    log_transaction("Burn staked NFT", &tx);
}

#[test]
fn burn_requires_delegate_and_failure_keeps_nft_staked() {
    let mut f = Fixture::new();
    f.initialize();
    f.stake();
    f.advance(10);
    let ix = RevokePluginAuthorityV1Builder::new()
        .asset(f.asset)
        .collection(Some(f.collection))
        .payer(f.user.pubkey())
        .authority(Some(f.user.pubkey()))
        .plugin_type(PluginType::BurnDelegate)
        .instruction();
    send(&mut f.svm, &f.user, &[ix], &[]).unwrap();
    let ix = f.burn_ix();
    let error = send(&mut f.svm, &f.user, &[ix], &[]).unwrap_err();
    assert!(error
        .meta
        .logs
        .iter()
        .any(|log| log.contains("InvalidBurnDelegate")));
    f.assert_frozen();
    assert_eq!(f.total_staked(), 1);
    assert!(f.svm.get_account(&f.user_rewards_ata).is_none());
    f.claim().unwrap();
    assert_eq!(f.balance(), 10 * RATE);
    let ix = f.unstake_ix();
    send(&mut f.svm, &f.user, &[ix], &[]).unwrap();
    assert_eq!(f.total_staked(), 0);
}
