use std::collections::{BTreeMap, HashMap};
use std::hash::RandomState;
use std::sync::Arc;

use futures_util::StreamExt;
use kube::{
    Resource,
    api::{Api, Patch, PatchParams, PostParams},
    client::Client,
    runtime::{Controller, controller::Action},
};
use petgraph::{graphmap::DiGraphMap, visit::Topo};
use thiserror::Error;
use tokio::time::Duration;
use tracing::{error, info, warn};

use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::api::core::v1::{
    ConfigMap, ConfigMapVolumeSource, Container, PersistentVolumeClaim, PersistentVolumeClaimSpec,
    PersistentVolumeClaimVolumeSource, PodSpec, PodTemplateSpec, Volume, VolumeMount,
    VolumeResourceRequirements,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

use qflow_types::{QFlowTask, QFlowTaskSpec, QcbmOptimizerSpec, QuantumWorkflow};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Defines the volume for a workflow.
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct WorkflowVolumeSpec {
    pub size: String,
}

/// Represents the observed state of a QuantumWorkflow.
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Default)]
pub struct QuantumWorkflowStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_statuses: Option<BTreeMap<String, String>>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Kubernetes API Error: {0}")]
    KubeError(#[from] kube::Error),
    #[error("An error occurred: {0}")]
    Anyhow(#[from] anyhow::Error),
    #[error("MissingObjectKey: {0}")]
    MissingObjectKey(&'static str),
    #[error("Workflow is invalid: {0}")]
    InvalidWorkflow(String),
}

const PVC_NAME: &str = "qflow-workspace";
const TASK_PENDING: &str = "Pending";
const TASK_RUNNING: &str = "Running";
const TASK_SUCCEEDED: &str = "Succeeded";
const TASK_FAILED: &str = "Failed";
const QFLOW_TASK_NAME_LABEL: &str = "qflow.io/task-name";

async fn create_pvc_if_not_exists(client: &Client, wf: &QuantumWorkflow) -> Result<(), Error> {
    let ns = wf
        .metadata
        .namespace
        .clone()
        .ok_or(Error::MissingObjectKey("namespace"))?;
    let pvc_api = Api::<PersistentVolumeClaim>::namespaced(client.clone(), &ns);
    let pvc_name = format!("{}-{}", wf.metadata.name.clone().unwrap(), PVC_NAME);

    if pvc_api.get(&pvc_name).await.is_err() {
        info!("PVC {} not found, creating.", pvc_name);
        let size = wf
            .spec
            .volume
            .as_ref()
            .map(|v| v.size.clone())
            .unwrap_or_else(|| "1Gi".to_string());
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

/// Creates a Kubernetes Job for a given task spec.
/// This function has been refactored to handle Classical, Quantum, and the new QCBM task types.
fn create_job_for_task(
    wf: &QuantumWorkflow,
    task: &QFlowTask,
    cm_name: Option<String>,
) -> Result<Job, Error> {
    let pvc_name = format!("{}-{}", wf.metadata.name.clone().unwrap(), PVC_NAME);

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

    let container = match &task.spec {
        QFlowTaskSpec::Classical { image } => Container {
            name: "task-runner".to_string(),
            image: Some(image.clone()),
            command: Some(vec!["/qsim".to_string()]),
            volume_mounts: Some(volume_mounts),
            image_pull_policy: Some("Never".to_string()),
            ..Default::default()
        },
        QFlowTaskSpec::Quantum { image, .. } => {
            let mount = VolumeMount {
                name: "qflow-input".to_string(),
                mount_path: "/workspace/input".to_string(),
                read_only: Some(true),
                ..Default::default()
            };
            volume_mounts.push(mount);

            if let Some(cm) = cm_name {
                volumes.push(Volume {
                    name: "qflow-input".to_string(),
                    config_map: Some(ConfigMapVolumeSource {
                        name: cm,
                        ..Default::default()
                    }),
                    ..Default::default()
                });
            }
            let default_image = "qsim:latest".to_string();
            let input_file_path = "/workspace/input/circuit.qasm";
            Container {
                name: "task-runner".to_string(),
                image: Some(default_image),
                command: Some(vec!["/qsim".to_string()]),
                args: Some(vec![
                    "--input-file".to_string(),
                    input_file_path.to_string(),
                ]),
                volume_mounts: Some(volume_mounts),
                image_pull_policy: Some("Never".to_string()),
                ..Default::default()
            }
        }
        QFlowTaskSpec::Qcbm(qcbm_spec) => {
            let training_data_json = serde_json::to_string(&qcbm_spec.training_data)
                .map_err(|e| Error::Anyhow(anyhow::Error::from(e)))?;

            let optimizer_spec = qcbm_spec
                .optimizer
                .clone()
                .unwrap_or_else(|| QcbmOptimizerSpec {
                    name: "Adam".to_string(),
                    epochs: 100,
                    learning_rate: 0.01,
                    initial_params: None,
                });

            let mut args = vec![
                "--ansatz".to_string(),
                qcbm_spec.ansatz.clone(),
                "--training-data".to_string(),
                training_data_json,
                "--epochs".to_string(),
                optimizer_spec.epochs.to_string(),
                "--learning-rate".to_string(),
                optimizer_spec.learning_rate.to_string(),
            ];
            if let Some(params) = optimizer_spec.initial_params {
                args.push("--initial-params".to_string());
                args.push(params);
            }

            Container {
                name: "task-runner".to_string(),
                image: Some(qcbm_spec.image.clone()),
                args: Some(args),
                volume_mounts: Some(volume_mounts),
                image_pull_policy: Some("Never".to_string()),
                ..Default::default()
            }
        }
    };

    let job_name = format!("{}-{}", wf.metadata.name.clone().unwrap(), task.name);
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
                    containers: vec![container],
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

async fn update_status(
    api: &Api<QuantumWorkflow>,
    name: &str,
    status: QuantumWorkflowStatus,
) -> Result<(), Error> {
    let patch = Patch::Merge(serde_json::json!({ "status": status }));
    api.patch_status(name, &PatchParams::default(), &patch)
        .await?;
    Ok(())
}

async fn reconcile(wf: Arc<QuantumWorkflow>, ctx: Arc<Context>) -> Result<Action, Error> {
    let client = &ctx.client;
    let ns = wf
        .metadata
        .namespace
        .clone()
        .ok_or(Error::MissingObjectKey("namespace"))?;
    let wf_api = Api::<QuantumWorkflow>::namespaced(client.clone(), &ns);
    let job_api = Api::<Job>::namespaced(client.clone(), &ns);
    let cm_api = Api::<ConfigMap>::namespaced(client.clone(), &ns);

    if wf.status.is_none() {
        info!(
            "Initializing status for workflow '{}'",
            wf.metadata.name.clone().unwrap()
        );
        create_pvc_if_not_exists(client, &wf).await?;
        let mut initial_statuses = BTreeMap::new();
        for task in &wf.spec.tasks {
            initial_statuses.insert(task.name.clone(), TASK_PENDING.to_string());
        }
        let status = QuantumWorkflowStatus {
            phase: Some(TASK_PENDING.to_string()),
            task_statuses: Some(initial_statuses),
        };
        update_status(&wf_api, &wf.metadata.name.clone().unwrap(), status).await?;
        return Ok(Action::requeue(Duration::from_secs(1)));
    }

    let mut graph = DiGraphMap::<&str, _, RandomState>::new();
    let task_map: HashMap<&str, &QFlowTask> =
        wf.spec.tasks.iter().map(|t| (t.name.as_str(), t)).collect();

    for task in &wf.spec.tasks {
        graph.add_node(&task.name);
    }
    for task in &wf.spec.tasks {
        if let Some(deps) = &task.depends_on {
            for dep_name in deps {
                if !graph.contains_node(dep_name) {
                    return Err(Error::InvalidWorkflow(format!(
                        "Task '{}' depends on non-existent task '{}'",
                        task.name, dep_name
                    )));
                }
                graph.add_edge(dep_name, &task.name, ());
            }
        }
    }
    if petgraph::algo::is_cyclic_directed(&graph) {
        return Err(Error::InvalidWorkflow("Workflow has a cycle".to_string()));
    }

    let mut current_statuses = wf
        .status
        .as_ref()
        .and_then(|s| s.task_statuses.as_ref())
        .cloned()
        .unwrap_or_default();
    let mut made_change = false;

    for (task_name, status) in current_statuses.iter_mut() {
        if *status == TASK_RUNNING {
            let job_name = format!("{}-{}", wf.metadata.name.clone().unwrap(), task_name);
            match job_api.get_status(&job_name).await {
                Ok(job) => {
                    if let Some(s) = job.status {
                        if s.succeeded.unwrap_or(0) > 0 {
                            *status = TASK_SUCCEEDED.to_string();
                            made_change = true;
                        } else if s.failed.unwrap_or(0) > 0 {
                            *status = TASK_FAILED.to_string();
                            made_change = true;
                        }
                    }
                }
                Err(e) => error!("Failed to get job status for {}: {}", job_name, e),
            }
        }
    }

    for task in &wf.spec.tasks {
        let task_name = &task.name;
        if !current_statuses.contains_key(task_name) {
            current_statuses.insert(task_name.clone(), TASK_PENDING.to_string());
        }
    }

    let mut topo = Topo::new(&graph);
    while let Some(node_idx) = topo.next(&graph) {
        let task = task_map[node_idx];
        let task_name = &task.name;
        if current_statuses.get(task_name) == Some(&TASK_PENDING.to_string()) {
            let deps_succeeded = task.depends_on.as_ref().map_or(true, |deps| {
                deps.iter().all(|dep_name| {
                    current_statuses.get(dep_name) == Some(&TASK_SUCCEEDED.to_string())
                })
            });

            if deps_succeeded {
                info!("Dependencies met for task '{}', starting job.", task_name);
                let cm_name = if let QFlowTaskSpec::Quantum {
                    circuit, params, ..
                } = &task.spec
                {
                    let cm_name = format!("{}-{}-cm", wf.metadata.name.clone().unwrap(), task.name);
                    match cm_api.get(&cm_name).await {
                        Ok(_) => {
                            info!("ConfigMap '{}' already exists, skipping creation.", cm_name);
                        }
                        Err(_) => {
                            let cm = ConfigMap {
                                metadata: ObjectMeta {
                                    name: Some(cm_name.clone()),
                                    owner_references: Some(vec![
                                        wf.controller_owner_ref(&()).unwrap(),
                                    ]),
                                    ..Default::default()
                                },
                                data: Some(
                                    [
                                        ("circuit.qasm".to_string(), circuit.clone()),
                                        ("params.json".to_string(), params.clone()),
                                    ]
                                    .into(),
                                ),
                                ..Default::default()
                            };
                            cm_api.create(&PostParams::default(), &cm).await?;
                        }
                    }
                    Some(cm_name)
                } else {
                    None
                };

                // This single function call now handles all task types
                let job_name = format!("{}-{}", wf.metadata.name.clone().unwrap(), task_name);
                match job_api.get(&job_name).await {
                    Ok(_) => {
                        info!("Job '{}' already exists, skipping creation.", job_name);
                    }
                    Err(_) => {
                        let job = create_job_for_task(&wf, task, cm_name)?;
                        job_api.create(&PostParams::default(), &job).await?;
                    }
                }
                current_statuses.insert(task_name.clone(), TASK_RUNNING.to_string());
                made_change = true;
            }
        } else {
            // print all current statuses
            for (task_name, current_status) in &current_statuses {
                println!("Task '{}' depends on '{}'", task_name, current_status);
            }
            println!(
                "task: '{}', status: '{:?}'",
                task_name,
                current_statuses.get(task_name)
            );
        }
    }

    let final_phase = if current_statuses.values().any(|s| s == TASK_FAILED) {
        Some(TASK_FAILED.to_string())
    } else if current_statuses.values().all(|s| s == TASK_SUCCEEDED) {
        Some(TASK_SUCCEEDED.to_string())
    } else {
        Some(TASK_RUNNING.to_string())
    };

    if made_change || wf.status.as_ref().unwrap().phase != final_phase {
        let new_status = QuantumWorkflowStatus {
            phase: final_phase,
            task_statuses: Some(current_statuses),
        };
        update_status(&wf_api, &wf.metadata.name.clone().unwrap(), new_status).await?;
    }

    Ok(Action::requeue(Duration::from_secs(15)))
}

struct Context {
    client: Client,
}

fn on_error(wf: Arc<QuantumWorkflow>, error: &Error, _ctx: Arc<Context>) -> Action {
    warn!(
        "Reconciliation error for '{:?}': {:?}",
        wf.metadata.name, error
    );
    Action::requeue(Duration::from_secs(5))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let client = Client::try_default().await?;
    let context = Arc::new(Context {
        client: client.clone(),
    });

    let workflows = Api::<QuantumWorkflow>::all(client);

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
