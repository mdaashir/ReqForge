use reqforge_core::collection::CollectionStorage;
use reqforge_core::environment::{Environment, EnvironmentStorage};
use reqforge_core::history::{HistoryEntry, HistoryStorage};
use reqforge_core::import::{
    BrunoImporter, CurlImporter, Importer, InsomniaImporter, PostmanImporter,
};
use reqforge_core::request::{Request as CoreRequest, Response as CoreResponse};
use reqforge_core::testing::{Assertion, AssertionType, TestRunner, TestStatus};
use reqforge_core::{Collection, HttpHandler, ProtocolHandler, Result as CoreResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::command;

mod keychain;
mod oauth;

/// Application state shared across Tauri commands
pub struct AppState {
    pub workspace_root: Mutex<Option<PathBuf>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            workspace_root: Mutex::new(None),
        }
    }

    pub fn storage(&self) -> CoreResult<CollectionStorage> {
        let guard = self.workspace_root.lock().unwrap();
        let root = guard
            .as_ref()
            .ok_or_else(|| reqforge_core::Error::config("Workspace not initialised"))?;
        Ok(CollectionStorage::new(root.clone()))
    }

    pub fn env_storage(&self) -> CoreResult<EnvironmentStorage> {
        let guard = self.workspace_root.lock().unwrap();
        let root = guard
            .as_ref()
            .ok_or_else(|| reqforge_core::Error::config("Workspace not initialised"))?;
        let env_dir = root.join("environments");
        EnvironmentStorage::new(env_dir).map_err(|e| reqforge_core::Error::other(e.to_string()))
    }
}

/// Frontend ping request payload
#[derive(Debug, Serialize, Deserialize)]
pub struct PingRequest {
    pub message: String,
}

/// Frontend ping response payload
#[derive(Debug, Serialize, Deserialize)]
pub struct PingResponse {
    pub message: String,
    pub timestamp: u64,
}

/// Initialise the workspace at the given directory path
#[command]
fn init_workspace(state: tauri::State<'_, AppState>, path: String) -> CoreResult<()> {
    let path_buf = PathBuf::from(path);
    *state.workspace_root.lock().unwrap() = Some(path_buf);
    Ok(())
}

/// Initialise a workspace and seed it with the bundled starter collections
/// if no collections exist yet. Returns the count of collections written.
#[command]
async fn bootstrap_workspace(state: tauri::State<'_, AppState>, path: String) -> CoreResult<usize> {
    let path_buf = PathBuf::from(path);
    *state.workspace_root.lock().unwrap() = Some(path_buf);

    let storage = state.storage()?;
    let count = reqforge_core::samples::seed_into(&storage).await?;
    Ok(count)
}

/// Test command: ping the Rust backend
#[command]
fn ping(request: PingRequest) -> PingResponse {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    PingResponse {
        message: format!("Pong: {}", request.message),
        timestamp: now,
    }
}

/// Get the application version
#[command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Get the application name
#[command]
fn get_app_name() -> String {
    env!("CARGO_PKG_NAME").to_string()
}

/// Send an HTTP request through the core engine
#[command]
async fn send_request(request: CoreRequest) -> CoreResult<CoreResponse> {
    let handler = HttpHandler::new();
    handler.send(request).await
}

/// Test assertion payload from the frontend (matches `AssertionConfig`)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TestAssertionPayload {
    StatusCode { expected: u16 },
    ResponseTime { max_ms: u64 },
    BodyContains { substring: String },
    BodyMatches { pattern: String },
    HeaderEquals { header: String, expected: String },
    HeaderContains { header: String, substring: String },
    JsonPath { path: String, expected: String },
    ContentType { expected: String },
}

