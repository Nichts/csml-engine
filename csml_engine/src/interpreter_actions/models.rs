#[derive(Debug, Clone)]
pub enum InterpreterReturn {
    Continue,
    End,
    SwitchBot(SwitchBot),
}

#[derive(Debug, Clone)]
pub struct SwitchBot {
    pub bot_id: String,
    pub version_id: Option<String>,
    pub flow: Option<String>,
    pub step: String,
}
