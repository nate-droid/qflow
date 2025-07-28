use axum::extract::Multipart;
use axum::extract::Request;
use axum::response::Html;
use axum::routing::post;
use axum::{
    Form, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use k8s_openapi::api::{batch::v1::Job, core::v1::Pod};
use kube::{
    Client, CustomResource,
    api::{Api, ListParams, LogParams, PostParams},
};
use qflow_types::{QFlowTaskSpec, QuantumSVMWorkflowSpec, QuantumWorkflow, QuantumWorkflowSpec};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::{collections::HashMap, sync::Arc};
use std::io::Write;
use tempfile::NamedTempFile;
use tokio::io::AsyncReadExt;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

fn default_epochs() -> i32 {
    100
}

fn default_learning_rate() -> f64 {
    0.01
}

/// Defines the optimizer configuration for a QCBM task.
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct QcbmOptimizerSpec {
    pub name: String,
    #[serde(default = "default_epochs")]
    pub epochs: i32,
    #[serde(rename = "learningRate", default = "default_learning_rate")]
    pub learning_rate: f64,
    #[serde(rename = "initialParams", skip_serializing_if = "Option::is_none")]
    pub initial_params: Option<String>,
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quantum: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    classical: Option<serde_json::Value>,
    /// Added field to support QCBM tasks in the API response.
    #[serde(skip_serializing_if = "Option::is_none")]
    qcbm: Option<serde_json::Value>,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct Status {
    task_status: HashMap<String, String>,
}
// --- END: API-Specific Response Models ---

struct AppState {
    client: Client,
}

#[derive(Deserialize)]
pub struct FetchWorkflowParams {
    pub namespace: String,
}

#[tokio::main]
async fn main() {
    let client = Client::try_default()
        .await
        .expect("Failed to create K8s client");
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let app_state = Arc::new(AppState { client });

    let app = Router::new()
        .route("/api/workflows/{name}", get(fetch_workflow))
        .route(
            "/api/workflows/{namespace}/{name}/tasks/{task_name}/results",
            get(fetch_task_results),
        )
        .route("/api/workflows/{namespace}/new", post(submit_workflow))
        .route("/api/ml/svm", axum::routing::post(run_ml_svm))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request| {
                    let method = req.method().clone();
                    let uri = req.uri().clone();
                    println!("Received request: {} {}", method, uri);

                    println!("{:#?}", req);
                    tracing::debug_span!(
                        "request",
                        method = %method,
                        uri = %uri,
                        headers = ?req.headers(),
                    )
                })
                .on_failure(()),
        )
        // This endpoint remains hypothetical as it depends on a `qflowc` library
        .route("/api/workflows/{namespace}/{name}/qasm", post(submit_qasm))
        .with_state(app_state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn fetch_workflow(
    State(state): State<Arc<AppState>>,
    Path(workflow_name): Path<String>,
    Query(params): Query<FetchWorkflowParams>,
) -> Result<Json<SyntheticWorkflow>, StatusCode> {
    let wf_api: Api<QuantumWorkflow> = Api::namespaced(state.client.clone(), &params.namespace);
    let job_api: Api<Job> = Api::namespaced(state.client.clone(), &params.namespace);

    // 1. Fetch the source of truth: the QuantumWorkflow CR
    let workflow_cr = wf_api.get(&workflow_name).await.map_err(|e| {
        eprintln!("Error fetching QuantumWorkflow '{}': {}", workflow_name, e);
        StatusCode::NOT_FOUND
    })?;

    // 2. List all jobs in the namespace and create a map from task name to Job status
    let all_jobs = job_api.list(&ListParams::default()).await.map_err(|e| {
        eprintln!("Error listing jobs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut job_status_map: HashMap<String, String> = HashMap::new();
    for job in all_jobs.items {
        if let Some(owner_refs) = job.metadata.owner_references.as_ref() {
            if owner_refs.iter().any(|owner| owner.name == workflow_name) {
                // This job belongs to our workflow. Find its task name from the label.
                if let Some(labels) = job.metadata.labels {
                    if let Some(task_name) = labels.get("qflow.io/task-name") {
                        let status_str = match job.status {
                            Some(s) if s.succeeded.map_or(false, |c| c > 0) => "Succeeded",
                            Some(s) if s.failed.map_or(false, |c| c > 0) => "Failed",
                            Some(s) if s.active.map_or(false, |c| c > 0) => "Running",
                            _ => "Pending",
                        }
                        .to_string();
                        job_status_map.insert(task_name.clone(), status_str);
                    }
                }
            }
        }
    }

    // 3. Build the synthetic response
    let mut tasks = Vec::new();
    let mut task_status_map = HashMap::new();

    for task_from_cr in workflow_cr.spec.tasks {
        let task_name = task_from_cr.name.clone();

        let (quantum, classical, qcbm) = match task_from_cr.spec {
            QFlowTaskSpec::Classical { image } => {
                (None, Some(serde_json::json!({ "image": image })), None)
            }
            QFlowTaskSpec::Quantum {
                image,
                circuit,
                params,
            } => (
                Some(serde_json::json!({
                    "image": image,
                    "circuit": circuit,
                    "params": params,
                })),
                None,
                None,
            ),
            QFlowTaskSpec::Qcbm(spec) => (
                None,
                None,
                Some(serde_json::to_value(spec).unwrap_or(serde_json::Value::Null)),
            ),
        };

        tasks.push(Task {
            name: task_name.clone(),
            depends_on: task_from_cr.depends_on,
            quantum,
            classical,
            qcbm,
        });

        // Use the status from the job map, or default to Pending if no job is found yet
        let status = job_status_map
            .get(&task_name)
            .cloned()
            .unwrap_or_else(|| "Pending".to_string());
        task_status_map.insert(task_name, status);
    }

    let response = SyntheticWorkflow {
        metadata: Metadata {
            name: workflow_name,
            namespace: params.namespace,
        },
        spec: Spec { tasks },
        status: Status {
            task_status: task_status_map,
        },
    };

    Ok(Json(response))
}

async fn fetch_task_results(
    State(state): State<Arc<AppState>>,
    Path((namespace, _workflow_name, task_name)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let pods: Api<Pod> = Api::namespaced(state.client.clone(), &namespace);
    let jobs: Api<Job> = Api::namespaced(state.client.clone(), &namespace);

    // Find the job associated with the task name to construct the correct pod label selector
    let job_list = jobs.list(&ListParams::default()).await.map_err(|e| {
        eprintln!("Error listing jobs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let job_name = job_list
        .items
        .into_iter()
        .find(|job| {
            job.metadata.labels.as_ref().map_or(false, |labels| {
                labels.get("qflow.io/task-name") == Some(&task_name)
            })
        })
        .and_then(|job| job.metadata.name);

    let job_name = match job_name {
        Some(name) => name,
        None => {
            eprintln!("No job found for task '{}'", task_name);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    let pod_label = format!("job-name={}", job_name);
    let lp = ListParams::default().labels(&pod_label);

    let pod_list = pods.list(&lp).await.map_err(|e| {
        eprintln!("Error listing pods: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Find a succeeded pod to fetch logs from
    if let Some(pod) = pod_list.items.into_iter().find(|p| {
        p.status
            .as_ref()
            .map_or(false, |s| s.phase == Some("Succeeded".to_string()))
    }) {
        if let Some(pod_name) = &pod.metadata.name {
            let logs = pods
                .logs(pod_name, &LogParams::default())
                .await
                .map_err(|e| {
                    eprintln!("Error fetching logs for pod '{}': {}", pod_name, e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            // Try to parse logs as JSON, otherwise return as raw string
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

async fn submit_workflow(
    State(state): State<Arc<AppState>>,
    Path((namespace)): Path<(String)>,
    Json(workflow): Json<QuantumWorkflowSpec>,
) -> Result<StatusCode, StatusCode> {
    // check the workflow
    println!("Submitting workflow '{:?}'", workflow);

    let wf_api: Api<QuantumWorkflow> = Api::namespaced(state.client.clone(), &namespace);

    // todo: will need to handle types of WorkflowSpec here
    // For now, we assume the workflow is of type QuantumSVMWorkflowSpec

    // Convert the SyntheticWorkflow to a QuantumWorkflow CR
    let quantum_workflow = QuantumWorkflow {
        metadata: kube::api::ObjectMeta {
            name: Some("workflow_name".parse().unwrap()),
            namespace: Some(namespace),
            ..Default::default()
        },
        spec: workflow,
        status: Default::default(),
    };

    // Create or update the QuantumWorkflow CR
    match wf_api
        .create(&PostParams::default(), &quantum_workflow)
        .await
    {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(e) => {
            eprintln!("Error submitting workflow: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn submit_qasm(
    State(state): State<Arc<AppState>>,
    Path((namespace, workflow_name)): Path<(String, String)>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<StatusCode, StatusCode> {
    let qasm_data = form.get("qasm_data").cloned().unwrap_or_default();
    println!(
        "Submitting QASM for workflow '{}': {}",
        workflow_name, qasm_data
    );

    // Construct a Quantum task using the QASM string
    let quantum_task = qflow_types::QFlowTask {
        name: "qasm-task".to_string(),
        depends_on: None,
        spec: qflow_types::QFlowTaskSpec::Quantum {
            image: "your-quantum-image:latest".to_string(), // <-- Replace with your actual image
            circuit: qasm_data.clone(),
            params: "".to_string(), // You may want to parse/accept params separately
        },
    };

    // Build the workflow spec
    let workflow_spec = qflow_types::QuantumWorkflowSpec {
        volume: None,
        tasks: vec![quantum_task],
    };

    // Build the QuantumWorkflow CR
    let quantum_workflow = qflow_types::QuantumWorkflow {
        metadata: kube::api::ObjectMeta {
            name: Some(workflow_name.clone()),
            namespace: Some(namespace.clone()),
            ..Default::default()
        },
        spec: workflow_spec,
        status: Default::default(),
    };

    // Submit to Kubernetes
    let wf_api: Api<qflow_types::QuantumWorkflow> =
        Api::namespaced(state.client.clone(), &namespace);

    match wf_api
        .create(&PostParams::default(), &quantum_workflow)
        .await
    {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(e) => {
            eprintln!("Error submitting QASM workflow: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Deserialize)]
struct MlSvmParams {
    test_size: f64,
    target_column: String,
    // ... other params
}

#[derive(Serialize)]
struct MlSvmResult {
    metrics: String,
    plot_base64: String,
}

async fn run_ml_svm(
    State(_state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<MlSvmResult>, StatusCode> {
    // --- 1. Parse multipart form ---
    let mut csv_path = None;
    let mut target_column = None;
    let mut test_size = None;


    // TODO: This needs to get refactored to use Kubernetes Jobs instead
    // check out the PVC viewer
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or("");
        match name {
            "data_file" => {
                let mut file =
                    NamedTempFile::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                let data = field
                    .bytes()
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                file.write_all(&data)
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                csv_path = Some(file.into_temp_path());
            }
            "target_column" => {
                target_column = Some(field.text().await.unwrap_or_default());
            }
            "test_size" => {
                test_size = Some(field.text().await.unwrap_or_default());
            }
            _ => {}
        }
    }

    let csv_path = csv_path.ok_or(StatusCode::BAD_REQUEST)?;
    let target_column = target_column.ok_or(StatusCode::BAD_REQUEST)?;
    let test_size = test_size.ok_or(StatusCode::BAD_REQUEST)?;

    // --- 2. Prepare output paths ---
    let metrics_path = csv_path.with_extension("metrics.txt");
    let plot_path = csv_path.with_extension("plot.png");

    // --- 3. Run Python script ---
    let python_args = [
        "ml/svm2.py",
        "--data_path",
        csv_path.to_str().unwrap(),
        "--target-column",
        &target_column,
        "--output-plot",
        plot_path.to_str().unwrap(),
        "--output-metrics",
        metrics_path.to_str().unwrap(),
        "--test-size",
        &test_size,
    ];

    let output = tokio::process::Command::new("/Users/nathaniel.ham/RustroverProjects/qflow/ml/venv/bin/python")
        .args(&python_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| { eprintln!("Python process error: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR });

    let output = match output {
        Ok(output) => output,
        Err(status) => {
            println!("Error while running ml_svm: {:?}", status);
            return Err(StatusCode::INTERNAL_SERVER_ERROR)
        },
    };

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        eprintln!("Python error: {}", err);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // --- 4. Read metrics and plot ---
    let metrics = tokio::fs::read_to_string(&metrics_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut plot_file = tokio::fs::File::open(&plot_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut plot_bytes = Vec::new();
    plot_file
        .read_to_end(&mut plot_bytes)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let plot_base64 = base64::encode(&plot_bytes);

    Ok(Json(MlSvmResult {
        metrics,
        plot_base64,
    }))
}
