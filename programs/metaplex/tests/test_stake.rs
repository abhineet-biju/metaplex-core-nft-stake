mod common;

use {
    anchor_lang::prelude::Pubkey,
    common::{log_transaction, send, Fixture, BONUS},
    mpl_core::instructions::CreateV1Builder,
    solana_keypair::Keypair,
    solana_signer::Signer,
};

#[test]
fn rejects_wrong_owner_and_duplicate_stake() {
    let mut f = Fixture::new();
    f.initialize();
    let ix = f.stake_ix(f.admin.pubkey());
    let error = send(&mut f.svm, &f.admin, &[ix], &[]).unwrap_err();
    assert!(error
        .meta
        .logs
        .iter()
        .any(|log| log.contains("InvalidOwner")));
    assert_eq!(f.total_staked(), 0);
    let tx = f.stake();
    f.svm.expire_blockhash();
    let ix = f.stake_ix(f.user.pubkey());
    assert!(send(&mut f.svm, &f.user, &[ix], &[]).is_err());
    assert_eq!(f.total_staked(), 1);
    log_transaction("Stake", &tx);
}

#[test]
fn rejects_nft_from_another_collection() {
    let mut f = Fixture::new();
    f.initialize();
    let asset = Keypair::new();
    let ix = CreateV1Builder::new()
        .asset(asset.pubkey())
        .payer(f.admin.pubkey())
        .owner(Some(f.user.pubkey()))
        .name("Unrelated NFT".into())
        .uri("https://example.com/other.json".into())
        .instruction();
    send(&mut f.svm, &f.admin, &[ix], &[&asset]).unwrap();
    let mut ix = f.stake_ix(f.user.pubkey());
    ix.accounts[2].pubkey = asset.pubkey();
    let stake_account = Pubkey::find_program_address(
        &[
            metaplex::STAKE_SEED,
            f.config.as_ref(),
            asset.pubkey().as_ref(),
        ],
        &metaplex::id(),
    )
    .0;
    ix.accounts[4].pubkey = stake_account;
    let error = send(&mut f.svm, &f.user, &[ix], &[]).unwrap_err();
    assert!(error
        .meta
        .logs
        .iter()
        .any(|log| log.contains("InvalidCollection")));
    assert!(f.svm.get_account(&stake_account).is_none());
    assert_eq!(f.total_staked(), 0);
}

#[test]
fn collection_counter_tracks_multiple_nfts() {
    let mut f = Fixture::new();
    f.initialize();
    f.stake();
    let first_asset = f.asset;
    let first_stake = f.stake_account;
    let asset = Keypair::new();
    let ix = CreateV1Builder::new()
        .asset(asset.pubkey())
        .collection(Some(f.collection))
        .authority(Some(f.admin.pubkey()))
        .payer(f.admin.pubkey())
        .owner(Some(f.user.pubkey()))
        .name("Second NFT".into())
        .uri("https://example.com/second.json".into())
        .instruction();
    send(&mut f.svm, &f.admin, &[ix], &[&asset]).unwrap();
    f.asset = asset.pubkey();
    f.stake_account = Pubkey::find_program_address(
        &[metaplex::STAKE_SEED, f.config.as_ref(), f.asset.as_ref()],
        &metaplex::id(),
    )
    .0;
    f.stake();
    assert_eq!(f.total_staked(), 2);
    let ix = f.burn_ix();
    send(&mut f.svm, &f.user, &[ix], &[]).unwrap();
    assert_eq!(f.total_staked(), 1);
    f.asset = first_asset;
    f.stake_account = first_stake;
    f.assert_frozen();
    let ix = f.unstake_ix();
    send(&mut f.svm, &f.user, &[ix], &[]).unwrap();
    assert_eq!(f.total_staked(), 0);
    assert_eq!(f.balance(), BONUS);
}
