use proc_qq::MessageChainAppendTrait;
use proc_qq::{
    event, module, GroupMessageEvent, MessageChainParseTrait, MessageContentTrait,
    MessageSendToSourceTrait, Module,
};

pub fn module() -> Module {
    module!("video", "video", video)
}

#[event]
async fn video(event: &GroupMessageEvent) -> anyhow::Result<bool> {
    let content = event.message_content();
    match content.find("https://www.bilibili.com/video/") {
        Some(_) => {
            let content = &content[31..];
            let bv = match content.find("/") {
                Some(index) => &content[..index],
                None => content,
            };
            let result = reqwest::get(format!(
                "http://api.bilibili.com/x/web-interface/view?bvid={}",
                bv
            ))
            .await?;
            let json_result = json::parse(result.text().await.unwrap().as_str()).unwrap();
            let data = &json_result["data"];
            let title = &data["title"];
            let pic = data["pic"].to_string();

            let mut msg = format!("https://www.bilibili.com/video/{}\n", bv);
            msg += title.to_string().as_str();
            msg += "\n";

            let img = reqwest::get(pic).await?.bytes().await?;
            let img = event.upload_image_to_source(img).await?;
            event
                .send_message_to_source(msg.parse_message_chain().append(img))
                .await?;
        }
        None => return Ok(false),
    };
    Ok(true)
}
