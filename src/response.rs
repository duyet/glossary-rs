use actix_web::error::ResponseError;
use actix_web::{error, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use thiserror::Error;

#[derive(Debug, Deserialize, Serialize)]
pub struct ListResp<T> {
    pub results: Vec<T>,
    pub count: i32,
}

impl<T> Default for ListResp<T>
where
    T: std::clone::Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ListResp<T>
where
    T: std::clone::Clone,
{
    pub fn new() -> Self {
        Self {
            results: vec![],
            count: 0,
        }
    }

    pub fn from(results: &[T]) -> Self {
        Self {
            results: results.to_vec(),
            count: results.len() as i32,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    pub message: String,
}

impl Message {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

#[derive(Debug, Error, Deserialize, Serialize)]
pub struct ErrorResp {
    pub error: String,
}

impl ErrorResp {
    pub fn new(error: &str) -> Self {
        Self {
            error: error.to_string(),
        }
    }

    pub fn from(error: diesel::result::Error) -> Self {
        Self {
            error: error.to_string(),
        }
    }
}

impl Display for ErrorResp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl ResponseError for ErrorResp {
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::BAD_REQUEST
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        let status_code = self.status_code();
        let json_response = ErrorResp {
            error: self.error.clone(),
        };

        actix_web::HttpResponse::build(status_code).json(json_response)
    }
}

pub fn json_error_handler(err: error::JsonPayloadError, _req: &HttpRequest) -> error::Error {
    use actix_web::error::JsonPayloadError;

    let detail = ErrorResp::new(&err.to_string());
    let resp = match &err {
        JsonPayloadError::ContentType => HttpResponse::UnsupportedMediaType().json(detail),
        JsonPayloadError::Deserialize(json_err) if json_err.is_data() => {
            HttpResponse::UnprocessableEntity().json(detail)
        }
        _ => HttpResponse::BadRequest().json(detail),
    };
    error::InternalError::from_response(err, resp).into()
}
