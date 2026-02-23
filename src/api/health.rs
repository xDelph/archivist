use http::StatusCode;
use vercel_runtime::{Error, Request, Response, ResponseBody};

pub async fn handler(_req: Request) -> Result<Response<ResponseBody>, Error> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(ResponseBody::from(r#"{"ok":true}"#))?)
}
