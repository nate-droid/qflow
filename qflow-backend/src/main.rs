use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router, http::StatusCode,
};
use k8s_openapi::api::{
    batch::v1::Job,
    core::v1::Pod,
};
use kube::{
    api::{Api, ListParams, LogParams},
    Client,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tower_http::cors::{Any, CorsLayer};

// --- Structs for the JSON response sent to the frontend ---
// These are designed to mimic the structure of a CRD so the frontend
// doesn't need to change how it processes data.

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SyntheticWorkflow {
    metadata: Metadata,
    spec: Spec,
    status: Status,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Metadata {
    name: String,
    namespace: String,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct Spec {
    tasks: Vec<Task>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Task {
    name: String,
    depends_on: Vec<String>,
    quantum: Option<serde_json::Value>,
    classical: Option<serde_json::Value>,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct Status {
    task_status: HashMap<String, String>,
}


// --- Application State and Main Setup ---

struct AppState {
    client: Client,
}

#[derive(Deserialize)]
pub struct FetchWorkflowParams {
    pub namespace: String,
}

#[tokio::main]
async fn main() {
    let client = Client::try_default().await.expect("Failed to create K8s client");
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);
    let app_state = Arc::new(AppState { client });

    let app = Router::new()
        .route("/api/workflows/{name}", get(fetch_workflow_from_jobs))
        .route("/api/workflows/{namespace}/{name}/tasks/{task_name}/results", get(fetch_task_results))
        .with_state(app_state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}


/// **REWORKED: Fetches all Kubernetes Jobs with a specific label to construct a workflow.**
///
/// This handler no longer looks for a `QuantumWorkflow` CRD. Instead, it synthesizes
/// the workflow by inspecting `Job` resources directly.
///
/// It makes the following assumptions about your Jobs:
/// - **Workflow Label:** `quantum.workflow/name: <workflow_name>` (used for lookup)
/// - **Task Name Label:** `quantum.workflow/task-name: <task_name>`
/// - **Dependencies Annotation:** `quantum.workflow/dependsOn: "task-a,task-b"`
/// - **Data Annotations:** `quantum.workflow/circuit`, `quantum.workflow/params`, etc.
async fn fetch_workflow_from_jobs(
    State(state): State<Arc<AppState>>,
    Path(workflow_name): Path<String>,
    Query(params): Query<FetchWorkflowParams>,
) -> Result<Json<SyntheticWorkflow>, StatusCode> {
    let jobs_api: Api<Job> = Api::namespaced(state.client.clone(), &params.namespace);
    let label_selector = format!("quantum.workflow/name={}", workflow_name);
    let lp = ListParams::default().labels(&label_selector);

    let all_jobs = jobs_api.list(&ListParams::default()).await.map_err(|e| {
        eprintln!("Error listing jobs in namespace '{}': {}", params.namespace, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;


    let workflow_jobs: Vec<Job> = all_jobs.items.into_iter()
        .filter(|job| {
            job.metadata.name.as_ref().map_or(false, |n| n.starts_with(&workflow_name))
        })
        .collect();

    let job_list = jobs_api.list(&lp).await.map_err(|e| {
        eprintln!("Error listing jobs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if workflow_jobs.is_empty() {
        eprintln!("No jobs found with prefix '{}' in namespace '{}'", workflow_name, params.namespace);
        return Err(StatusCode::NOT_FOUND);
    }

    // if job_list.items.is_empty() {
    //     eprintln!("No jobs found for workflow '{}' in namespace '{}' with label selector '{}'", workflow_name, params.namespace, label_selector);
    //     return Err(StatusCode::NOT_FOUND);
    // }

    let mut tasks = Vec::new();
    let mut task_status_map = HashMap::new();

    for job in workflow_jobs {
        let annotations = job.metadata.annotations.as_ref();
        let labels = job.metadata.labels.as_ref();

        // Prefer the explicit task-name label, but fall back to the job's full name.
        let task_name = labels.and_then(|l| l.get("quantum.workflow/task-name").cloned())
            .unwrap_or_else(|| job.metadata.name.clone().unwrap_or_default());

        // --- Extract Task Spec ---
        let depends_on = annotations
            .and_then(|a| a.get("quantum.workflow/dependsOn"))
            .map(|s| s.split(',').map(String::from).collect())
            .unwrap_or_default();

        let circuit = annotations.and_then(|a| a.get("quantum.workflow/circuit").cloned());
        let task_params = annotations.and_then(|a| a.get("quantum.workflow/params").cloned());

        let (quantum, classical) = if circuit.is_some() {
            let q_params: serde_json::Value = serde_json::from_str(&task_params.unwrap_or_else(|| "{}".to_string())).unwrap_or(serde_json::Value::Null);
            (Some(serde_json::json!({
                "circuit": circuit,
                "params": q_params,
            })), None)
        } else {
            (None, Some(serde_json::json!({
                "command": "See Job Spec",
                 "params": task_params,
            })))
        };

        tasks.push(Task {
            name: task_name.clone(),
            depends_on,
            quantum,
            classical,
        });

        // --- Determine Task Status ---
        let status_str = match job.status {
            Some(s) if s.succeeded.map_or(false, |c| c > 0) => "Succeeded".to_string(),
            Some(s) if s.clone().failed.map_or(false, |c| c > 0) => "Failed".to_string(),
            Some(s) if s.clone().active.map_or(false, |c| c > 0) => "Running".to_string(),
            _ => "Pending".to_string(),
        };
        task_status_map.insert(task_name, status_str);
    }

    let workflow = SyntheticWorkflow {
        metadata: Metadata {
            name: workflow_name,
            namespace: params.namespace,
        },
        spec: Spec { tasks },
        status: Status { task_status: task_status_map },
    };


    // for job in job_list.items {
    //     let annotations = job.metadata.annotations.as_ref();
    //     let labels = job.metadata.labels.as_ref();
    //
    //     let task_name = labels.and_then(|l| l.get("quantum.workflow/task-name").cloned())
    //         .unwrap_or_else(|| job.metadata.name.clone().unwrap_or_default());
    //
    //     // --- Extract Task Spec ---
    //     let depends_on = annotations
    //         .and_then(|a| a.get("quantum.workflow/dependsOn"))
    //         .map(|s| s.split(',').map(String::from).collect())
    //         .unwrap_or_default();
    //
    //     // A simple way to get task details is from annotations.
    //     // A better long-term solution is mounting a ConfigMap to the Job.
    //     let circuit = annotations.and_then(|a| a.get("quantum.workflow/circuit").cloned());
    //     let task_params = annotations.and_then(|a| a.get("quantum.workflow/params").cloned());
    //
    //     let (quantum, classical) = if circuit.is_some() {
    //         let q_params: serde_json::Value = serde_json::from_str(&task_params.unwrap_or_else(|| "{}".to_string())).unwrap_or(serde_json::Value::Null);
    //         (Some(serde_json::json!({
    //             "circuit": circuit,
    //             "params": q_params,
    //         })), None)
    //     } else {
    //         (None, Some(serde_json::json!({
    //             "command": "See Job Spec",
    //              "params": task_params,
    //         })))
    //     };
    //
    //     tasks.push(Task {
    //         name: task_name.clone(),
    //         depends_on,
    //         quantum,
    //         classical,
    //     });
    //
    //     // --- Determine Task Status ---
    //     let status_str = match job.status {
    //         Some(s) if s.succeeded.map_or(false, |c| c > 0) => "Succeeded".to_string(),
    //         Some(s) if s.clone().failed.map_or(false, |c| c > 0) => "Failed".to_string(),
    //         Some(s) if s.clone().active.map_or(false, |c| c > 0) => "Running".to_string(),
    //         _ => "Pending".to_string(),
    //     };
    //     task_status_map.insert(task_name, status_str);
    // }
    //
    // let workflow = SyntheticWorkflow {
    //     metadata: Metadata {
    //         name: workflow_name,
    //         namespace: params.namespace,
    //     },
    //     spec: Spec { tasks },
    //     status: Status { task_status: task_status_map },
    // };

    Ok(Json(workflow))
}


/// Axum handler to fetch the results (logs) of a specific task pod.
/// This handler remains largely the same as it already works by finding pods via labels.
async fn fetch_task_results(
    State(state): State<Arc<AppState>>,
    Path((namespace, workflow_name, task_name)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let pods: Api<Pod> = Api::namespaced(state.client.clone(), &namespace);
    // The job-name label is standard for pods created by a Job.
    let pod_label = format!("job-name={}", task_name);
    let lp = ListParams::default().labels(&pod_label);

    let pod_list = pods.list(&lp).await.map_err(|e| {
        eprintln!("Error listing pods: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if let Some(pod) = pod_list.items.into_iter().find(|p| p.status.as_ref().map_or(false, |s| s.phase == Some("Succeeded".to_string()))) {
        if let Some(pod_name) = &pod.metadata.name {
            let logs = pods.logs(pod_name, &LogParams::default()).await.map_err(|e| {
                eprintln!("Error fetching logs for pod '{}': {}", pod_name, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            match serde_json::from_str::<serde_json::Value>(&logs) {
                Ok(json_value) => Ok(Json(json_value)),
                Err(_) => Ok(Json(serde_json::json!({ "raw_logs": logs }))),
            }
        } else {
            Err(StatusCode::NOT_FOUND)
        }
    } else {
        eprintln!("No succeeded pod found with label '{}'", pod_label);
        Err(StatusCode::NOT_FOUND)
    }
}
