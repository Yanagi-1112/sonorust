use langrustang::{format_t, lang_t};
use serenity::all::{ComponentInteraction, ComponentInteractionDataKind, Context, EditMessage};
use sonorust_db::GuildDataMut;

use crate::{
    crate_extensions::{serenity::SerenityHttpExt, sonorust_setting::SettingJsonExt},
    errors::SonorustError,
    Handler,
};

pub async fn server(
    handler: &Handler,
    ctx: &Context,
    interaction: &ComponentInteraction,
) -> Result<(), SonorustError> {
    let lang = handler.setting_json.get_bot_lang();

    let guild_id = interaction
        .guild_id
        .ok_or_else(|| SonorustError::GuildIdIsNone)?;
    let inter_member = guild_id.member(&ctx.http, interaction.user.id).await?;

    let is_bot_owner = {
        let bot_owner_id = ctx.http.get_bot_owner_id().await;
        interaction.user.id == bot_owner_id
    };

    let is_admin = {
        #[allow(deprecated)]
        match inter_member.permissions(&ctx.cache) {
            Ok(permissons) => permissons.administrator(),
            Err(_) => false,
        }
    };

    let send_ephemeral_msg = |content: &str| {
        eq_uilibrium::create_response_msg!(
            interaction,
            &ctx.http,
            content = content,
            ephemeral = true
        )
    };

    // 管理者でもbotの所有者でもなければ
    if !is_admin && !is_bot_owner {
        send_ephemeral_msg(lang_t!("msg.only_admin", lang)).await?;
        return Ok(());
    }

    // 選択した値を取得
    let choice_value = match &interaction.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => values[0].as_str(),

        _ => {
            log::error!(lang_t!("log.fail_get_data"));
            send_ephemeral_msg(lang_t!("msg.failed.get", lang)).await?;
            return Ok(());
        }
    };
    // サーバーデータの更新
    let new_bool = {
        let mut guilddata_mut = GuildDataMut::from(guild_id).await?;

        // 選択した値によってサーバーデータを編集して、変化後の値を取得する
        let change_value = |ref_bool: &mut bool| {
            *ref_bool = !*ref_bool;
            *ref_bool
        };

        let new_bool = match choice_value {
            lang_t!("guild.is_dic_onlyadmin") => {
                change_value(&mut guilddata_mut.options.is_dic_onlyadmin)
            }
            lang_t!("guild.is_entrance_exit_log") => {
                change_value(&mut guilddata_mut.options.is_entrance_exit_log)
            }
            lang_t!("guild.is_entrance_exit_play") => {
                change_value(&mut guilddata_mut.options.is_entrance_exit_play)
            }
            lang_t!("guild.is_notice_attachment") => {
                change_value(&mut guilddata_mut.options.is_notice_attachment)
            }
            lang_t!("guild.is_if_long_fastread") => {
                change_value(&mut guilddata_mut.options.is_if_long_fastread)
            }

            _ => {
                log::error!("{}", lang_t!("log.not_implemented_customid"));
                send_ephemeral_msg(lang_t!("msg.failed.get", lang)).await?;
                return Ok(());
            }
        };

        guilddata_mut.update().await?;
        new_bool
    };

    let new_bool_value = match new_bool {
        true => "ON",
        false => "OFF",
    };

    // 元の メッセージと embed を取得して選択された値を変更
    let choice_value_title = match choice_value {
        lang_t!("guild.is_dic_onlyadmin") => lang_t!("guild.desc.is_dic_onlyadmin", lang),
        lang_t!("guild.is_entrance_exit_log") => lang_t!("guild.desc.is_entrance_exit_log", lang),
        lang_t!("guild.is_entrance_exit_play") => lang_t!("guild.desc.is_entrance_exit_play", lang),
        lang_t!("guild.is_notice_attachment") => lang_t!("guild.desc.is_notice_attachment", lang),
        lang_t!("guild.is_if_long_fastread") => lang_t!("guild.desc.is_if_long_fastread", lang),

        _ => {
            log::error!("{}", lang_t!("log.not_implemented_customid"));
            send_ephemeral_msg(lang_t!("msg.failed.get", lang)).await?;
            return Ok(());
        }
    };

    let mut interaction_msg = interaction
        .channel_id
        .message(&ctx.http, interaction.message.id)
        .await?;

    // embed の取得と書き換え
    let Some(mut embed) = interaction_msg.embeds.get(0).cloned() else {
        log::error!("{}", lang_t!("log.fail_get_data"));
        send_ephemeral_msg(lang_t!("msg.failed.get", lang)).await?;
        return Ok(());
    };

    let Some(field_value) = embed
        .fields
        .iter_mut()
        .filter(|i| i.name == choice_value_title)
        .map(|i| &mut i.value)
        .next()
    else {
        log::error!("{}", lang_t!("log.fail_get_data"));
        send_ephemeral_msg(lang_t!("msg.failed.get", lang)).await?;
        return Ok(());
    };

    *field_value = new_bool_value.to_string();

    // メッセージの編集
    let edit_message = EditMessage::new().embed(embed.into());
    let task_edit_message = interaction_msg.edit(&ctx.http, edit_message);

    // 返信用のメッセージを送信
    let task_create_response = eq_uilibrium::create_response_msg!(
        interaction,
        &ctx.http,
        content = format_t!("server.changed", lang, choice_value_title, new_bool_value),
        ephemeral = false,
    );

    let (result1, result2) = tokio::join!(task_edit_message, task_create_response);
    result1?;
    result2?;

    Ok(())
}
