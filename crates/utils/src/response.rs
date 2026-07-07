use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS)]
pub struct ApiResponse<T, E = T> {
    success: bool,
    data: Option<T>,
    error_data: Option<E>,
    message: Option<String>,
}

impl<T, E> ApiResponse<T, E> {
    /// Creates a successful response, with `data` and no message.
    pub fn success(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            message: None,
            error_data: None,
        }
    }

    /// Creates an error response, with `message` and no data.
    pub fn error(message: &str) -> Self {
        ApiResponse {
            success: false,
            data: None,
            message: Some(message.to_string()),
            error_data: None,
        }
    }
    /// Creates an error response, with no `data`, no `message`, but with arbitrary `error_data`.
    pub fn error_with_data(data: E) -> Self {
        ApiResponse {
            success: false,
            data: None,
            error_data: Some(data),
            message: None,
        }
    }

    /// Returns true if the response was successful.
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// Consumes the response and returns the data if present.
    pub fn into_data(self) -> Option<T> {
        self.data
    }

    /// Returns a reference to the error message if present.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_has_correct_fields() {
        let response: ApiResponse<i32, ()> = ApiResponse::success(42);
        assert!(response.is_success());
        assert_eq!(response.into_data(), Some(42));
    }

    #[test]
    fn error_has_message() {
        let response = ApiResponse::<()>::error("something went wrong");
        assert!(!response.is_success());
        assert_eq!(response.message(), Some("something went wrong"));
        assert!(response.into_data().is_none());
    }

    #[test]
    fn error_with_data_stores_error_data() {
        #[derive(Debug, PartialEq)]
        struct ErrPayload {
            code: u32,
        }

        let payload = ErrPayload { code: 404 };
        let response = ApiResponse::<(), ErrPayload>::error_with_data(payload);
        assert!(!response.is_success());
        assert!(response.message().is_none());
    }

    #[test]
    fn success_into_data_consumes_response() {
        let response: ApiResponse<&str, ()> = ApiResponse::success("hello");
        assert_eq!(response.into_data(), Some("hello"));
    }

    #[test]
    fn message_returns_none_for_success() {
        let response: ApiResponse<&str, ()> = ApiResponse::success("data");
        assert_eq!(response.message(), None);
    }

    #[test]
    fn message_returns_some_for_error() {
        let response = ApiResponse::<()>::error("failure");
        assert_eq!(response.message(), Some("failure"));
    }

    #[test]
    fn error_with_data_has_no_message_by_default() {
        let response = ApiResponse::<(), String>::error_with_data("bad request".to_string());
        assert!(response.message().is_none());
        assert!(!response.is_success());
    }
}
