use std::env;

use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use bytes::Bytes;

pub struct R2Client {
    client: Client,
    bucket: String,
    public_url: String,
}

impl R2Client {
    pub async fn from_env() -> anyhow::Result<Self> {
        let account_id = env::var("CLOUDFLARED_R2_ACCOUNT_ID")?;
        let access_key = env::var("CLOUDFLARED_R2_ACCESS_KEY")?;
        let secret_key = env::var("CLOUDFLARED_R2_SECRET_KEY")?;
        let bucket = env::var("CLOUDFLARED_R2_BUCKET")?;
        let public_url = env::var("CLOUDFLARED_R2_PUBLIC_URL")?;

        let endpoint = format!("https://{account_id}.r2.cloudflarestorage.com");
        let creds = Credentials::new(&access_key, &secret_key, None, None, "r2");

        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new("auto"))
            .credentials_provider(creds)
            .endpoint_url(&endpoint)
            .load()
            .await;

        let client = aws_sdk_s3::config::Builder::from(&config)
            .force_path_style(true)
            .build();

        Ok(Self {
            client: Client::from_conf(client),
            bucket,
            public_url,
        })
    }

    pub async fn upload(
        &self,
        key: &str,
        data: Bytes,
        content_type: &str,
    ) -> anyhow::Result<String> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .body(ByteStream::from(data))
            .send()
            .await?;
        Ok(self.public_url(key))
    }

    pub fn public_url(&self, key: &str) -> String {
        format!("{}/{}", self.public_url.trim_end_matches('/'), key)
    }
}
