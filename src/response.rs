use actix_web::{
    error::{self, BlockingError, ResponseError},
    HttpRequest, HttpResponse,
};
use diesel::result::{DatabaseErrorKind, Error as DieselError};
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

/// Semantic error types with proper HTTP status code mapping
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Unprocessable entity: {0}")]
    UnprocessableEntity(String),

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}

impl ApiError {
    pub fn not_found(msg: &str) -> Self {
        ApiError::NotFound(msg.to_string())
    }

    pub fn invalid_input(msg: &str) -> Self {
        ApiError::InvalidInput(msg.to_string())
    }

    pub fn conflict(msg: &str) -> Self {
        ApiError::Conflict(msg.to_string())
    }

    pub fn internal(msg: &str) -> Self {
        ApiError::InternalError(msg.to_string())
    }

    fn to_error_resp(&self) -> ErrorResp {
        ErrorResp {
            error: self.to_string(),
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;
        match self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::InternalError(_) | ApiError::DatabaseError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        HttpResponse::build(status_code).json(self.to_error_resp())
    }
}

// Convert Diesel errors to semantic API errors
impl From<DieselError> for ApiError {
    fn from(error: DieselError) -> Self {
        match error {
            DieselError::NotFound => ApiError::NotFound("Resource not found".to_string()),
            DieselError::DatabaseError(kind, info) => match kind {
                DatabaseErrorKind::UniqueViolation => {
                    ApiError::Conflict("Resource already exists".to_string())
                }
                DatabaseErrorKind::ForeignKeyViolation => {
                    ApiError::Conflict("Foreign key constraint violation".to_string())
                }
                _ => ApiError::DatabaseError(format!("Database error: {}", info.message())),
            },
            _ => ApiError::InternalError("An unexpected error occurred".to_string()),
        }
    }
}

impl From<BlockingError> for ApiError {
    fn from(error: BlockingError) -> Self {
        ApiError::InternalError(format!("Blocking error: {}", error))
    }
}

// Legacy ErrorResp for backward compatibility during migration
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

impl std::convert::From<BlockingError> for ErrorResp {
    fn from(error: BlockingError) -> Self {
        Self {
            error: error.to_string(),
        }
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
