use proc_qq::re_exports::ricq::version::ANDROID_WATCH;
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
    let modules = module::get_module();

    init_tracing_subscriber();
    let client = ClientBuilder::new()
        .authentication(Authentication::QRCode)
        .show_rq(ShowQR::OpenBySystem)
        .device(DeviceSource::JsonFile("device.json".to_owned()))
        .version(&ANDROID_WATCH)
        .session_store(FileSessionStore::boxed("session.token"))
        .modules(modules)
        .result_handlers(vec![on_result {}.into()])
        .build()
        .await
        .unwrap();
    run_client(Arc::new(client)).await.unwrap();
}
