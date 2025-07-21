use std::collections::{BTreeMap, HashMap};
use std::hash::RandomState;
use futures_util::StreamExt;
use kube::{
    api::{Api, ListParams, Patch, PatchParams, PostParams},
    client::Client,
    runtime::{controller::Action, Controller},
    CustomResource, Resource,
};
use petgraph::{graphmap::DiGraphMap, visit::Topo, Graph};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::time::Duration;
use tracing::{error, info, warn};
use schemars::JsonSchema;
use serde_json::json;

use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::api::core::v1::{PodTemplateSpec, PodSpec, Container, ConfigMap, Volume, VolumeMount, ConfigMapVolumeSource, PersistentVolumeClaim, PersistentVolumeClaimSpec, ResourceRequirements, VolumeResourceRequirements, PersistentVolumeClaimVolumeSource};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use qflow_types::{QFlowTask, QFlowTaskSpec};
use qflow_types::{QuantumWorkflowSpec, QuantumWorkflowStatus, QuantumWorkflow};

// --- 2. Error Handling ---
// A custom error enum for our reconciler logic
#[derive(Error, Debug)]
pub enum Error {
    #[error("Kubernetes API Error: {0}")]
    KubeError(#[from] kube::Error),
    #[error("An error occurred: {0}")]
    Anyhow(#[from] anyhow::Error),
    #[error("MissingObjectKey: {0}")]
    MissingObjectKey(&'static str),
    #[error("Workflow is invalid: {0}")] InvalidWorkflow(String),
}

const PVC_NAME: &str = "qflow-workspace";
const TASK_PENDING: &str = "Pending";
const TASK_RUNNING: &str = "Running";
const TASK_SUCCEEDED: &str = "Succeeded";
const TASK_FAILED: &str = "Failed";
const QFLOW_TASK_NAME_LABEL: &str = "qflow.io/task-name";

async fn create_pvc_if_not_exists(client: &Client, wf: &QuantumWorkflow) -> Result<(), Error> {
    let ns = wf.metadata.namespace.clone().ok_or(Error::MissingObjectKey("namespace"))?;
    let pvc_api = Api::<PersistentVolumeClaim>::namespaced(client.clone(), &ns);
    let pvc_name = format!("{}-{}", wf.metadata.name.clone().unwrap(), PVC_NAME);

    if pvc_api.get(&pvc_name).await.is_err() {
        info!("PVC {} not found, creating.", pvc_name);
        let size = wf.spec.volume.as_ref().map(|v| v.size.clone()).unwrap_or_else(|| "1Gi".to_string());
        let pvc = PersistentVolumeClaim {
            metadata: ObjectMeta {
                name: Some(pvc_name),
                owner_references: Some(vec![wf.controller_owner_ref(&()).unwrap()]),
                ..Default::default()
            },
            spec: Some(PersistentVolumeClaimSpec {
                access_modes: Some(vec!["ReadWriteOnce".to_string()]),
                resources: Some(VolumeResourceRequirements {
                    requests: Some([("storage".to_string(), Quantity(size))].into()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        pvc_api.create(&PostParams::default(), &pvc).await?;
    }
    Ok(())
}

fn create_job_for_task(wf: &QuantumWorkflow, task: &QFlowTask, cm_name: Option<String>) -> Result<Job, Error> {
    let pvc_name = format!("{}-{}", wf.metadata.name.clone().unwrap(), PVC_NAME);
    let (image, config_map_mount) = match &task.spec {
        QFlowTaskSpec::Classical { image } => (image.clone(), None),
        QFlowTaskSpec::Quantum { image, .. } => {
            let mount = VolumeMount {
                name: "qflow-input".to_string(),
                mount_path: "/workspace/input".to_string(),
                read_only: Some(true),
                recursive_read_only: None,
                sub_path: None,
                mount_propagation: None,
                sub_path_expr: None,
            };
            (image.clone(), Some(mount))
        }
    };

    let mut volumes = vec![Volume {
        name: "qflow-workspace".to_string(),
        persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
            claim_name: pvc_name,
            ..Default::default()
        }),
        ..Default::default()
    }];
    let mut volume_mounts = vec![VolumeMount {
        name: "qflow-workspace".to_string(),
        mount_path: "/workspace".to_string(),
        ..Default::default()
    }];

    if let (Some(cm), Some(mount)) = (cm_name, config_map_mount) {
        volumes.push(Volume {
            name: "qflow-input".to_string(),
            config_map: Some(ConfigMapVolumeSource { name: cm, ..Default::default() }),
            ..Default::default()
        });
        volume_mounts.push(mount);
    }

    let job_name = format!("{}-{}", wf.metadata.name.clone().unwrap(), task.name);
    let input_file_path = "/workspace/input/circuit.qasm"; // Example input file path

    Ok(Job {
        metadata: ObjectMeta {
            name: Some(job_name),
            owner_references: Some(vec![wf.controller_owner_ref(&()).unwrap()]),
            labels: Some([(QFLOW_TASK_NAME_LABEL.to_string(), task.name.clone())].into()),
            ..Default::default()
        },
        spec: Some(JobSpec {
            template: PodTemplateSpec {
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: "task-runner".to_string(),
                        image: Some(image),
                        command: Some(vec!["/qsim".to_string()]), // Replace with the actual executable
                        args: Some(vec!["--input-file".to_string(), input_file_path.to_string()]),
                        volume_mounts: Some(volume_mounts),
                        image_pull_policy: Some("Never".to_string()),
                        ..Default::default()
                    }],
                    volumes: Some(volumes),
                    restart_policy: Some("Never".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            backoff_limit: Some(4),
            ..Default::default()
        }),
        ..Default::default()
    })
}

async fn update_status(api: &Api<QuantumWorkflow>, name: &str, status: QuantumWorkflowStatus) -> Result<(), Error> {
    let patch = Patch::Merge(serde_json::json!({
        "status": status
    }));
    api.patch_status(name, &PatchParams::default(), &patch).await?;
    Ok(())
}

// --- 3. The Reconciliation Logic ---
// This is the core of the operator. It's called whenever a QuantumWorkflow
// resource changes.

async fn reconcile(wf: Arc<QuantumWorkflow>, ctx: Arc<Context>) -> Result<Action, Error> {
    // println!("reconciling {:?}", wf);
    let client = &ctx.client;
    let ns = wf.metadata.namespace.clone().ok_or(Error::MissingObjectKey("namespace"))?;
    let wf_api = Api::<QuantumWorkflow>::namespaced(client.clone(), &ns);
    let job_api = Api::<Job>::namespaced(client.clone(), &ns);
    let cm_api = Api::<ConfigMap>::namespaced(client.clone(), &ns);

    // 1. Initialize status and PVC if they don't exist
    if wf.status.is_none() || wf.status.as_ref().unwrap().task_statuses.is_none() {
        info!("Initializing status for workflow '{}'", wf.metadata.name.clone().unwrap());
        create_pvc_if_not_exists(client, &wf).await?;
        let mut initial_statuses = BTreeMap::new();
        for task in &wf.spec.tasks {
            initial_statuses.insert(task.name.clone(), TASK_PENDING.to_string());
        }
        let status = QuantumWorkflowStatus { phase: Some(TASK_PENDING.to_string()), task_statuses: Some(initial_statuses) };
        update_status(&wf_api, &wf.metadata.name.clone().unwrap(), status).await?;
        return Ok(Action::requeue(Duration::from_secs(1))); // Requeue immediately to process
    }

    let mut real_task_statuses = BTreeMap::new();
    let owned_jobs = job_api.list(&ListParams::default().labels(&format!("app.kubernetes.io/instance={}", wf.metadata.name.clone().unwrap()))).await?;

    for job in owned_jobs {
        if let Some(labels) = job.metadata.labels {
            if let Some(task_name) = labels.get(QFLOW_TASK_NAME_LABEL) {
                let status = if job.status.as_ref().and_then(|s| s.succeeded).unwrap_or(0) > 0 {
                    TASK_SUCCEEDED
                } else if job.status.as_ref().and_then(|s| s.failed).unwrap_or(0) > 0 {
                    TASK_FAILED
                } else {
                    TASK_RUNNING
                };
                real_task_statuses.insert(task_name.clone(), status.to_string());
            }
        }
    }

    // Fill in any tasks from the spec that don't have a job yet
    for task in &wf.spec.tasks {
        real_task_statuses.entry(task.name.clone()).or_insert(TASK_PENDING.to_string());
    }

    // 2. Build DAG and check for cycles

    //let mut graph = DiGraphMap::<&str, ()>::new();
    let mut graph = DiGraphMap::<&str, _, RandomState>::new();
    let task_map: HashMap<&str, &QFlowTask> = wf.spec.tasks.iter().map(|t| (t.name.as_str(), t)).collect();

    for task in &wf.spec.tasks {
        graph.add_node(&task.name);
    }
    for task in &wf.spec.tasks {
        if let Some(deps) = &task.depends_on {
            for dep_name in deps {
                if !graph.contains_node(dep_name) {
                    return Err(Error::InvalidWorkflow(format!("Task '{}' depends on non-existent task '{}'", task.name, dep_name)));
                }
                graph.add_edge(dep_name, &task.name, ());
            }
        }
    }
    if petgraph::algo::is_cyclic_directed(&graph) {
        return Err(Error::InvalidWorkflow("Workflow has a cycle".to_string()));
    }
    if petgraph::algo::is_cyclic_directed(&graph) {
        return Err(Error::InvalidWorkflow("Workflow has a cycle".to_string()));
    }

    // 3. Process tasks based on status
    let mut current_statuses = wf.status.as_ref().unwrap().task_statuses.as_ref().unwrap().clone();
    let mut made_change = false;

    // Check running jobs
    for (task_name, status) in current_statuses.iter_mut() {
        if *status == TASK_RUNNING {
            let job_name = format!("{}-{}", wf.metadata.name.clone().unwrap(), task_name);
            match job_api.get_status(&job_name).await {
                Ok(job) => {
                    if let Some(s) = job.status {
                        if s.succeeded.unwrap_or(0) > 0 { *status = TASK_SUCCEEDED.to_string(); made_change = true; }
                        else if s.failed.unwrap_or(0) > 0 { *status = TASK_FAILED.to_string(); made_change = true; }
                    }
                },
                Err(e) => error!("Failed to get job status for {}: {}", job_name, e),
            }
        }
    }

    // Start new jobs

    // assert that graph returns a QFlowTask

    let mut topo = Topo::new(&graph);
    while let Some(node_idx) = topo.next(&graph) {
        // let task: &QFlowTask = graph[node_idx];
        let task = task_map[node_idx];
        // let task: &QFlowTask = graph[node_idx];
        let task_name = &task.name;
        if current_statuses.get(task_name) == Some(&TASK_PENDING.to_string()) {
            let deps_succeeded = task.depends_on.as_ref().map_or(true, |deps| {
                deps.iter().all(|dep_name| current_statuses.get(dep_name) == Some(&TASK_SUCCEEDED.to_string()))
            });

            if deps_succeeded {
                info!("Dependencies met for task '{}', starting job.", task_name);
                let cm_name = if let QFlowTaskSpec::Quantum { circuit, params, .. } = &task.spec {
                    let cm_name = format!("{}-{}-cm", wf.metadata.name.clone().unwrap(), task.name);
                    let cm = ConfigMap {
                        metadata: ObjectMeta { name: Some(cm_name.clone()), owner_references: Some(vec![wf.controller_owner_ref(&()).unwrap()]), ..Default::default() },
                        data: Some([("circuit.qasm".to_string(), circuit.clone()), ("params.json".to_string(), params.clone())].into()),
                        ..Default::default()
                    };
                    cm_api.create(&PostParams::default(), &cm).await?;
                    Some(cm_name)
                } else { None };

                let job = create_job_for_task(&wf, task, cm_name)?;
                job_api.create(&PostParams::default(), &job).await?;
                current_statuses.insert(task_name.clone(), TASK_RUNNING.to_string());
                made_change = true;
            }
        }
    }

    // 4. Update overall workflow status
    let final_phase = if current_statuses.values().any(|s| s == TASK_FAILED) {
        Some(TASK_FAILED.to_string())
    } else if current_statuses.values().all(|s| s == TASK_SUCCEEDED) {
        Some(TASK_SUCCEEDED.to_string())
    } else {
        Some(TASK_RUNNING.to_string())
    };

    if made_change || wf.status.as_ref().unwrap().phase != final_phase {
        let new_status = QuantumWorkflowStatus { phase: final_phase, task_statuses: Some(current_statuses) };
        update_status(&wf_api, &wf.metadata.name.clone().unwrap(), new_status).await?;
    }

    Ok(Action::requeue(Duration::from_secs(15)))
}

/// The context for our reconciler.
struct Context {
    client: Client,
}

/// A helper function for handling errors during reconciliation.
fn on_error(wf: Arc<QuantumWorkflow>, error: &Error, _ctx: Arc<Context>) -> Action {
    warn!("Reconciliation error for '{:?}': {:?}", wf.metadata.name, error);
    println!("Reconciliation error for '{:?}': {:?} in ns: {:?}", wf.metadata.name, error, wf.metadata.namespace);
    // Requeue after 5 seconds on error.
    Action::requeue(Duration::from_secs(5))
}

// --- 4. The `main` function to run the operator ---
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing (for logging)
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let client = Client::try_default().await?;
    let context = Arc::new(Context {
        client: client.clone(),
    });
    println!("Connecting to Kubernetes API");
    let workflows = Api::<QuantumWorkflow>::all(client);
    println!("Listing Quantum Workflows");
    println!("total: {}", workflows.list(&ListParams::default()).await?.items.len());

    info!("Starting qflow-operator");

    Controller::new(workflows, Default::default())
        .run(reconcile, on_error, context)
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("Reconciled {:?}", o),
                Err(e) => warn!("Reconciliation failed: {}", e),
            }
        })
        .await;

    Ok(())
}