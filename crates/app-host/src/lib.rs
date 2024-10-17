use rpc::ScrollSgxClient;

pub async fn host_entrypoint() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let client = rpc::create_client("http://127.0.0.1:1234").unwrap();
    let res = client.get_address().await.unwrap();
    log::info!("res = {:?}", res);
}
