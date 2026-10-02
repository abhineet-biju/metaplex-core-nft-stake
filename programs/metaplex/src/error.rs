use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid reward rate or burn bonus")]
    InvalidRewards,
    #[msg("Invalid Core asset or collection")]
    InvalidCoreAccount,
    #[msg("Only the collection update authority can initialize staking")]
    InvalidAuthority,
    #[msg("The NFT does not belong to this collection")]
    InvalidCollection,
    #[msg("The signer does not own this NFT")]
    InvalidOwner,
    #[msg("The NFT is not frozen by this staking program")]
    NotStaked,
    #[msg("The burn delegate is not the staking program")]
    InvalidBurnDelegate,
    #[msg("Invalid collection staking attributes")]
    InvalidAttributes,
    #[msg("Arithmetic overflow or invalid timestamp")]
    ArithmeticOverflow,
    #[msg("No rewards available to claim")]
    NoRewards,
}
