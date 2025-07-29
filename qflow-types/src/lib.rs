use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "qflow.io",
    version = "v1alpha1",
    kind = "QuantumWorkflow",
    namespaced,
    status = "QuantumWorkflowStatus"
)]
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
    Qcbm(QcbmTaskSpec),
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

/// Defines the desired state of a QuantumSVMWorkflow.
/// This spec is a high-level, declarative interface for running a Quantum SVM experiment.
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(
    group = "upcloud.com",
    version = "v1alpha1",
    kind = "QuantumSVMWorkflow",
    plural = "quantumsvmworkflows",
    namespaced,
    status = "QuantumSVMWorkflowStatus",
    printcolumn = r#"{"name":"Phase","type":"string","jsonPath":".status.phase"}"#,
    printcolumn = r#"{"name":"Age","type":"date","json_path":".metadata.creationTimestamp"}"#
)]
pub struct QuantumSVMWorkflowSpec {
    /// Defines the dataset to be used for the experiment.
    pub dataset: DatasetSpec,

    /// Specifies the container image for the quantum kernel computation.
    pub kernel: KernelSpec,

    /// Configures the classical SVM trainer.
    pub trainer: TrainerSpec,

    /// Defines where to store the output artifacts.
    pub output: OutputSpec,
}

/// Defines the dataset parameters.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct DatasetSpec {
    /// The name of the dataset generator. e.g., "make_moons".
    /// The operator will have built-in logic for this generator.
    pub generator: String,

    #[serde(default = "default_samples")]
    pub samples: u32,

    #[serde(default = "default_noise")]
    pub noise: f64,

    #[serde(default = "default_test_size")]
    pub test_size: f64,
}

/// Specifies the container image containing the custom kernel logic.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct KernelSpec {
    /// The full image path, e.g., "upcloud/quantum-svm:latest".
    pub image: String,
}

/// Configures the classical SVM trainer parameters.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct TrainerSpec {
    #[serde(rename = "svmParameters")]
    pub svm_parameters: SvmParameters,
}

/// Parameters for the scikit-learn SVC.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct SvmParameters {
    /// The regularization parameter C.
    #[serde(rename = "C", default = "default_c_param")]
    pub c: f64,
}

/// Defines the names for the output artifacts.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct OutputSpec {
    /// The filename for the saved model, e.g., "qsvm-model.pkl".
    #[serde(rename = "modelName")]
    pub model_name: String,

    /// The filename for the generated plot, e.g., "decision-boundary.png".
    #[serde(rename = "plotName")]
    pub plot_name: String,
}

/// Represents the observed state of a QuantumSVMWorkflow.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct QuantumSVMWorkflowStatus {
    /// The current phase of the workflow (e.g., GeneratingData, Training, Completed, Failed).
    pub phase: Option<String>,
    /// A human-readable message about the current status.
    pub message: Option<String>,
}

// Default value functions for serde
fn default_samples() -> u32 {
    100
}
fn default_noise() -> f64 {
    0.3
}
fn default_test_size() -> f64 {
    0.3
}
fn default_c_param() -> f64 {
    1.0
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct QcbmTaskSpec {
    pub image: String,
    pub ansatz: String,
    #[serde(rename = "trainingData")]
    pub training_data: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimizer: Option<QcbmOptimizerSpec>,
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

fn default_epochs() -> i32 {
    100
}
fn default_learning_rate() -> f64 {
    0.01
}
