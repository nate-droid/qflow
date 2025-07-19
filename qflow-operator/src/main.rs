use std::collections::BTreeMap;
use futures_util::StreamExt;
use kube::{
    api::{Api, ListParams, Patch, PatchParams, PostParams},
    client::Client,
    runtime::{controller::Action, Controller},
    CustomResource, Resource,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::time::Duration;
use tracing::{info, warn};
use schemars::JsonSchema;
use serde_json::json;

use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::api::core::v1::{PodTemplateSpec, PodSpec, Container, ConfigMap, Volume, VolumeMount, ConfigMapVolumeSource};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use qflow_types::{QFlowTaskSpec};
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
}

fn create_job_for_workflow(wf: &QuantumWorkflow) -> Result<Job, Error> {
    let task = wf.spec.tasks.get(0).ok_or(Error::MissingObjectKey("tasks"))?;
    let wf_name = wf.metadata.name.clone().unwrap();
    let cm_name = wf.metadata.name.clone().unwrap_or_else(|| "default".to_string());

    let (image, volumes, volume_mounts) = match &task.spec {
        QFlowTaskSpec::Classical { image } => (image.clone(), None, None),
        QFlowTaskSpec::Quantum { image, .. } => {
            let vol = Volume {
                name: "qflow-input".to_string(),
                config_map: Some(ConfigMapVolumeSource { name: cm_name, ..Default::default() }),
                ..Default::default()
            };
            let mount = VolumeMount {
                name: "qflow-input".to_string(),
                mount_path: "/workspace/input".to_string(),
                read_only: Some(true),
                recursive_read_only: None,
                sub_path: None,
                mount_propagation: None,
                sub_path_expr: None,
            };
            (image.clone(), Some(vec![vol]), Some(vec![mount]))
        }
    };

    let job_name = format!("{}-{}", wf_name, task.name);

    // match if it is a Quantum task

    let job = Job {
        metadata: ObjectMeta {
            name: Some(job_name),
            namespace: wf.metadata.namespace.clone(),
            // Set the QuantumWorkflow as the owner of this Job.
            // This ensures the Job is garbage collected when the QuantumWorkflow is deleted.
            owner_references: Some(vec![wf.controller_owner_ref(&()).unwrap()]),
            ..Default::default()
        },
        spec: Some(JobSpec {
            template: PodTemplateSpec {
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: "task-runner".to_string(),
                        image: Some(image.clone()),
                        image_pull_policy: Some("Never".into()),
                        ..Default::default()
                    }],
                    restart_policy: Some("Never".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            backoff_limit: Some(4),
            ..Default::default()
        }),
        ..Default::default()
    };
    Ok(job)
}

/// Patches the status of the QuantumWorkflow resource with a new phase.
async fn update_workflow_phase(api: &Api<QuantumWorkflow>, name: &str, phase: &str) -> Result<(), Error> {
    let new_status = json!({
        "status": {
            "phase": "Acknowledged"
        }
    });
    let status_patch = Patch::Merge(&new_status);
    let patch_params = PatchParams::apply("qflow-controller");
    api.patch_status(&name, &patch_params, &status_patch)
        .await
        .map_err(Error::KubeError)?;

    // let new_status = Patch::Apply(serde_json::json!({
    //     "apiVersion": "qflow.io/v1alpha1", // Replace with your CRD's actual apiVersion
    //     "kind": "QuantumWorkflow",
    //     "status": {
    //         "phase": phase
    //     }
    // }));
    // let ps = PatchParams::apply("qflow-operator").force();
    // api.patch_status(name, &ps, &new_status).await?;
    Ok(())
}

