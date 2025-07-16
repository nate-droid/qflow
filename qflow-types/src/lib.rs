use kube::CustomResource;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// The top-level Custom Resource for a QuantumWorkflow.
/// This is the struct that will be serialized to/from YAML.


#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(group = "qflow.io", version = "v1alpha1", kind = "QuantumWorkflow", namespaced, status = "QuantumWorkflowStatus")]
#[serde(rename_all = "camelCase")]
pub struct QuantumWorkflowSpec {
    pub tasks: Vec<Task>,
}

/// Represents a single task in the workflow.
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct Task {
    pub name: String,
    pub image: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TaskSpec {
    pub name: String,
    pub image: String,
}

/// Represents the observed state of a QuantumWorkflow.
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuantumWorkflowStatus {
    pub phase: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QuantumWorkflowResource {
    pub api_version: String,
    pub kind: String,
    pub metadata: Metadata,
    pub spec: QuantumWorkflowSpec,
}

#[derive(Serialize, Debug)]
pub struct Metadata {
    pub name: String,
}