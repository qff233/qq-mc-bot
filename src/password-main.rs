use proc_qq::re_exports::ricq::version::ANDROID_PHONE;
use proc_qq::*;
use std::sync::Arc;

mod module;
use qq_bot::init_tracing_subscriber;

#[result]
pub async fn on_result(result: &EventResult) -> anyhow::Result<bool> {
    match result {
        EventResult::Process(info) => {
            tracing::info!("{} : {} : 处理了一条消息", info.module_id, info.handle_name);
        }
        EventResult::Exception(info, err) => {
            tracing::info!(
                "{} : {} : 遇到了错误 : {}",
                info.module_id,
                info.handle_name,
                err
            );
        }
    }
    Ok(false)
}

#[tokio::main]
async fn main() {
    init_tracing_subscriber();
    let client = ClientBuilder::new()
        .authentication(Authentication::UinPasswordMd5(123456, [0; 16]))
        .show_slider_pop_menu_if_possible()
        .device(DeviceSource::JsonFile("device.json".to_owned()))
        .version(&ANDROID_PHONE)
        .session_store(Box::new(FileSessionStore {
            path: "session.token".to_string(),
        }))
        .modules(module::get_module())
        .result_handlers(vec![on_result {}.into()])
        .build()
        .await
        .unwrap();
    run_client(Arc::new(client)).await.unwrap();
}
