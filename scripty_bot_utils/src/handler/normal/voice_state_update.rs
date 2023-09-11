use scripty_audio_handler::get_voice_channel_id;
use serenity::{
	all::{ChannelId, VoiceState},
	client::Context,
};

pub async fn voice_state_update(ctx: Context, _: Option<VoiceState>, new: VoiceState) {
	let Some(gid) = new.guild_id else {
		warn!("no guild id in voice_state_update");
		return;
	};

	if let Some(cid) = get_voice_channel_id(&ctx, gid).await {
		let own_user_id = ctx.cache.current_user().id;

		// GuildRef forces a block here to prevent hold over await
		{
			let guild = match gid.to_guild_cached(&ctx) {
				Some(g) => g,
				None => {
					warn!("guild id {} not found in cache", gid);
					return;
				}
			};

			// iterate through voice states in the guild
			// if there are any more than 1 in this channel, return
			// if there are 0, leave the channel
			for (_, vs) in guild.voice_states.iter() {
				if vs.channel_id == Some(cid) && vs.user_id != own_user_id {
					return;
				}
			}
		}

		// if we get here, we are the only one in the channel
		// so we should leave
		debug!(
			"leaving voice channel {} in guild {} (we're last user)",
			cid, gid
		);
		if let Err(e) = scripty_audio_handler::disconnect_from_vc(&ctx, gid).await {
			error!("error disconnecting from voice channel: {:?}", e);
		};
	} else {
		debug!("not in a voice channel in guild {}", gid);

		// check if the guild has active premium
		let Some(_) = scripty_premium::get_guild(gid.0).await else {
			// it does not, so we don't need to do anything
			return;
		};

		// does the guild have automod enabled?
		let db = scripty_db::get_db();
		let Some(resp) = (match sqlx::query!(
			"SELECT enabled, auto_join_voice, log_channel_id FROM automod_config WHERE guild_id = $1",
			gid.0.get() as i64
		)
		.fetch_optional(db)
		.await
		{
			Ok(res) => res,
			Err(e) => {
				error!("error fetching automod config: {:?}", e);
				return;
			}
		}) else {
			// automod is not set up, so we don't need to do anything
			debug!(
				"automod not set up in guild {}, not continuing with join",
				gid
			);
			return;
		};
		if !resp.enabled && !resp.auto_join_voice {
			// automod is not enabled, so we don't need to do anything
			debug!(
				"automod not enabled in guild {}, not continuing with join",
				gid
			);
			return;
		};

		let log_channel_id = ChannelId::new(resp.log_channel_id as u64);

		// now we need to check the voice channel the user is joining
		// discord doesn't give us the channel id, so we need to get it from the guild's voice states
		let vs = {
			let guild = match gid.to_guild_cached(&ctx) {
				Some(g) => g,
				None => {
					warn!("guild id {} not found in cache", gid);
					return;
				}
			};

			// fetch the user's voice state
			match guild.voice_states.get(&new.user_id) {
				Some(vs) => vs.clone(), // a relatively cheap clone, only one string internally
				None => {
					warn!("user id {} not found in guild voice states", new.user_id);
					return;
				}
			}
		};
		let Some(cid) = vs.channel_id else {
			warn!("user id {} not in a voice channel", new.user_id);
			return;
		};

		// join the channel
		debug!(
			"joining voice channel {} in guild {} as guild has auto join enabled",
			cid, gid
		);
		if let Err(e) = scripty_audio_handler::connect_to_vc(
			ctx.clone(),
			gid,
			cid,
			log_channel_id,
			None,
			false,
			false,
		)
		.await
		{
			error!("error joining voice channel: {:?}", e);
			// fire a message to the log channel
			let _ = log_channel_id
				.say(
					&ctx.http,
					format!(
						"Failed to join voice channel due to auto-join error: {}\n\
						You may want to report this in our support server.",
						e
					),
				)
				.await;
		}
	};
}
