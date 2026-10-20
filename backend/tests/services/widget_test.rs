use std::collections::HashMap;
use std::sync::Mutex;

use backend::common::DeviceLifecycleKind;
use backend::models::device::DeviceWidget;
use backend::models::error::{AppError, AppResult};
use backend::services::device::DeviceService;
use backend::services::store::{DeviceNotifier, HubNotifier, NoNotify, WidgetStore};

fn weather_widget(enabled: bool) -> DeviceWidget {
    DeviceWidget {
        widget_key: "weather".to_string(),
        enabled,
        config: serde_json::json!({"city": "Berlin", "latitude": 52.52, "longitude": 13.41}),
    }
}

/// In-memory widget store: one owner per token plus that token's widgets.
struct FakeWidgetStore {
    owners: Mutex<HashMap<String, i64>>,
    widgets: Mutex<HashMap<String, Vec<DeviceWidget>>>,
    fail_with: Mutex<Option<String>>,
    fail_replace: Mutex<Option<String>>,
}

impl FakeWidgetStore {
    fn new() -> Self {
        Self {
            owners: Mutex::new(HashMap::new()),
            widgets: Mutex::new(HashMap::new()),
            fail_with: Mutex::new(None),
            fail_replace: Mutex::new(None),
        }
    }

    fn with_device(self, token: &str, user_id: i64, widgets: Vec<DeviceWidget>) -> Self {
        self.owners.lock().unwrap().insert(token.to_string(), user_id);
        self.widgets.lock().unwrap().insert(token.to_string(), widgets);
        self
    }

    fn stored(&self, token: &str) -> Vec<DeviceWidget> {
        self.widgets.lock().unwrap().get(token).cloned().unwrap_or_default()
    }

    fn fail_next(&self, msg: &str) {
        *self.fail_with.lock().unwrap() = Some(msg.to_string());
    }
}

impl WidgetStore for FakeWidgetStore {
    async fn store_device_token_owner(&self, token: &str) -> AppResult<Option<i64>> {
        if let Some(msg) = self.fail_with.lock().unwrap().take() {
            return Err(AppError::bad_gateway(msg));
        }
        Ok(self.owners.lock().unwrap().get(token).copied())
    }

    async fn store_get_device_widgets(&self, token: &str) -> AppResult<Vec<DeviceWidget>> {
        Ok(self.stored(token))
    }

    async fn store_replace_device_widgets(
        &self,
        token: &str,
        widgets: &[DeviceWidget],
    ) -> AppResult<()> {
        if let Some(msg) = self.fail_replace.lock().unwrap().take() {
            return Err(AppError::bad_gateway(msg));
        }
        if let Some(msg) = self.fail_with.lock().unwrap().take() {
            return Err(AppError::bad_gateway(msg));
        }
        self.widgets.lock().unwrap().insert(token.to_string(), widgets.to_vec());
        Ok(())
    }
}

struct RecordingDeviceNotifier {
    calls: Mutex<Vec<String>>,
}

impl RecordingDeviceNotifier {
    fn new() -> Self {
        Self { calls: Mutex::new(Vec::new()) }
    }
}

impl DeviceNotifier for RecordingDeviceNotifier {
    async fn notify_revoked(&self, _device_token: &str) {}

    async fn notify_widgets_updated(&self, device_token: &str) {
        self.calls.lock().unwrap().push(device_token.to_string());
    }
}

struct RecordingHubNotifier {
    calls: Mutex<Vec<(i64, DeviceLifecycleKind, Option<String>)>>,
}

impl RecordingHubNotifier {
    fn new() -> Self {
        Self { calls: Mutex::new(Vec::new()) }
    }
}

impl HubNotifier for RecordingHubNotifier {
    async fn notify_hub(&self, user_id: i64, kind: DeviceLifecycleKind, device_token: Option<&str>) {
        self.calls.lock().unwrap().push((user_id, kind, device_token.map(str::to_string)));
    }
}

#[tokio::test]
async fn get_widgets_returns_owner_widgets() {
    let store = FakeWidgetStore::new().with_device("tok-1", 7, vec![weather_widget(true)]);
    let got = DeviceService::get_device_widgets_with(&store, 7, "tok-1").await.unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].widget_key, "weather");
    assert!(got[0].enabled);
    assert_eq!(got[0].config["city"], "Berlin");
}

