use anchor_lang::{
    prelude::*,
    solana_program::program::invoke_signed,
    system_program::{transfer, Transfer},
};

use crate::state::{Guild, Miner, Ore};

use ore::instruction::register;

pub fn process_create_miner(ctx: Context<CreateMiner>) -> Result<()> {
    // transfer the lamports before registering, so the ore program does not try to transfer lamports from the signer (the PDA)
    let transfer_accounts = Transfer {
        from: ctx.accounts.authority.to_account_info(),
        to: ctx.accounts.proof_account.to_account_info(),
    };

    transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            transfer_accounts,
        ),
        1559040,
    )?;

    // register miner with ore program
    let register_ix = register(ctx.accounts.miner.key());

    let register_accounts = [
        ctx.accounts.ore_program.to_account_info(),
        ctx.accounts.miner.to_account_info(),
        ctx.accounts.proof_account.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
    ];

    invoke_signed(
        &register_ix,
        &register_accounts,
        &[&[
            Miner::PREFIX,
            ctx.accounts.guild.key().as_ref(),
            &ctx.accounts.guild.miner_index.to_le_bytes(),
            &[ctx.bumps.miner],
        ]],
    )?;

    // set initial mining data
    ctx.accounts.miner.set_inner(Miner {
        id: ctx.accounts.guild.miner_index,
    });
    ctx.accounts.guild.miner_index += 1;

    Ok(())
}

#[derive(Accounts)]
pub struct CreateMiner<'info> {
    #[account(mut, has_one = authority)]
    pub guild: Account<'info, Guild>,

    #[account(init, payer = authority, space = Miner::SPACE, seeds = [Miner::PREFIX, guild.key().as_ref(), &guild.miner_index.to_le_bytes()], bump)]
    pub miner: Account<'info, Miner>,

    /// CHECK: done via cpi
    #[account(mut, seeds = [ore::PROOF, miner.key().as_ref()], seeds::program = ore_program.key(), bump)]
    pub proof_account: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,

    pub ore_program: Program<'info, Ore>,
}
