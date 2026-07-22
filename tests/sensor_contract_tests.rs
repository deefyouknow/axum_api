use axum_api::models::sensor::SensorLog;
use axum_api::schemas::sensor::{SensorInsertedResponse, SensorLatestResponse, SensorPayload};
use chrono::Utc;
use serde_json::json;

#[test]
fn test_sensor_payload_existing_json_fields_are_unchanged() {
    let payload: SensorPayload = serde_json::from_value(json!({
        "lux_panel_left": 202,
        "lux_panel_right": null,
        "lux_l": 10,
        "lux_ml": 20,
        "lux_mr": 30,
        "lux_r": 40,
        "voltage": 12.5,
        "current": 1.5,
        "power": 18.75
    }))
    .expect("existing POST payload must deserialize");

    let serialized = serde_json::to_value(payload).expect("payload must serialize");
    assert_eq!(serialized["lux_panel_left"], 202);
    assert_eq!(serialized["lux_l"], 10);
    assert_eq!(serialized["voltage"], 12.5);
    assert_eq!(serialized.as_object().expect("object").len(), 9);
}

#[test]
fn test_sensor_post_response_existing_json_fields_are_unchanged() {
    let response = SensorInsertedResponse {
        success: true,
        message: "Buffered in Redis".to_string(),
    };

    assert_eq!(
        serde_json::to_value(response).expect("response must serialize"),
        json!({"success": true, "message": "Buffered in Redis"})
    );
}

#[test]
fn test_sensor_latest_response_existing_json_fields_are_unchanged() {
    let response = SensorLatestResponse {
        reading: Some(SensorLog {
            id: 1,
            timestamp_slot: Utc::now(),
            lux_l: Some(10),
            lux_ml: Some(20),
            lux_mr: Some(30),
            lux_r: Some(40),
            lux_panel_left: Some(202),
            lux_panel_right: None,
            voltage: Some(12.5),
            current: Some(1.5),
            power: Some(18.75),
            is_online: true,
        }),
    };

    let serialized = serde_json::to_value(response).expect("response must serialize");
    let reading = serialized["reading"].as_object().expect("reading object");
    assert_eq!(serialized.as_object().expect("top-level object").len(), 1);
    assert_eq!(reading.len(), 12);
    assert_eq!(reading["lux_panel_left"], 202);
    assert_eq!(reading["is_online"], true);
}
