use anchor_lang::prelude::*;

#[account]
pub struct Guild {
    pub miner_index: u64,
    pub authority: Pubkey,
}

impl Guild {
    pub const SPACE: usize = 8 + 8 + 32;
}

#[account]
pub struct Miner {
    pub id: u64,
}

impl Miner {
    pub const PREFIX: &'static [u8] = b"miner";
    pub const SPACE: usize = 8 + 8;
}

#[derive(Clone)]
pub struct Ore;

impl anchor_lang::Id for Ore {
    fn id() -> Pubkey {
        ore::id()
    }
}

#[error_code]
pub enum GuildError {
    #[msg("Invalid Hash")]
    InvalidHash,
}
