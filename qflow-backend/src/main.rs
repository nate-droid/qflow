use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use axum::http::StatusCode;

use kube::{Api, Client};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use qflow_types::{QuantumWorkflowSpec, QuantumWorkflowStatus, QuantumWorkflow};

// The state of our application, containing the Kubernetes client
struct AppState {
    client: Client,
}

#[tokio::main]
async fn main() {
    // 1. Initialize Kubernetes client
    let client = Client::try_default()
        .await
        .expect("Failed to create Kubernetes client");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 2. Create an Arc for the application state
    let app_state = Arc::new(AppState { client });

    // 3. Build our application with a route
    let app = Router::new()
        .route("/api/workflows/{name}", get(fetch_workflow))
        .with_state(app_state)
        .layer(cors);;

    // 4. Run the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}

/// Axum handler to fetch a QuantumWorkflow
async fn fetch_workflow(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<QuantumWorkflow>, axum::http::StatusCode> {

    // Get a handle to the QuantumWorkflow API in the "default" namespace
    // Change "default" to the namespace your resources are in
    let api: Api<QuantumWorkflow> = Api::default_namespaced(state.client.clone());

    // Fetch the resource from Kubernetes
    match api.get(&name).await {
        Ok(workflow) => Ok(Json(workflow)),
        Err(e) => {
            eprintln!("Error fetching workflow '{}': {}", name, e);
            // You can map different kube::Error variants to different HTTP status codes
            Err(StatusCode::from_u16(404).unwrap())
        }
    }
}