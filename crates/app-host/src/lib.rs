use rpc::ScrollSgxClient;

pub async fn host_entrypoint() {
    println!("Hello, world!");

    let client = rpc::create_client("http://127.0.0.1:1234").unwrap();
    let res = client.hello().await.unwrap();
    println!("res = {:?}", res);
}
