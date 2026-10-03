#![allow(dead_code)]

use anchor_lang::{
    prelude::{msg, AccountInfo, Clock, Pubkey},
    solana_program::{instruction::Instruction, program_pack::Pack, system_program},
    AccountDeserialize, InstructionData, ToAccountMetas,
};
use anchor_spl::{associated_token, token::spl_token};
use litesvm::{
    types::{FailedTransactionMetadata, TransactionMetadata},
    LiteSVM,
};
use mpl_core::{
    accounts::BaseAssetV1,
    fetch_asset_plugin, fetch_collection_plugin,
    instructions::{CreateCollectionV1Builder, CreateV1Builder},
    types::{
        Attribute, Attributes, FreezeDelegate, Plugin, PluginAuthority, PluginAuthorityPair,
        PluginType,
    },
};
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

pub const RATE: u64 = 1_000_000;
pub const BONUS: u64 = 1_000_000_000;

pub fn send(
    svm: &mut LiteSVM,
    payer: &Keypair,
    instructions: &[Instruction],
    signers: &[&Keypair],
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let mut all_signers = vec![payer];
    all_signers.extend_from_slice(signers);
    let message =
        Message::new_with_blockhash(instructions, Some(&payer.pubkey()), &svm.latest_blockhash());
    let transaction =
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), &all_signers).unwrap();
    svm.send_transaction(transaction)
}

pub struct Fixture {
    pub svm: LiteSVM,
    pub admin: Keypair,
    pub user: Keypair,
    pub collection: Pubkey,
    pub asset: Pubkey,
    pub config: Pubkey,
    pub stake_account: Pubkey,
    pub rewards_mint: Pubkey,
    pub user_rewards_ata: Pubkey,
}

impl Fixture {
    pub fn new() -> Self {
        Self::with_attributes(true)
    }

