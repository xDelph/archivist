use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    Client,
    config::{Credentials, Region},
    primitives::ByteStream,
};
use bytes::Bytes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct R2Config {
    account_id: String,
    access_key_id: String,
    secret_access_key: String,
    bucket: String,
    public_url: String,
    endpoint_url: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct R2Client {
    client: Client,
    bucket: String,
    public_url: String,
}

impl R2Config {
    pub(crate) fn from_options(
        account_id: Option<String>,
        access_key_id: Option<String>,
        secret_access_key: Option<String>,
        bucket: Option<String>,
        public_url: Option<String>,
        endpoint_url: Option<String>,
    ) -> Option<Self> {
        match (
            non_empty(account_id),
            non_empty(access_key_id),
            non_empty(secret_access_key),
            non_empty(bucket),
            non_empty(public_url),
        ) {
            (
                Some(account_id),
                Some(access_key_id),
                Some(secret_access_key),
                Some(bucket),
                Some(public_url),
            ) => Some(Self {
                account_id,
                access_key_id,
                secret_access_key,
                bucket,
                public_url,
                endpoint_url: non_empty(endpoint_url),
            }),
            _ => None,
        }
    }

    pub(crate) fn public_url(&self, key: &str) -> String {
        format!("{}/{}", self.public_url, key)
    }
}

impl R2Client {
    pub(crate) async fn from_config(config: &R2Config) -> Self {
        let endpoint = config
            .endpoint_url
            .clone()
            .unwrap_or_else(|| format!("https://{}.r2.cloudflarestorage.com", config.account_id));
        let credentials = Credentials::new(
            &config.access_key_id,
            &config.secret_access_key,
            None,
            None,
            "cloudflare-r2",
        );

        Self::from_parts(&config.bucket, &config.public_url, &endpoint, credentials).await
    }

    async fn from_parts(
        bucket: &str,
        public_url: &str,
        endpoint: &str,
        credentials: Credentials,
    ) -> Self {
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new("auto"))
            .credentials_provider(credentials)
            .endpoint_url(endpoint)
            .load()
            .await;
        let client = aws_sdk_s3::config::Builder::from(&sdk_config)
            .force_path_style(true)
            .build();

        Self {
            client: Client::from_conf(client),
            bucket: bucket.to_owned(),
            public_url: public_url.to_owned(),
        }
    }

    pub(crate) async fn upload(
        &self,
        key: &str,
        data: Bytes,
        content_type: &str,
    ) -> Result<String, aws_sdk_s3::Error> {
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

    pub(crate) fn public_url(&self, key: &str) -> String {
        format!("{}/{}", self.public_url, key)
    }
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then_some(trimmed.to_owned())
    })
}
