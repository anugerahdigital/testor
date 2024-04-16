use anchor_lang::{
    prelude::*,
    solana_program::{keccak::Hash as KeccakHash, program::invoke_signed},
};
use ore::{instruction::mine, TREASURY_ADDRESS};

use crate::state::{Guild, Miner, Ore};

pub fn process_mine(ctx: Context<Mine>, args: MineArgs) -> Result<()> {
    let mine_ix = mine(
        ctx.accounts.miner.key(),
        ctx.accounts.bus.key(),
        KeccakHash::new_from_array(args.hash).into(),
        args.nonce,
    );

    let mine_accounts = [
        ctx.accounts.ore_program.to_account_info(),
        ctx.accounts.miner.to_account_info(),
        ctx.accounts.bus.to_account_info(),
        ctx.accounts.proof_account.to_account_info(),
        ctx.accounts.ore_treasury.to_account_info(),
        ctx.accounts.slot_hashes.to_account_info(),
    ];

    match invoke_signed(
        &mine_ix,
        &mine_accounts,
        &[&[
            Miner::PREFIX,
            ctx.accounts.guild.key().as_ref(),
            &ctx.accounts.miner.id.to_le_bytes(),
            &[ctx.bumps.miner],
        ]],
    ) {
        Ok(()) => Ok(()),
        Err(e) => {
            msg!("error mining: {}", e);
            Ok(())
        }
    }
}

#[derive(Accounts)]
pub struct Mine<'info> {
    #[account()]
    pub guild: Account<'info, Guild>,

    #[account(mut, seeds = [Miner::PREFIX, guild.key().as_ref(), &miner.id.to_le_bytes()], bump)]
    pub miner: Account<'info, Miner>,

    /// CHECK: done by cpi
    #[account(mut)]
    pub bus: UncheckedAccount<'info>,

    /// CHECK: done via cpi
    #[account(mut, seeds = [ore::PROOF, miner.key().as_ref()], seeds::program = ore::id(), bump)]
    pub proof_account: UncheckedAccount<'info>,

    /// CHECK: manual
    #[account(address = TREASURY_ADDRESS)]
    pub ore_treasury: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: todo
    pub slot_hashes: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    pub ore_program: Program<'info, Ore>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MineArgs {
    pub hash: [u8; 32],
    pub nonce: u64,
}
