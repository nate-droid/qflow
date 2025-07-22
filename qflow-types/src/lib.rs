use std::collections::BTreeMap;
use kube::CustomResource;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(group = "qflow.io", version = "v1alpha1", kind = "QuantumWorkflow", namespaced, status = "QuantumWorkflowStatus")]
#[serde(rename_all = "camelCase")]
pub struct QuantumWorkflowSpec {
    pub volume: Option<VolumeSpec>,
    pub tasks: Vec<QFlowTask>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct VolumeSpec {
    pub size: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QFlowTask {
    pub name: String,
    #[serde(rename = "dependsOn")]
    pub depends_on: Option<Vec<String>>,
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
        circuit: String,
        params: String,
    },
}

impl Default for QFlowTaskSpec {
    fn default() -> Self {
        QFlowTaskSpec::Classical {
            image: String::new(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuantumWorkflowStatus {
    pub phase: Option<String>,
    pub task_statuses: Option<BTreeMap<String, String>>,
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