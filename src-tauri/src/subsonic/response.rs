//! Parsing of the `subsonic-response` JSON envelope.
//!
//! Convention: every ID coming from the API is a `String`, never a number.

use serde::Deserialize;

use super::error::SubsonicError;

#[derive(Debug, Deserialize)]
struct Envelope {
    #[serde(rename = "subsonic-response")]
    body: ResponseBody,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Failed,
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub code: i32,
    pub message: Option<String>,
}

/// Fields common to every Subsonic response. `type`, `serverVersion` and
/// `openSubsonic` are OpenSubsonic extensions, absent on older servers.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseBody {
    pub status: Status,
    pub version: String,
    #[serde(rename = "type")]
    pub server_type: Option<String>,
    pub server_version: Option<String>,
    pub open_subsonic: Option<bool>,
    pub error: Option<ApiError>,
}

/// Parses a raw JSON body. A `failed` status becomes `SubsonicError::Api`.
pub fn parse_response(json: &str) -> Result<ResponseBody, SubsonicError> {
    let envelope: Envelope = serde_json::from_str(json)
        .map_err(|e| SubsonicError::InvalidResponse(e.to_string()))?;
    let body = envelope.body;

    match body.status {
        Status::Ok => Ok(body),
        Status::Failed => {
            let (code, message) = match body.error {
                Some(err) => (err.code, err.message.unwrap_or_default()),
                None => (0, String::from("unknown error")),
            };
            Err(SubsonicError::Api { code, message })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_navidrome_success() {
        let json = r#"{"subsonic-response":{"status":"ok","version":"1.16.1",
            "type":"navidrome","serverVersion":"0.58.0 (abc1234)","openSubsonic":true}}"#;

        let body = parse_response(json).expect("should parse");
        assert_eq!(body.status, Status::Ok);
        assert_eq!(body.version, "1.16.1");
        assert_eq!(body.server_type.as_deref(), Some("navidrome"));
        assert_eq!(body.server_version.as_deref(), Some("0.58.0 (abc1234)"));
        assert_eq!(body.open_subsonic, Some(true));
    }

    #[test]
    fn parses_minimal_success_without_opensubsonic_fields() {
        let json = r#"{"subsonic-response":{"status":"ok","version":"1.16.1"}}"#;

        let body = parse_response(json).expect("should parse");
        assert!(body.server_type.is_none());
        assert!(body.open_subsonic.is_none());
    }

    #[test]
    fn parses_wrong_credentials_error() {
        let json = r#"{"subsonic-response":{"status":"failed","version":"1.16.1",
            "type":"navidrome","serverVersion":"0.58.0","openSubsonic":true,
            "error":{"code":40,"message":"Wrong username or password"}}}"#;

        match parse_response(json) {
            Err(SubsonicError::Api { code, message }) => {
                assert_eq!(code, 40);
                assert_eq!(message, "Wrong username or password");
            }
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    #[test]
    fn failed_status_without_error_object_is_still_an_error() {
        let json = r#"{"subsonic-response":{"status":"failed","version":"1.16.1"}}"#;
        assert!(matches!(
            parse_response(json),
            Err(SubsonicError::Api { code: 0, .. })
        ));
    }

    #[test]
    fn html_page_is_invalid_response() {
        let html = "<!doctype html><html><body>Not a Subsonic server</body></html>";
        assert!(matches!(
            parse_response(html),
            Err(SubsonicError::InvalidResponse(_))
        ));
    }

    #[test]
    fn json_without_envelope_is_invalid_response() {
        assert!(matches!(
            parse_response(r#"{"status":"ok"}"#),
            Err(SubsonicError::InvalidResponse(_))
        ));
    }
}
