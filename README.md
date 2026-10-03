# Core NFT staking

Stake Metaplex Core NFTs to earn rewards. Claim while an NFT stays frozen, unstake it, or burn it for a one-time bonus. Collection Attributes track `total_staked`.

## Instructions

| Instruction | Who signs | What it does |
| --- | --- | --- |
| `initialize(rewards_per_second, burn_bonus)` | Collection update authority | Creates the staking config and reward mint, and initializes `total_staked`. |
| `stake()` | NFT owner | Freezes the NFT and starts earning rewards. |
| `claim_rewards()` | NFT owner | Pays earned rewards to the owner's ATA without unfreezing the NFT. |
| `unstake()` | NFT owner | Thaws the NFT, pays remaining rewards, and removes the delegates. |
| `burn_staked_nft()` | NFT owner | Burns through BurnDelegate and pays remaining rewards plus the bonus. |

Staking increments `total_staked`. Unstaking and burning decrement it and close the stake account.

Rewards use six decimals. `1_000_000` base units equal one token. Both initialization amounts use base units.

## Rules

- NFTs must belong to the configured collection and have no existing FreezeDelegate or BurnDelegate plugins when staking.
- Initialize each collection once. Existing attributes are preserved; `total_staked` must be absent.
- The config PDA controls reward minting and the plugins. Keep collection Attributes delegated to it.
- There is no minimum staking period. Unstaked NFTs can be staked again; burning is permanent.

## Build and test

Use Anchor 1.1.2 and Solana CLI 4.2.2. Run from this directory:

```sh
mkdir -p target/deploy
solana program dump -u devnet CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d target/deploy/mpl_core.so
anchor build --ignore-keys
cargo test
```

Rebuild after code changes. LiteSVM tests need no deployment or validator. Core SDK stack-size diagnostics refer to unused helpers.

## Tests

All 10 integration tests and the program ID test pass. Tests cover the five instructions, reward accounting, permissions, collection attributes, and rollback.
