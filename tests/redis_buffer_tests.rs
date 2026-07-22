use axum_api::services::redis_service::Redis;

#[tokio::test]
async fn test_take_latest_and_clear_returns_newest_lpush_value_once() {
    let Ok(redis_url) = std::env::var("REDIS_URL") else {
        eprintln!("REDIS_URL not set; skipping Redis integration test");
        return;
    };
    let redis = Redis::connect(&redis_url)
        .await
        .expect("test Redis should connect");
    let key = format!("test:sensor:buffer:{}", std::process::id());

    redis
        .lpush(&key, "older", 60)
        .await
        .expect("older value should buffer");
    redis
        .lpush(&key, "newest", 60)
        .await
        .expect("newest value should buffer");

    assert_eq!(
        redis
            .take_latest_and_clear(&key)
            .await
            .expect("buffer read should succeed"),
        Some("newest".to_string())
    );
    assert_eq!(
        redis
            .take_latest_and_clear(&key)
            .await
            .expect("cleared buffer read should succeed"),
        None
    );
}
