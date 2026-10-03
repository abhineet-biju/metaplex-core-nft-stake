mod common;

use {
    anchor_lang::prelude::AccountInfo,
    common::{log_transaction, send, Fixture},
    mpl_core::{
        fetch_collection_plugin,
        types::{Attributes, PluginAuthority, PluginType},
    },
    solana_signer::Signer,
};

#[test]
fn only_collection_authority_can_initialize() {
    let mut f = Fixture::new();
    let ix = f.initialize_ix(f.user.pubkey());
    let error = send(&mut f.svm, &f.user, &[ix], &[]).unwrap_err();
    assert!(error
        .meta
        .logs
        .iter()
        .any(|log| log.contains("InvalidAuthority")));
    assert!(f.svm.get_account(&f.config).is_none());
    assert!(f.svm.get_account(&f.rewards_mint).is_none());
}

#[test]
fn initializes_missing_attributes_plugin_and_preserves_existing_attributes() {
    for existing in [true, false] {
        let mut f = Fixture::with_attributes(existing);
        let tx = f.initialize();
        let mut account = f.svm.get_account(&f.collection).unwrap();
        let info = AccountInfo::new(
            &f.collection,
            false,
            false,
            &mut account.lamports,
            &mut account.data,
            &account.owner,
            false,
        );
        let (authority, attributes, _) =
            fetch_collection_plugin::<Attributes>(&info, PluginType::Attributes).unwrap();
        assert_eq!(authority, PluginAuthority::Address { address: f.config });
        assert_eq!(
            attributes
                .attribute_list
                .iter()
                .any(|attr| attr.key == "course" && attr.value == "Turbin3"),
            existing
        );
        f.stake();
        assert_eq!(f.total_staked(), 1);
        if !existing {
            log_transaction("Initialize", &tx);
        }
    }
}
