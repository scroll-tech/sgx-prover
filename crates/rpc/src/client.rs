use jsonrpsee::http_client::HttpClient;

pub fn create_client(target: impl AsRef<str>) -> anyhow::Result<HttpClient> {
    Ok(HttpClient::builder().build(target)?)
}