#[tokio::test]
async fn get_widgets_hides_other_users_devices_as_404() {
    let store = FakeWidgetStore::new().with_device("tok-1", 7, vec![weather_widget(true)]);
    let err = DeviceService::get_device_widgets_with(&store, 99, "tok-1").await.unwrap_err();
    assert_eq!(err.status, 404);
}

#[tokio::test]
async fn get_widgets_unknown_token_is_404() {
    let store = FakeWidgetStore::new();
    let err = DeviceService::get_device_widgets_with(&store, 7, "nope").await.unwrap_err();
    assert_eq!(err.status, 404);
}

#[tokio::test]
async fn save_widgets_replaces_and_pushes_to_device_and_hub() {
    let store = FakeWidgetStore::new().with_device("tok-1", 7, vec![weather_widget(true)]);
    let device = RecordingDeviceNotifier::new();
    let hub = RecordingHubNotifier::new();

    let saved = DeviceService::save_device_widgets_with(
        &store,
        &device,
        &hub,
        7,
        "tok-1",
        vec![weather_widget(false)],
    )
    .await
    .unwrap();

    assert_eq!(saved.len(), 1);
    assert!(!saved[0].enabled);
    // Replace semantics: stored state matches the PUT body exactly.
    assert!(!store.stored("tok-1")[0].enabled);
    assert_eq!(device.calls.lock().unwrap().as_slice(), ["tok-1"]);
    assert_eq!(
        hub.calls.lock().unwrap().as_slice(),
        [(7, DeviceLifecycleKind::WidgetsUpdated, Some("tok-1".to_string()))]
    );
}

#[tokio::test]
async fn save_widgets_empty_list_clears_all() {
    let store = FakeWidgetStore::new().with_device("tok-1", 7, vec![weather_widget(true)]);
    let saved =
        DeviceService::save_device_widgets_with(&store, &NoNotify, &NoNotify, 7, "tok-1", vec![])
            .await
            .unwrap();
    assert!(saved.is_empty());
    assert!(store.stored("tok-1").is_empty());
}

#[tokio::test]
async fn save_widgets_for_other_user_is_404_and_pushes_nothing() {
    let store = FakeWidgetStore::new().with_device("tok-1", 7, vec![weather_widget(true)]);
    let device = RecordingDeviceNotifier::new();
    let hub = RecordingHubNotifier::new();

    let err = DeviceService::save_device_widgets_with(
        &store,
        &device,
        &hub,
        99,
        "tok-1",
        vec![weather_widget(false)],
    )
    .await
    .unwrap_err();

    assert_eq!(err.status, 404);
    // Untouched store, no pushes on rejection.
    assert!(store.stored("tok-1")[0].enabled);
    assert!(device.calls.lock().unwrap().is_empty());
    assert!(hub.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn get_own_widgets_uses_device_token_as_credential() {
    let store = FakeWidgetStore::new().with_device("tok-1", 7, vec![weather_widget(true)]);
    let got = DeviceService::get_own_widgets_with(&store, "tok-1").await.unwrap();
    assert_eq!(got.len(), 1);

    let err = DeviceService::get_own_widgets_with(&store, "unknown").await.unwrap_err();
    assert_eq!(err.status, 401);
}

#[tokio::test]
async fn widget_store_failures_propagate() {
    let store = FakeWidgetStore::new().with_device("tok-1", 7, vec![weather_widget(true)]);
    store.fail_next("db error");
    let err = DeviceService::get_device_widgets_with(&store, 7, "tok-1").await.unwrap_err();
    assert_eq!(err.status, 502);

    store.fail_next("db error");
    let err = DeviceService::get_own_widgets_with(&store, "tok-1").await.unwrap_err();
    assert_eq!(err.status, 502);

    store.fail_next("db error");
    let err = DeviceService::save_device_widgets_with(
        &store,
        &NoNotify,
        &NoNotify,
        7,
        "tok-1",
        vec![],
    )
    .await
    .unwrap_err();
    assert_eq!(err.status, 502);

    // Fail replace
    *store.fail_replace.lock().unwrap() = Some("replace error".to_string());
    let err = DeviceService::save_device_widgets_with(
        &store,
        &NoNotify,
        &NoNotify,
        7,
        "tok-1",
        vec![],
    )
    .await
    .unwrap_err();
    assert_eq!(err.status, 502);
}
