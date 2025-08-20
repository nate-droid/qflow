use axum::extract::Multipart;
use axum::extract::Request;
use axum::routing::post;
use axum::{
    Form, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};

use k8s_openapi::api::{batch::v1::Job, core::v1::Pod};
use kube::{
    Client,
    api::{Api, ListParams, LogParams, PostParams},
};
use qflow_types::{QFlowTaskSpec, QuantumWorkflow, QuantumWorkflowSpec};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::{collections::HashMap, sync::Arc};
use tempfile::NamedTempFile;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    qcbm: Option<serde_json::Value>,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct Status {
    task_status: HashMap<String, String>,
}

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
        .route("/api/ml/svm", post(run_ml_svm))
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

    let workflow_cr = wf_api.get(&workflow_name).await.map_err(|e| {
        eprintln!("Error fetching QuantumWorkflow '{}': {}", workflow_name, e);
        StatusCode::NOT_FOUND
    })?;

    let all_jobs = job_api.list(&ListParams::default()).await.map_err(|e| {
        eprintln!("Error listing jobs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut job_status_map: HashMap<String, String> = HashMap::new();
    for job in all_jobs.items {
        if let Some(owner_refs) = job.metadata.owner_references.as_ref() {
            if owner_refs.iter().any(|owner| owner.name == workflow_name) {
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

    let quantum_task = qflow_types::QFlowTask {
        name: "qasm-task".to_string(),
        depends_on: None,
        spec: QFlowTaskSpec::Quantum {
            image: "your-quantum-image:latest".to_string(),
            circuit: qasm_data.clone(),
            params: "".to_string(),
        },
    };

    let workflow_spec = QuantumWorkflowSpec {
        volume: None,
        tasks: vec![quantum_task],
    };

    let quantum_workflow = QuantumWorkflow {
        metadata: kube::api::ObjectMeta {
            name: Some(workflow_name.clone()),
            namespace: Some(namespace.clone()),
            ..Default::default()
        },
        spec: workflow_spec,
        status: Default::default(),
    };

    let wf_api: Api<QuantumWorkflow> = Api::namespaced(state.client.clone(), &namespace);

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
}

#[derive(Serialize)]
struct MlSvmResult {
    metrics: String,
    plot_base64: String,
}

async fn run_ml_svm(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut csv_path = None;
    let mut target_column = None;
    let mut test_size = None;

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

    let job_name = format!("ml-svm-job-{}", "job-12345");
    let namespace = "default";
    let image = "qsim:latest";
    let csv_file_name = "input.csv";

    // TODO: refactor this to create a QFlowTaskSpec for SVM
    // let quantum_task = qflow_types::QFlowTask {
    //     name: "qasm-task".to_string(),
    //     depends_on: None,
    //     spec: QFlowTaskSpec::Quantum {
    //         image: "your-quantum-image:latest".to_string(), // <-- Replace with your actual image
    //         circuit: qasm_data.clone(),
    //         params: "".to_string(), // You may want to parse/accept params separately
    //     },
    // };

    // Save the uploaded CSV to a location accessible by the Job (e.g., a PVC or object storage)
    // For now, this is a placeholder. You may need to implement PVC upload or use a shared volume.
    // Here, we assume the Job can access the file at /data/input.csv

    // Build Job spec
    let job_spec = serde_json::json!({
        "apiVersion": "batch/v1",
        "kind": "Job",
        "metadata": {
            "name": job_name,
            "namespace": namespace,
            "labels": {
                "qflow.io/task-name": job_name
            }
        },
        "spec": {
            "template": {
                "spec": {
                    "containers": [{
                        "name": "ml-svm",
                        "image": image,
                        "args": [
                            "--data_path", format!("/data/{}", csv_file_name),
                            "--target-column", target_column,
                            "--output-plot", "/data/plot.png",
                            "--output-metrics", "/data/metrics.txt",
                            "--test-size", test_size
                        ],
                        "volumeMounts": [{
                            "name": "data-volume",
                            "mountPath": "/data"
                        }]
                    }],
                    "restartPolicy": "Never",
                    "volumes": [{
                        "name": "data-volume",
                        // Define your PVC here
                        "persistentVolumeClaim": { "claimName": "your-pvc" }
                    }]
                }
            }
        }
    });

    let job_api: Api<Job> = Api::namespaced(state.client.clone(), namespace);
    let job: Job =
        serde_json::from_value(job_spec).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match job_api.create(&PostParams::default(), &job).await {
        Ok(_) => Ok(Json(serde_json::json!({
            "message": "SVM Job submitted",
            "job_name": job_name
        }))),
        Err(e) => {
            eprintln!("Error submitting SVM Job: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