async fn create_config_map(client: Client, wf: &QuantumWorkflow, circuit: &str, params: &str) -> Result<String, Error> {
    let wf_name = wf.metadata.name.clone().unwrap();
    let ns = wf.metadata.namespace.clone().unwrap();
    let cm_name = format!("{}-cm", wf_name);
    let mut data = BTreeMap::new();
    data.insert("circuit.qasm".to_string(), circuit.to_string());
    data.insert("params.json".to_string(), params.to_string());

    let cm = ConfigMap {
        metadata: ObjectMeta {
            name: Some(cm_name.clone()),
            owner_references: Some(vec![wf.controller_owner_ref(&()).unwrap()]),
            ..Default::default()
        },
        data: Some(data),
        ..Default::default()
    };

    let cm_api = Api::<ConfigMap>::namespaced(client, &ns);
    cm_api.create(&PostParams::default(), &cm).await?;
    Ok(cm_name)
}

// --- 3. The Reconciliation Logic ---
// This is the core of the operator. It's called whenever a QuantumWorkflow
// resource changes.

async fn reconcile(wf: Arc<QuantumWorkflow>, ctx: Arc<Context>) -> Result<Action, Error> {
    // println!("reconciling {:?}", wf);
    let client = &ctx.client;

    let ns = wf.meta().namespace.clone().ok_or_else(|| anyhow::anyhow!("Missing namespace"))?;
    let wf_name = wf.meta().name.clone().ok_or_else(|| anyhow::anyhow!("Missing name"))?;
    let wf_api = Api::<QuantumWorkflow>::namespaced(client.clone(), &ns);
    println!("namespace: {}, name: {}", ns, wf_name);

    let job_api = Api::<Job>::namespaced(client.clone(), &ns);
    let current_phase = wf.status.as_ref().and_then(|s| s.phase.as_deref());

    match current_phase {
        None => {
            info!("Phase is empty, creating Job for '{}'", wf_name);
            let task = wf.spec.tasks.get(0).ok_or(Error::MissingObjectKey("tasks"))?;
            let cm_name = if let QFlowTaskSpec::Quantum { circuit, params, .. } = &task.spec {
                Some(create_config_map(client.clone(), &wf, circuit, params).await?)
            } else { None };

            let job = create_job_for_workflow(&wf)?;
            job_api.create(&PostParams::default(), &job).await?;
            update_workflow_phase(&wf_api, &wf_name, "Running").await?;
            info!("Job created, workflow '{}' phase is now Running", wf_name);
        }
        Some("Running") => {
            let task_name = &wf.spec.tasks[0].name;
            let job_name = format!("{}-{}", wf_name, task_name);

            match job_api.get_status(&job_name).await {
                Ok(job) => {
                    if let Some(status) = job.status {
                        if status.succeeded.unwrap_or(0) > 0 {
                            info!("Job '{}' succeeded.", job_name);
                            update_workflow_phase(&wf_api, &wf_name, "Succeeded").await?;
                        } else if status.failed.unwrap_or(0) > 0 {
                            info!("Job '{}' failed.", job_name);
                            update_workflow_phase(&wf_api, &wf_name, "Failed").await?;
                        }
                    }
                }
                Err(kube::Error::Api(e)) if e.code == 404 => {
                    warn!("Job '{}' not found for running workflow. Re-creating.", job_name);
                    let job = create_job_for_workflow(&wf)?;
                    job_api.create(&PostParams::default(), &job).await?;
                }
                Err(e) => return Err(Error::KubeError(e)),
            }
        }
        Some("Succeeded") | Some("Failed") => {
            // Terminal state, do nothing.
        }
        _ => {
            // Unknown state, maybe patch to Pending or Failed.
            warn!("Unknown phase '{:?}' for workflow '{}'. Setting to Pending.", current_phase, wf_name);
            update_workflow_phase(&wf_api, &wf_name, "Pending").await?;
        }
    }

    // Ok(Action::requeue(Duration::from_secs(30)));

    // let new_status = json!({
    //     "status": {
    //         "phase": "Acknowledged"
    //     }
    // });
    // let status_patch = Patch::Merge(&new_status);
    // let patch_params = PatchParams::apply("qflow-controller");
    // api.patch_status(&name, &patch_params, &status_patch)
    //     .await
    //     .map_err(Error::KubeError)?;

    println!("Patched status of QuantumWorkflow '{}' to 'Acknowledged'", wf_name);

    Ok(Action::requeue(Duration::from_secs(600)))
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