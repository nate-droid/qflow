use futures_util::StreamExt;
use kube::{
    api::{Api, ListParams, Patch, PatchParams},
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

use qflow_types::{TaskSpec, QuantumWorkflowSpec, QuantumWorkflowStatus, Task, QuantumWorkflow};

// --- 2. Error Handling ---
// A custom error enum for our reconciler logic
#[derive(Error, Debug)]
pub enum Error {
    #[error("Kubernetes API Error: {0}")]
    KubeError(#[from] kube::Error),
    #[error("An error occurred: {0}")]
    Anyhow(#[from] anyhow::Error),
}

// --- 3. The Reconciliation Logic ---
// This is the core of the operator. It's called whenever a QuantumWorkflow
// resource changes.

async fn reconcile(wf: Arc<QuantumWorkflow>, ctx: Arc<Context>) -> Result<Action, Error> {
    println!("reconciling {:?}", wf);
    let client = &ctx.client;

    let ns = wf.meta().namespace.clone().ok_or_else(|| anyhow::anyhow!("Missing namespace"))?;
    let name = wf.meta().name.clone().ok_or_else(|| anyhow::anyhow!("Missing name"))?;
    let api = Api::<QuantumWorkflow>::namespaced(client.clone(), &ns);
    println!("namespace: {}, name: {}", ns, name);

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

    println!("Patched status of QuantumWorkflow '{}' to 'Acknowledged'", name);

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