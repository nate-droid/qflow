use kube::CustomResource;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// The top-level Custom Resource for a QuantumWorkflow.
/// This is the struct that will be serialized to/from YAML.


#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(group = "qflow.io", version = "v1alpha1", kind = "QuantumWorkflow", namespaced, status = "QuantumWorkflowStatus")]
#[serde(rename_all = "camelCase")]
pub struct QuantumWorkflowSpec {
    pub tasks: Vec<QFlowTask>,
}

/// Represents a single task in the workflow.
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct QFlowTask {
    pub name: String,
    #[serde(flatten)]
    pub spec: QFlowTaskSpec,
}

#[derive(Serialize, Debug, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum QFlowTaskSpec {
    Classical {
        image: String,
    },
    Quantum {
        image: String,
        circuit: String, // The full QASM circuit as a string
        params: String,  // The full parameters JSON as a string
    },
}

impl Default for QFlowTaskSpec {
    fn default() -> Self {
        QFlowTaskSpec::Classical {
            image: String::new(),
        }
    }
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