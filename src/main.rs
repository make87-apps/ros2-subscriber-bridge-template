use make87::{get_subscriber, resolve_topic_name};
use make87_messages::text::PlainText;
use ros2_client::{Context, MessageTypeName, Name, NodeName, NodeOptions};
use ros2_interfaces_rolling::std_msgs;
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

fn sanitize_and_checksum(input: &str) -> String {
    let prefix = "ros2_";

    // Sanitize the input string
    let mut sanitized = String::with_capacity(input.len());
    for c in input.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            sanitized.push(c);
        } else {
            sanitized.push('_');
        }
    }

    // Compute checksum
    let mut sum: u64 = 0;
    for b in input.bytes() {
        sum = (sum * 31 + b as u64) % 1_000_000_007;
    }
    let checksum = sum.to_string();

    // Calculate maximum allowed length for the sanitized string
    const MAX_TOTAL_LENGTH: usize = 256;
    let prefix_length = prefix.len();
    let checksum_length = checksum.len();
    let max_sanitized_length = MAX_TOTAL_LENGTH - prefix_length - checksum_length;

    // Truncate sanitized string if necessary
    if sanitized.len() > max_sanitized_length {
        sanitized.truncate(max_sanitized_length);
    }

    // Construct the final string
    format!("{}{}{}", prefix, sanitized, checksum)
}

fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    make87::initialize();

    let context = Context::new()?;
    let node_id = format!("make87_{}", Uuid::new_v4().simple());

    let mut node = context.new_node(NodeName::new("/make87", &node_id)?, NodeOptions::new())?;

    let ros_topic_name = resolve_topic_name("OUTGOING_MESSAGE")
        .map(|name| sanitize_and_checksum(&name)) // Prefix and replace '.' with '_'
        .ok_or("Failed to resolve topic name OUTGOING_MESSAGE")?;
    let proxy_topic = node.create_topic(
        &Name::new("/", &ros_topic_name)?,
        MessageTypeName::new("std_msgs", "String"),
        &ros2_client::DEFAULT_PUBLISHER_QOS,
    )?;

    let proxy_publisher =
        Arc::new(node.create_publisher::<std_msgs::msg::String>(&proxy_topic, None)?);

    let make87_topic_name = resolve_topic_name("INCOMING_MESSAGE")
        .ok_or("Failed to resolve topic name INCOMING_MESSAGE")?;
    let proxy_subscriber = get_subscriber::<PlainText>(make87_topic_name)
        .ok_or("Failed to get subscriber for INCOMING_MESSAGE")?;

    // Move proxy_publisher into the closure
    proxy_subscriber.subscribe(move |msg| {
        let ros_msg = std_msgs::msg::String {
            data: msg.body.clone(),
        };
        if let Err(e) = proxy_publisher.publish(ros_msg) {
            eprintln!("Failed to publish: {:?}", e);
        }
        println!("Proxied: {:?}", msg);
    })?;

    make87::keep_running();

    Ok(())
}
