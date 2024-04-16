use anchor_lang::prelude::*;

declare_id!("5qkXj9tRfFXDEbkyha3wrPfiWDyNB8KXWTcrMZSrUVmB");

pub mod instructions;
pub mod state;
pub use instructions::*;
pub use ore::{
    state::{Proof, Treasury},
    utils::AccountDeserialize as OreAccountDeserialize,
    BUS_ADDRESSES, ID as OreProgramId, TREASURY_ADDRESS,
};

#[program]
pub mod guild {
    use self::instructions::process_initialize;

    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        process_initialize(ctx)
    }

    pub fn create_miner(ctx: Context<CreateMiner>) -> Result<()> {
        process_create_miner(ctx)
    }

    pub fn mine(ctx: Context<Mine>, args: MineArgs) -> Result<()> {
        process_mine(ctx, args)
    }

    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        process_claim(ctx)
    }
}
