use crate::data::models::BotOpt;
use crate::data::{AsyncDatabase, EngineError};
use crate::future::db_connectors;
use csml_interpreter::data::CsmlBot;

impl BotOpt {
    pub async fn search_bot_async(
        &self,
        db: &mut AsyncDatabase<'_>,
    ) -> Result<CsmlBot, EngineError> {
        match self {
            BotOpt::CsmlBot(csml_bot) => Ok(csml_bot.to_owned()),
            BotOpt::BotId {
                bot_id,
                apps_endpoint,
                multibot,
            } => {
                let bot_version = db_connectors::bot::get_last_bot_version(bot_id, db).await?;

                match bot_version {
                    Some(mut bot_version) => {
                        bot_version.bot.apps_endpoint = apps_endpoint.to_owned();
                        bot_version.bot.multibot = multibot.to_owned();
                        Ok(bot_version.bot)
                    }
                    None => Err(EngineError::Manager(format!(
                        "bot ({}) not found in db",
                        bot_id
                    ))),
                }
            }
            BotOpt::Id {
                version_id,
                bot_id,
                apps_endpoint,
                multibot,
            } => {
                let bot_version =
                    db_connectors::bot::get_by_version_id(version_id, bot_id, db).await?;

                match bot_version {
                    Some(mut bot_version) => {
                        bot_version.bot.apps_endpoint = apps_endpoint.to_owned();
                        bot_version.bot.multibot = multibot.to_owned();
                        Ok(bot_version.bot)
                    }
                    None => Err(EngineError::Manager(format!(
                        "bot version ({}) not found in db",
                        version_id
                    ))),
                }
            }
        }
    }
}
