mod common;

use {
    common::{log_transaction, send, Fixture, RATE},
    mpl_core::instructions::TransferV1Builder,
    solana_signer::Signer,
};

#[test]
fn claim_keeps_nft_frozen_and_pays_only_new_rewards() {
    let mut f = Fixture::new();
    f.initialize();
    f.stake();
    f.assert_frozen();
    assert_eq!(f.total_staked(), 1);
    let started = f.stake_data().last_claimed;
    f.advance(10);
    let tx = f.claim().unwrap();
    assert_eq!(f.balance(), 10 * RATE);
    assert_eq!(f.stake_data().last_claimed, started + 10);
    f.assert_frozen();
    assert_eq!(f.total_staked(), 1);
    f.svm.expire_blockhash();
    let error = f.claim().unwrap_err();
    assert!(error.meta.logs.iter().any(|log| log.contains("NoRewards")));
    assert_eq!(f.balance(), 10 * RATE);
    f.advance(5);
    f.claim().unwrap();
    assert_eq!(f.balance(), 15 * RATE);
    f.assert_frozen();
    log_transaction("Claim rewards", &tx);
}

#[test]
fn another_user_cannot_claim_and_frozen_nft_cannot_transfer() {
    let mut f = Fixture::new();
    f.initialize();
    f.stake();
    f.advance(10);
    let ix = f.claim_ix(f.admin.pubkey());
    assert!(send(&mut f.svm, &f.admin, &[ix], &[]).is_err());
    let ix = TransferV1Builder::new()
        .asset(f.asset)
        .collection(Some(f.collection))
        .payer(f.user.pubkey())
        .authority(Some(f.user.pubkey()))
        .new_owner(f.admin.pubkey())
        .instruction();
    assert!(send(&mut f.svm, &f.user, &[ix], &[]).is_err());
    f.assert_frozen();
    assert_eq!(f.total_staked(), 1);
    f.claim().unwrap();
    assert_eq!(f.balance(), 10 * RATE);
}
