use anchor_lang::prelude::*;

use crate::state::Guild;

pub fn process_initialize(ctx: Context<Initialize>) -> Result<()> {
    ctx.accounts.guild.set_inner(Guild {
        miner_index: 0,
        authority: ctx.accounts.authority.key(),
    });

    Ok(())
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = Guild::SPACE)]
    pub guild: Account<'info, Guild>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}
