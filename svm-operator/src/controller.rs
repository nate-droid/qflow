// src/controller.rs

use k8s_openapi::api::batch::v1::{Job, JobSpec, JobStatus};
use k8s_openapi::api::core::v1::{
    Container, PersistentVolumeClaim, PersistentVolumeClaimSpec, PersistentVolumeClaimStatus,
    PersistentVolumeClaimVolumeSource, PodSpec, PodTemplateSpec, Volume, VolumeMount,
    VolumeResourceRequirements,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::{
    Api, Client, Resource, ResourceExt,
    api::{Patch, PatchParams, PostParams},
    runtime::controller::Action,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::time::Duration;

use qflow_types::{QuantumSVMWorkflow, QuantumSVMWorkflowStatus};

// Define our custom error type
#[derive(Debug, Error)]
pub enum Error {
    #[error("Kube API Error: {0}")]
    KubeError(#[from] kube::Error),
    #[error("MissingObjectKey: {0}")]
    MissingObjectKey(&'static str),
}

// The context for our reconciler
pub struct Context {
    pub client: Client,
}

impl Context {
    pub fn new(client: Client) -> Self {
        Context { client }
    }
}

/// The main reconciliation function. This is called every time a change
/// is detected on a QuantumSVMWorkflow resource.
pub async fn reconcile(qsvm: Arc<QuantumSVMWorkflow>, ctx: Arc<Context>) -> Result<Action, Error> {
    let ns = qsvm
        .namespace()
        .ok_or(Error::MissingObjectKey(".metadata.namespace"))?;
    let name = qsvm.name_any();
    let client = ctx.client.clone();

    let qsvm_api: Api<QuantumSVMWorkflow> = Api::namespaced(client.clone(), &ns);
    let job_api: Api<Job> = Api::namespaced(client.clone(), &ns);
    let pvc_api: Api<PersistentVolumeClaim> = Api::namespaced(client.clone(), &ns);

    let phase = qsvm
        .status
        .as_ref()
        .and_then(|s| s.phase.clone())
        .unwrap_or_else(|| "Pending".to_string());

    match phase.as_str() {
        "Pending" => {
            println!("Workflow {} starting, creating PVC...", name);
            let pvc_name = format!("{}-pvc", name);
            let pvc = build_pvc(&name, &pvc_name);
            pvc_api.create(&PostParams::default(), &pvc).await?;

            update_status(
                &qsvm_api,
                &name,
                "CreatingVolume",
                "PersistentVolumeClaim created",
            )
            .await?;
            Ok(Action::requeue(Duration::from_secs(5)))
        }
        "CreatingVolume" => {
            let pvc_name = format!("{}-pvc", name);
            let pvc = pvc_api.get(&pvc_name).await?;
            if let Some(status) = pvc.status {
                if let Some(pvc_phase) = status.phase {
                    if pvc_phase == "Bound" {
                        println!("PVC {} is Bound, creating data generation job...", pvc_name);
                        let job = build_data_gen_job(&qsvm, &pvc_name)?;
                        job_api.create(&PostParams::default(), &job).await?;
                        update_status(
                            &qsvm_api,
                            &name,
                            "GeneratingData",
                            "Data generation job started",
                        )
                        .await?;
                        return Ok(Action::requeue(Duration::from_secs(10)));
                    }
                }
            }
            println!("Waiting for PVC {} to be bound...", pvc_name);
            Ok(Action::requeue(Duration::from_secs(5)))
        }
        "GeneratingData" => {
            let job_name = format!("{}-datagen", name);
            let job = job_api.get(&job_name).await?;
            if let Some(status) = job.status {
                if status.succeeded.unwrap_or(0) > 0 {
                    println!("Data generation job {} succeeded.", job_name);
                    // TODO: Create the second Kubernetes Job to run the main SVM experiment.
                    update_status(
                        &qsvm_api,
                        &name,
                        "TrainingModel",
                        "Data generation complete, starting training.",
                    )
                    .await?;
                    return Ok(Action::requeue(Duration::from_secs(10)));
                } else if status.failed.unwrap_or(0) > 0 {
                    println!("Data generation job {} failed.", job_name);
                    update_status(&qsvm_api, &name, "Failed", "Data generation job failed.")
                        .await?;
                    return Ok(Action::await_change());
                }
            }
            println!(
                "Waiting for data generation job {} to complete...",
                job_name
            );
            Ok(Action::requeue(Duration::from_secs(10)))
        }
        "TrainingModel" => {
            // TODO: Check if the training Job has completed.
            println!("Workflow {} is in TrainingModel phase.", name);
            Ok(Action::requeue(Duration::from_secs(10)))
        }
        "Completed" | "Failed" => {
            // Workflow is in a terminal state, do nothing.
            Ok(Action::await_change())
        }
        _ => Ok(Action::requeue(Duration::from_secs(10))),
    }
}

/// Helper function to build the PersistentVolumeClaim
fn build_pvc(owner_name: &str, pvc_name: &str) -> PersistentVolumeClaim {
    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), owner_name.to_string());

    PersistentVolumeClaim {
        metadata: ObjectMeta {
            name: Some(pvc_name.to_string()),
            labels: Some(labels),
            ..Default::default()
        },
        spec: Some(PersistentVolumeClaimSpec {
            access_modes: Some(vec!["ReadWriteOnce".to_string()]),
            resources: Some(VolumeResourceRequirements {
                requests: Some(BTreeMap::from([(
                    "storage".to_string(),
                    Quantity("1Gi".to_string()),
                )])),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Helper function to build the data generation Job
fn build_data_gen_job(qsvm: &QuantumSVMWorkflow, pvc_name: &str) -> Result<Job, Error> {
    let name = qsvm.name_any();
    let job_name = format!("{}-datagen", name);
    let mount_path = "/data";

    let command = vec![
        "sh".to_string(),
        "-c".to_string(),
        format!(
            "pip install numpy scikit-learn && python -c 'import numpy as np; from sklearn.datasets import make_moons; X, y = make_moons(n_samples={}, noise={}); np.save(\"{}/X.npy\", X); np.save(\"{}/y.npy\", y)'",
            qsvm.spec.dataset.samples, qsvm.spec.dataset.noise, mount_path, mount_path
        ),
    ];

    let job = Job {
        metadata: ObjectMeta {
            name: Some(job_name),
            ..Default::default()
        },
        spec: Some(JobSpec {
            template: PodTemplateSpec {
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: "data-generator".to_string(),
                        image: Some("python:3.9-slim".to_string()),
                        command: Some(command),
                        volume_mounts: Some(vec![VolumeMount {
                            name: "workdir".to_string(),
                            read_only: None,
                            recursive_read_only: None,
                            sub_path: None,
                            mount_path: mount_path.to_string(),
                            mount_propagation: None,
                            sub_path_expr: None,
                        }]),
                        ..Default::default()
                    }],
                    restart_policy: Some("Never".to_string()),
                    volumes: Some(vec![Volume {
                        name: "workdir".to_string(),
                        persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
                            claim_name: pvc_name.to_string(),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            backoff_limit: Some(2),
            ..Default::default()
        }),
        ..Default::default()
    };
    Ok(job)
}

/// Helper function to update the status of the QuantumSVMWorkflow resource
async fn update_status(
    api: &Api<QuantumSVMWorkflow>,
    name: &str,
    phase: &str,
    message: &str,
) -> Result<(), Error> {
    let new_status = Patch::Apply(json!({
        "apiVersion": "upcloud.com/v1alpha1",
        "kind": "QuantumSVMWorkflow",
        "status": QuantumSVMWorkflowStatus {
            phase: Some(phase.to_string()),
            message: Some(message.to_string()),
        }
    }));
    let ps = PatchParams::apply("qsvm-operator.upcloud.com");
    api.patch_status(name, &ps, &new_status).await?;
    Ok(())
}

/// The error policy for the controller. This is called when the `reconcile`
/// function returns an error.
pub fn error_policy(_qsvm: Arc<QuantumSVMWorkflow>, error: &Error, _ctx: Arc<Context>) -> Action {
    println!("Reconciliation error: {:?}", error);
    // For now, we just requeue after a short delay on any error.
    Action::requeue(Duration::from_secs(5))
}