impl From<TestAssertionPayload> for AssertionType {
    fn from(p: TestAssertionPayload) -> Self {
        match p {
            TestAssertionPayload::StatusCode { expected } => AssertionType::StatusCode { expected },
            TestAssertionPayload::ResponseTime { max_ms } => AssertionType::ResponseTime { max_ms },
            TestAssertionPayload::BodyContains { substring } => {
                AssertionType::BodyContains { substring }
            }
            TestAssertionPayload::BodyMatches { pattern } => AssertionType::BodyMatches { pattern },
            TestAssertionPayload::HeaderEquals { header, expected } => {
                AssertionType::HeaderEquals { header, expected }
            }
            TestAssertionPayload::HeaderContains { header, substring } => {
                AssertionType::HeaderContains { header, substring }
            }
            TestAssertionPayload::JsonPath { path, expected } => {
                AssertionType::JsonPath { path, expected }
            }
            TestAssertionPayload::ContentType { expected } => {
                AssertionType::ContentType { expected }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionInput {
    pub name: String,
    #[serde(flatten)]
    pub assertion: TestAssertionPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunResult {
    pub name: String,
    pub status: TestStatus,
    pub assertions: Vec<AssertionResultPayload>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResultPayload {
    pub passed: bool,
    pub message: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

/// Run a set of assertions against an already-captured response
#[command]
fn run_tests(
    response: CoreResponse,
    suite_name: String,
    assertions: Vec<AssertionInput>,
) -> CoreResult<TestRunResult> {
    let mut runner = TestRunner::new(&suite_name);
    for a in assertions {
        runner = runner.add(Assertion {
            name: a.name,
            assertion: a.assertion.into(),
        });
    }

    let result = runner.run(&response)?;

    Ok(TestRunResult {
        name: result.name,
        status: result.status,
        assertions: result
            .assertions
            .into_iter()
            .map(|a| AssertionResultPayload {
                passed: a.passed,
                message: a.message,
                expected: a.expected,
                actual: a.actual,
            })
            .collect(),
        duration_ms: result.duration_ms,
    })
}

// === Collection commands ===

#[command]
async fn save_collection(
    state: tauri::State<'_, AppState>,
    collection: Collection,
) -> CoreResult<()> {
    let storage = state.storage()?;
    storage.save(&collection).await
}

#[command]
async fn load_collection(state: tauri::State<'_, AppState>, id: String) -> CoreResult<Collection> {
    let storage = state.storage()?;
    storage.load(&id).await
}

#[command]
async fn list_collections(state: tauri::State<'_, AppState>) -> CoreResult<Vec<Collection>> {
    let storage = state.storage()?;
    storage.list_all().await
}

#[command]
async fn delete_collection(state: tauri::State<'_, AppState>, id: String) -> CoreResult<()> {
    let storage = state.storage()?;
    storage.delete(&id).await
}

// === Import commands ===

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportFormat {
    Postman,
    Curl,
    Insomnia,
    Bruno,
    Auto,
}

/// Import a collection from a string of content.
///
/// `format` may be `postman`, `curl`, `insomnia`, or `auto` (content sniff).
#[command]
async fn import_collection(
    state: tauri::State<'_, AppState>,
    content: String,
    format: ImportFormat,
    save: bool,
) -> CoreResult<Collection> {
    let collection = match format {
        ImportFormat::Postman => PostmanImporter.import(&content)?,
        ImportFormat::Curl => CurlImporter.import(&content)?,
        ImportFormat::Insomnia => InsomniaImporter.import(&content)?,
        ImportFormat::Bruno => BrunoImporter.import(&content)?,
        ImportFormat::Auto => reqforge_core::import::detect_importer(&content)
            .ok_or_else(|| reqforge_core::Error::other("Could not detect import format"))?
            .import(&content)?,
    };

    if save {
        let storage = state.storage()?;
        storage.save(&collection).await?;
    }

    Ok(collection)
}

/// Import environments bundled with an Insomnia export.
#[command]
fn import_environments(content: String) -> CoreResult<Vec<reqforge_core::Environment>> {
    InsomniaImporter
        .import_environments(&content)
        .map_err(Into::into)
}

// === History commands ===

/// Save a history entry to disk
#[command]
async fn record_history(state: tauri::State<'_, AppState>, entry: HistoryEntry) -> CoreResult<()> {
    let root = state.workspace_root.lock().unwrap().clone();
    let root = root.ok_or_else(|| reqforge_core::Error::config("Workspace not initialised"))?;
    let storage = HistoryStorage::new(root);
    storage.append(entry).await
}

/// List history entries (newest first)
#[command]
async fn list_history(
    state: tauri::State<'_, AppState>,
    limit: Option<usize>,
) -> CoreResult<Vec<HistoryEntry>> {
    let root = state.workspace_root.lock().unwrap().clone();
    let root = root.ok_or_else(|| reqforge_core::Error::config("Workspace not initialised"))?;
    let storage = HistoryStorage::new(root);
    storage.list(limit).await
}

/// Search history by URL / method / status
#[command]
async fn search_history(
    state: tauri::State<'_, AppState>,
    needle: String,
    limit: Option<usize>,
) -> CoreResult<Vec<HistoryEntry>> {
    let root = state.workspace_root.lock().unwrap().clone();
    let root = root.ok_or_else(|| reqforge_core::Error::config("Workspace not initialised"))?;
    let storage = HistoryStorage::new(root);
    storage.search(&needle, limit.unwrap_or(50)).await
}

/// Clear all history
#[command]
async fn clear_history(state: tauri::State<'_, AppState>) -> CoreResult<()> {
    let root = state.workspace_root.lock().unwrap().clone();
    let root = root.ok_or_else(|| reqforge_core::Error::config("Workspace not initialised"))?;
    let storage = HistoryStorage::new(root);
    storage.clear().await
}

/// Replay a request from history. Returns the original request so the
/// frontend can re-send it via the existing `send_request` command.
#[command]
async fn replay_history(state: tauri::State<'_, AppState>, id: String) -> CoreResult<CoreRequest> {
    let root = state.workspace_root.lock().unwrap().clone();
    let root = root.ok_or_else(|| reqforge_core::Error::config("Workspace not initialised"))?;
    let storage = HistoryStorage::new(root);

    let entry = storage
        .list(Some(1000))
        .await?
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| reqforge_core::Error::other(format!("History entry {} not found", id)))?;

    Ok(entry.request)
}

// === Environment commands ===

/// Save an environment to disk
#[command]
fn save_environment(state: tauri::State<'_, AppState>, env: Environment) -> CoreResult<()> {
    let storage = state.env_storage()?;
    storage
        .save(&env)
        .map_err(|e| reqforge_core::Error::other(e.to_string()))
}

/// Load an environment from disk
#[command]
fn load_environment(state: tauri::State<'_, AppState>, name: String) -> CoreResult<Environment> {
    let storage = state.env_storage()?;
    storage
        .load(&name)
        .map_err(|e| reqforge_core::Error::other(e.to_string()))
}

/// List all available environments
#[command]
fn list_environments(state: tauri::State<'_, AppState>) -> CoreResult<Vec<String>> {
    let storage = state.env_storage()?;
    storage
        .list()
        .map_err(|e| reqforge_core::Error::other(e.to_string()))
}

/// Delete an environment from disk
#[command]
fn delete_environment(state: tauri::State<'_, AppState>, name: String) -> CoreResult<()> {
    let storage = state.env_storage()?;
    storage
        .delete(&name)
        .map_err(|e| reqforge_core::Error::other(e.to_string()))
}

/// Start the OAuth 2.0 PKCE flow (browser popup + loopback listener).
#[command]
async fn start_oauth_flow(
    req: crate::oauth::OAuthFlowRequest,
) -> CoreResult<crate::oauth::OAuthFlowResult> {
    crate::oauth::run_oauth_flow(req).await
}

/// Save a credential to the OS keychain.
#[command]
async fn keychain_set(workspace_root: String, account: String, value: String) -> CoreResult<()> {
    crate::keychain::keychain_set_with_index(workspace_root, account, value)
        .await
        .map_err(reqforge_core::Error::other)
}

/// Retrieve a credential from the OS keychain.
#[command]
async fn keychain_get(account: String) -> CoreResult<Option<String>> {
    crate::keychain::keychain_get(account)
        .await
        .map_err(reqforge_core::Error::other)
}

/// Delete a credential from the OS keychain.
#[command]
async fn keychain_delete(account: String) -> CoreResult<bool> {
    crate::keychain::keychain_delete(account)
        .await
        .map_err(reqforge_core::Error::other)
}

/// List all credentials stored in the OS keychain (workspace-scoped).
#[command]
async fn keychain_list(workspace_root: String) -> CoreResult<Vec<crate::keychain::CredentialMeta>> {
    crate::keychain::keychain_list(workspace_root)
        .await
        .map_err(reqforge_core::Error::other)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            init_workspace,
            bootstrap_workspace,
            ping,
            get_app_version,
            get_app_name,
            send_request,
            run_tests,
            save_collection,
            load_collection,
            list_collections,
            delete_collection,
            import_collection,
            record_history,
            list_history,
            search_history,
            clear_history,
            save_environment,
            load_environment,
            list_environments,
            delete_environment,
            import_environments,
            replay_history,
            start_oauth_flow,
            keychain_set,
            keychain_get,
            keychain_delete,
            keychain_list,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
