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
use axum::routing::post;
use kube::api::{DynamicObject, GroupVersionKind, PostParams};
use tower_http::cors::{Any, CorsLayer};
use qflow_types::{QuantumWorkflow, QuantumWorkflowSpec};
use qflowc::compile_qflow_file;


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
        .route("/api/workflows/{namespace}/{name}/qasm", post(submit_qasm))
        .with_state(app_state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}


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

    if workflow_jobs.is_empty() {
        eprintln!("No jobs found with prefix '{}' in namespace '{}'", workflow_name, params.namespace);
        return Err(StatusCode::NOT_FOUND);
    }

    let mut tasks = Vec::new();
    let mut task_status_map = HashMap::new();

    for job in workflow_jobs {
        let annotations = job.metadata.annotations.as_ref();
        let labels = job.metadata.labels.as_ref();

        // Prefer the explicit task-name label, but fall back to the job's full name.
        let task_name = labels.and_then(|l| l.get("quantum.workflow/task-name").cloned())
            .unwrap_or_else(|| job.metadata.name.clone().unwrap_or_default());

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

        // get task status
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

    Ok(Json(workflow))
}


async fn fetch_task_results(
    State(state): State<Arc<AppState>>,
    Path((namespace, workflow_name, task_name)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let pods: Api<Pod> = Api::namespaced(state.client.clone(), &namespace);

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
async fn submit_qasm(
    State(state): State<Arc<AppState>>,
    Path((namespace, name)): Path<(String, String)>,
    body: String,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // this will accept a QASM file and invoke the qflowc compiler to create a Kubernetes Job and apply it

    // first construct a basic .qflow and inject the QASM content
    let qasm_content = body.trim();
    if qasm_content.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let qasm_content = qasm_content.replace("\r\n", "\n").replace("\r", "\n");

    // write the QASM content to a temporary file
    let qasm_path = std::env::temp_dir().join("temp_circuit.qasm");
    std::fs::write(&qasm_path, qasm_content).map_err(|e| {
        eprintln!("Error writing temporary QASM file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let qasm_path = qasm_path.to_str().unwrap_or_default();
    // creating a generic QFlow workflow content as this is for basic tasks
    // for now, more involved workflows can be constructed via cli

    let qflow_content = format!(
        "workflow {}  {{ \n\
            task simple-task {{ \n\
                circuit_from: \"{}\",\n\
                image: \"qsim\",\n\
                params_from: \"qflowc/examples/sim_params.json\"\n\
            }}
        \n}}",
        name,
        qasm_path
    );
    println!("QASM content:\n{}", qflow_content);
    let qflowc_path = std::env::temp_dir().join("temp_workflow.qflow");

    std::fs::write(&qflowc_path, qflow_content).map_err(|e| {
        eprintln!("Error writing temporary QFlow file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let res = compile_qflow_file(&qflowc_path);

    if let Ok(yaml_output) = res {
        let workflow: QuantumWorkflow = serde_yaml::from_str(&yaml_output).map_err(|e| {
            eprintln!("Error parsing YAML output: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let api: Api<QuantumWorkflow> = Api::namespaced(state.client.clone(), &namespace);

        api.create(&PostParams::default(), &workflow)
            .await
            .map_err(|e| {
                eprintln!("Error creating QuantumWorkflow: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let response = serde_json::json!({
            "message": "QuantumWorkflow created successfully",
            "workflow": workflow,
        });
        Ok(Json(response))
    } else {
        eprintln!("Error compiling QFlow file: {:?}", res);
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}