    pub fn with_attributes(has_attributes: bool) -> Self {
        let mut svm = LiteSVM::new();
        svm.add_program(
            metaplex::id(),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../target/deploy/metaplex.so"
            )),
        )
        .unwrap();
        svm.add_program(
            mpl_core::ID,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../target/deploy/mpl_core.so"
            )),
        )
        .unwrap();
        let admin = Keypair::new();
        let user = Keypair::new();
        let collection = Keypair::new();
        let asset = Keypair::new();
        svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();
        svm.airdrop(&user.pubkey(), 10_000_000_000).unwrap();
        let create_collection = CreateCollectionV1Builder::new()
            .collection(collection.pubkey())
            .payer(admin.pubkey())
            .update_authority(Some(admin.pubkey()))
            .name("Turbin3".into())
            .uri("https://example.com/collection.json".into())
            .plugins(if has_attributes {
                vec![PluginAuthorityPair {
                    plugin: Plugin::Attributes(Attributes {
                        attribute_list: vec![Attribute {
                            key: "course".into(),
                            value: "Turbin3".into(),
                        }],
                    }),
                    authority: None,
                }]
            } else {
                vec![]
            })
            .instruction();
        send(&mut svm, &admin, &[create_collection], &[&collection]).unwrap();
        let create_asset = CreateV1Builder::new()
            .asset(asset.pubkey())
            .collection(Some(collection.pubkey()))
            .authority(Some(admin.pubkey()))
            .payer(admin.pubkey())
            .owner(Some(user.pubkey()))
            .name("Staking NFT".into())
            .uri("https://example.com/nft.json".into())
            .instruction();
        send(&mut svm, &admin, &[create_asset], &[&asset]).unwrap();
        let config = Pubkey::find_program_address(
            &[metaplex::CONFIG_SEED, collection.pubkey().as_ref()],
            &metaplex::id(),
        )
        .0;
        let rewards_mint = Pubkey::find_program_address(
            &[metaplex::REWARDS_SEED, config.as_ref()],
            &metaplex::id(),
        )
        .0;
        let stake_account = Pubkey::find_program_address(
            &[
                metaplex::STAKE_SEED,
                config.as_ref(),
                asset.pubkey().as_ref(),
            ],
            &metaplex::id(),
        )
        .0;
        let user_rewards_ata =
            associated_token::get_associated_token_address(&user.pubkey(), &rewards_mint);
        Self {
            svm,
            admin,
            user,
            collection: collection.pubkey(),
            asset: asset.pubkey(),
            config,
            stake_account,
            rewards_mint,
            user_rewards_ata,
        }
    }

    pub fn initialize_ix(&self, admin: Pubkey) -> Instruction {
        Instruction {
            program_id: metaplex::id(),
            accounts: metaplex::accounts::Initialize {
                admin,
                collection: self.collection,
                config: self.config,
                rewards_mint: self.rewards_mint,
                core_program: mpl_core::ID,
                token_program: spl_token::ID,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
            data: metaplex::instruction::Initialize {
                rewards_per_second: RATE,
                burn_bonus: BONUS,
            }
            .data(),
        }
    }

    pub fn initialize(&mut self) -> TransactionMetadata {
        let ix = self.initialize_ix(self.admin.pubkey());
        let tx = send(&mut self.svm, &self.admin, &[ix], &[]).unwrap();
        assert_eq!(self.total_staked(), 0);
        tx
    }

    pub fn stake_ix(&self, user: Pubkey) -> Instruction {
        Instruction {
            program_id: metaplex::id(),
            accounts: metaplex::accounts::Stake {
                user,
                config: self.config,
                asset: self.asset,
                collection: self.collection,
                stake_account: self.stake_account,
                core_program: mpl_core::ID,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
            data: metaplex::instruction::Stake {}.data(),
        }
    }

    pub fn stake(&mut self) -> TransactionMetadata {
        let ix = self.stake_ix(self.user.pubkey());
        send(&mut self.svm, &self.user, &[ix], &[]).unwrap()
    }

    pub fn claim_ix(&self, user: Pubkey) -> Instruction {
        Instruction {
            program_id: metaplex::id(),
            accounts: metaplex::accounts::ClaimRewards {
                user,
                config: self.config,
                asset: self.asset,
                collection: self.collection,
                stake_account: self.stake_account,
                rewards_mint: self.rewards_mint,
                user_rewards_ata: associated_token::get_associated_token_address(
                    &user,
                    &self.rewards_mint,
                ),
                token_program: spl_token::ID,
                associated_token_program: associated_token::ID,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
            data: metaplex::instruction::ClaimRewards {}.data(),
        }
    }

    pub fn claim(&mut self) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        let ix = self.claim_ix(self.user.pubkey());
        send(&mut self.svm, &self.user, &[ix], &[])
    }

    pub fn unstake_ix(&self) -> Instruction {
        Instruction {
            program_id: metaplex::id(),
            accounts: metaplex::accounts::Unstake {
                user: self.user.pubkey(),
                config: self.config,
                asset: self.asset,
                collection: self.collection,
                stake_account: self.stake_account,
                rewards_mint: self.rewards_mint,
                user_rewards_ata: self.user_rewards_ata,
                core_program: mpl_core::ID,
                token_program: spl_token::ID,
                associated_token_program: associated_token::ID,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
            data: metaplex::instruction::Unstake {}.data(),
        }
    }

    pub fn burn_ix(&self) -> Instruction {
        Instruction {
            program_id: metaplex::id(),
            accounts: metaplex::accounts::BurnStakedNft {
                user: self.user.pubkey(),
                config: self.config,
                asset: self.asset,
                collection: self.collection,
                stake_account: self.stake_account,
                rewards_mint: self.rewards_mint,
                user_rewards_ata: self.user_rewards_ata,
                core_program: mpl_core::ID,
                token_program: spl_token::ID,
                associated_token_program: associated_token::ID,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
            data: metaplex::instruction::BurnStakedNft {}.data(),
        }
    }

    pub fn advance(&mut self, seconds: i64) {
        let mut clock = self.svm.get_sysvar::<Clock>();
        clock.unix_timestamp += seconds;
        self.svm.set_sysvar(&clock);
        self.svm.expire_blockhash();
    }

    pub fn total_staked(&self) -> u64 {
        let mut account = self.svm.get_account(&self.collection).unwrap();
        let info = AccountInfo::new(
            &self.collection,
            false,
            false,
            &mut account.lamports,
            &mut account.data,
            &account.owner,
            false,
        );
        let (_, attributes, _) =
            fetch_collection_plugin::<Attributes>(&info, PluginType::Attributes).unwrap();
        attributes
            .attribute_list
            .iter()
            .find(|attr| attr.key == metaplex::TOTAL_STAKED)
            .unwrap()
            .value
            .parse()
            .unwrap()
    }

    pub fn assert_frozen(&self) {
        let mut account = self.svm.get_account(&self.asset).unwrap();
        assert_eq!(
            BaseAssetV1::from_bytes(&account.data).unwrap().owner,
            self.user.pubkey()
        );
        let info = AccountInfo::new(
            &self.asset,
            false,
            false,
            &mut account.lamports,
            &mut account.data,
            &account.owner,
            false,
        );
        let (authority, freeze, _) =
            fetch_asset_plugin::<FreezeDelegate>(&info, PluginType::FreezeDelegate).unwrap();
        assert!(freeze.frozen);
        assert_eq!(
            authority,
            PluginAuthority::Address {
                address: self.config
            }
        );
    }

    pub fn balance(&self) -> u64 {
        let account = self.svm.get_account(&self.user_rewards_ata).unwrap();
        spl_token::state::Account::unpack(&account.data)
            .unwrap()
            .amount
    }

    pub fn stake_data(&self) -> metaplex::StakeAccount {
        let account = self.svm.get_account(&self.stake_account).unwrap();
        metaplex::StakeAccount::try_deserialize(&mut account.data.as_slice()).unwrap()
    }
}

pub fn log_transaction(instruction: &str, tx: &TransactionMetadata) {
    msg!("{} transaction successful", instruction);
    msg!("CUs Consumed: {}", tx.compute_units_consumed);
    msg!("Tx Signature: {}", tx.signature);
}
