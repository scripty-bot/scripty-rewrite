use poise::BoxFuture;

// TODO: do shit
async fn _post_command(_ctx: crate::Context<'_>) {
    // ctx.command().name
}

#[inline]
pub fn post_command(ctx: crate::Context<'_>) -> BoxFuture<()> {
    Box::pin(_post_command(ctx))
}
