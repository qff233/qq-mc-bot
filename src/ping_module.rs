use proc_qq::re_exports::ricq::client::event::GroupMessageEvent;
use proc_qq::{
    event, module, MessageChainParseTrait, MessageContentTrait, 
    MessageEvent, MessageSendToSourceTrait, Module, LoginEvent,
};

#[event]
async fn login(event: &LoginEvent) -> anyhow::Result<bool> {
    tracing::info!("正在登录 : {}", event.uin);
    Ok(false)
}

#[event(bot_command = "/ping")]
async fn ping(event:&GroupMessageEvent) -> anyhow::Result<bool> {
    event
        .send_message_to_source("hi~".parse_message_chain())
        .await?;
    Ok(true)
}

pub fn module() -> Module {
    module!("ping", "ping", ping)
}
