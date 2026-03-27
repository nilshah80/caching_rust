//! Pub/Sub API Schemas
//!
//! Request and response types for Pub/Sub endpoints.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

// ========== Publish Schemas ==========

/// Request to publish a message to a channel
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PublishRequest {
    /// Channel to publish to
    #[schema(example = "notifications")]
    pub channel: String,

    /// Message to publish
    #[schema(example = "Hello, subscribers!")]
    pub message: String,
}

/// Response from publish operation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PublishResponse {
    /// Channel the message was published to
    #[schema(example = "notifications")]
    pub channel: String,

    /// Number of clients that received the message
    #[schema(example = 5)]
    pub receivers: i64,
}

// ========== Channels Schemas ==========

/// Query parameters for listing channels
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct ChannelsQuery {
    /// Pattern to match channels (glob-style, e.g., "user:*")
    #[param(example = "user:*")]
    pub pattern: Option<String>,
}

/// Response with list of channels
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChannelsResponse {
    /// List of active channels
    #[schema(example = json!(["user:1", "user:2", "notifications"]))]
    pub channels: Vec<String>,
}

// ========== NumSub Schemas ==========

/// Request to get subscriber counts for channels
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NumSubRequest {
    /// Channels to get subscriber counts for
    #[schema(example = json!(["user:1", "notifications"]))]
    pub channels: Vec<String>,
}

/// Subscriber count for a channel
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NumSubItem {
    /// Channel name
    #[schema(example = "notifications")]
    pub channel: String,

    /// Number of subscribers
    #[schema(example = 10)]
    pub subscribers: i64,
}

/// Response with subscriber counts
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NumSubResponse {
    /// Subscriber counts per channel
    pub channels: Vec<NumSubItem>,
}

// ========== NumPat Schemas ==========

/// Response with pattern subscription count
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NumPatResponse {
    /// Total number of pattern subscriptions
    #[schema(example = 3)]
    pub patterns: i64,
}

// ========== Stats Schemas ==========

/// Pub/Sub statistics response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PubSubStatsResponse {
    /// Current number of active subscriptions
    #[schema(example = 25)]
    pub active_subscriptions: usize,

    /// Maximum allowed subscriptions
    #[schema(example = 100)]
    pub max_subscriptions: usize,

    /// Total subscriptions created (lifetime)
    #[schema(example = 150)]
    pub total_created: u64,

    /// Total messages received (lifetime)
    #[schema(example = 10000)]
    pub total_messages: u64,

    /// Total errors encountered
    #[schema(example = 5)]
    pub errors: u64,
}

// ========== WebSocket Schemas ==========

/// Query parameters for WebSocket subscription
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct SubscribeQuery {
    /// Comma-separated list of channels to subscribe to
    #[param(example = "user:123,notifications,events")]
    pub channels: String,
}

/// Query parameters for WebSocket pattern subscription
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct PSubscribeQuery {
    /// Comma-separated list of patterns to subscribe to
    #[param(example = "user:*,order:*")]
    pub patterns: String,
}

/// Message received from subscription
///
/// Messages are delivered as JSON over WebSocket. The `message` field contains
/// the payload as a UTF-8 string. For binary payloads that cannot be decoded
/// as UTF-8, the field will contain a base64-encoded string with a "base64:"
/// prefix (e.g., "base64:SGVsbG8gV29ybGQ=").
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PubSubMessage {
    /// Type of message ("message" for channel subscriptions, "pmessage" for pattern subscriptions)
    #[schema(example = "message")]
    pub r#type: String,

    /// Channel that the message was published to
    #[schema(example = "notifications")]
    pub channel: String,

    /// Pattern that matched (only present for pattern subscriptions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    /// Message payload (UTF-8 string, or "base64:..." for binary data)
    #[schema(example = "Hello!")]
    pub message: String,

    /// Timestamp when message was received by the server
    pub timestamp: DateTime<Utc>,
}

impl PubSubMessage {
    /// Create a new message for channel subscription
    pub fn new_message(channel: String, message: String) -> Self {
        Self {
            r#type: "message".to_string(),
            channel,
            pattern: None,
            message,
            timestamp: Utc::now(),
        }
    }

    /// Create a new message for pattern subscription
    pub fn new_pmessage(pattern: String, channel: String, message: String) -> Self {
        Self {
            r#type: "pmessage".to_string(),
            channel,
            pattern: Some(pattern),
            message,
            timestamp: Utc::now(),
        }
    }
}

/// WebSocket error message
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WebSocketError {
    /// Error type
    #[schema(example = "subscription_failed")]
    pub error: String,

    /// Error message
    #[schema(example = "Failed to subscribe to channel")]
    pub message: String,
}

/// WebSocket subscription confirmation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SubscriptionConfirmation {
    /// Type of confirmation
    #[schema(example = "subscribed")]
    pub r#type: String,

    /// Channel or pattern subscribed to
    #[schema(example = "notifications")]
    pub target: String,

    /// Current subscription count
    #[schema(example = 1)]
    pub count: i64,
}

impl SubscribeQuery {
    /// Parse channels from comma-separated string
    pub fn parse_channels(&self) -> Vec<String> {
        self.channels
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

impl PSubscribeQuery {
    /// Parse patterns from comma-separated string
    pub fn parse_patterns(&self) -> Vec<String> {
        self.patterns
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscribe_query_parse_channels() {
        let query = SubscribeQuery {
            channels: "ch1, ch2, ch3".to_string(),
        };
        let channels = query.parse_channels();
        assert_eq!(channels, vec!["ch1", "ch2", "ch3"]);
    }

    #[test]
    fn test_subscribe_query_parse_channels_empty() {
        let query = SubscribeQuery {
            channels: "".to_string(),
        };
        let channels = query.parse_channels();
        assert!(channels.is_empty());
    }

    #[test]
    fn test_psubscribe_query_parse_patterns() {
        let query = PSubscribeQuery {
            patterns: "user:*, order:*".to_string(),
        };
        let patterns = query.parse_patterns();
        assert_eq!(patterns, vec!["user:*", "order:*"]);
    }

    #[test]
    fn test_pubsub_message_new_message() {
        let msg = PubSubMessage::new_message("test".to_string(), "hello".to_string());
        assert_eq!(msg.r#type, "message");
        assert_eq!(msg.channel, "test");
        assert!(msg.pattern.is_none());
        assert_eq!(msg.message, "hello");
    }

    #[test]
    fn test_pubsub_message_new_pmessage() {
        let msg = PubSubMessage::new_pmessage(
            "user:*".to_string(),
            "user:123".to_string(),
            "hello".to_string(),
        );
        assert_eq!(msg.r#type, "pmessage");
        assert_eq!(msg.channel, "user:123");
        assert_eq!(msg.pattern, Some("user:*".to_string()));
        assert_eq!(msg.message, "hello");
    }
}
