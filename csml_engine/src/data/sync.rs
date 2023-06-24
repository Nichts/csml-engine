use csml_interpreter::data::CsmlBot;
use crate::data::{Database, EngineError};
use crate::data::models::BotOpt;
use crate::db_connectors;

impl BotOpt {
    pub fn search_bot(&self, db: &mut Database) -> Result<CsmlBot, EngineError> {
        match self {
            BotOpt::CsmlBot(csml_bot) => Ok(csml_bot.to_owned()),
            BotOpt::BotId {
                bot_id,
                apps_endpoint,
                multibot,
            } => {
                let bot_version = db_connectors::bot::get_last_bot_version(bot_id, db)?;

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
                let bot_version = db_connectors::bot::get_by_version_id(version_id, bot_id, db)?;

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
