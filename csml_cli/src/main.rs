mod init_package;
mod interface;
mod run;

use clap::{Parser};
use clap_derive::{Parser, Subcommand};
use csml_engine::data::models::BotOpt;

use interface::{chat_menu::format_initial_payload, StartUI};
use run::load_info;

#[derive(Parser)]
#[command(about = "CSML CLI")]
pub struct Args {
    #[command(subcommand)]
    command: Option<Commands>
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Run Bot")]
    Run {
        #[arg(short, long, help = "start run with text")]
        text: Option<String>,
        #[arg(short, long, help = "Select starting flow")]
        flow: Option<String>,
        #[arg(short, long, help = "Select starting step")]
        step: Option<String>,
        #[arg(short, long, help = "Print debug information's")]
        debug: bool,
    },
    #[command(about = "Create a new CSML Bot in the selected directory")]
    Init,
}


fn main() {
    let matches = Args::parse();

    if let Some(command) = matches.command {
        match command {
            Commands::Init => interface::csml_ui(StartUI::Init).unwrap(),
            Commands::Run { text, flow, step, debug: _ } => {
                let request = format_initial_payload(flow.as_deref(), step.as_deref(), text.as_deref());

                match load_info(".") {
                    Ok(bot) => {
                        let bot_opt = Some(BotOpt::CsmlBot(bot));

                        let start = StartUI::Run { request, bot_opt };

                        interface::csml_ui(start).unwrap();
                    }
                    Err(..) => {
                        println!("path [./] is not a valid bot directory")
                    }
                }
            }
        }
    } else {
        interface::csml_ui(StartUI::Main).unwrap()
    }
}
