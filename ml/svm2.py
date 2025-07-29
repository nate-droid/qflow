import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from sklearn.svm import SVC
from sklearn.model_selection import train_test_split
from sklearn.preprocessing import StandardScaler
from sklearn.datasets import make_moons, make_circles, make_blobs
import argparse
import os

# Quantum kernel import/fallback
try:
    from quantum_kernel_lib import quantum_kernel
except ImportError:
    print("Warning: 'quantum_kernel_lib' not found. Using a dummy kernel for demonstration.")
    def quantum_kernel(x1, x2, gamma=0.5):
        return np.exp(-gamma * np.linalg.norm(x1 - x2) ** 2)

def load_data(path, target_column):
    if not os.path.exists(path):
        raise FileNotFoundError(f"Data file not found at {path}")
    df = pd.read_csv(path)
    if target_column not in df.columns:
        raise KeyError(f"Target column '{target_column}' not found in the data file.")
    X = df.drop(columns=[target_column]).values
    y = df[target_column].values
    return X, y

def create_dummy_dataset(file_path, dataset_type='moons', n_samples=200, noise=0.25):
    print(f"Creating a '{dataset_type}' dummy dataset with {n_samples} samples at '{file_path}'...")
    if dataset_type == 'moons':
        X, y = make_moons(n_samples=n_samples, noise=noise, random_state=42)
    elif dataset_type == 'circles':
        circle_noise = noise / 2.0
        X, y = make_circles(n_samples=n_samples, noise=circle_noise, factor=0.5, random_state=42)
    elif dataset_type == 'blobs':
        X, y = make_blobs(n_samples=n_samples, centers=2, random_state=42, cluster_std=1.0 + noise)
    else:
        raise ValueError(f"Unknown dataset_type: {dataset_type}")
    df = pd.DataFrame(X, columns=['feature1', 'feature2'])
    df['target'] = y
    df.to_csv(file_path, index=False)
    print("Dummy dataset created successfully.")

def preprocess_data(X):
    scaler = StandardScaler()
    X_scaled = scaler.fit_transform(X)
    return X_scaled, scaler

def compute_gram_matrix(X1, X2):
    gram_matrix = np.zeros((X1.shape[0], X2.shape[0]))
    for i, x1 in enumerate(X1):
        for j, x2 in enumerate(X2):
            gram_matrix[i, j] = quantum_kernel(x1, x2)
    return gram_matrix

def train_svm(gram_train, y_train):
    svm = SVC(kernel='precomputed', probability=True)
    svm.fit(gram_train, y_train)
    return svm

def evaluate_model(svm, X_test, X_train, y_test):
    gram_test_eval = compute_gram_matrix(X_test, X_train)
    y_pred = svm.predict(gram_test_eval)
    accuracy = np.mean(y_pred == y_test)
    return accuracy, y_pred

def plot_decision_boundary(X_train, y_train, X_test, y_test, svm, scaler, output_plot):
    h = .05
    x_min, x_max = X_train[:, 0].min() - .5, X_train[:, 0].max() + .5
    y_min, y_max = X_train[:, 1].min() - .5, X_train[:, 1].max() + .5
    xx, yy = np.meshgrid(np.arange(x_min, x_max, h),
                         np.arange(y_min, y_max, h))
    mesh_data = np.c_[xx.ravel(), yy.ravel()]
    gram_mesh = compute_gram_matrix(mesh_data, X_train)
    Z = svm.decision_function(gram_mesh)
    Z = Z.reshape(xx.shape)
    plt.figure(figsize=(12, 10))
    plt.contourf(xx, yy, Z, cmap=plt.cm.coolwarm, alpha=0.8)
    plt.scatter(X_train[:, 0], X_train[:, 1], c=y_train, cmap=plt.cm.coolwarm, edgecolors='k', label='Training Points')
    plt.scatter(X_test[:, 0], X_test[:, 1], c=y_test, cmap=plt.cm.coolwarm, edgecolors='grey', marker='s', label='Test Points')
    plt.scatter(X_train[svm.support_, 0], X_train[svm.support_, 1], s=150, facecolors='none', edgecolors='k', linewidths=2, label='Support Vectors')
    plt.title('Quantum Kernel SVM Decision Boundary')
    plt.xlabel('Feature 1 (Standardized)')
    plt.ylabel('Feature 2 (Standardized)')
    plt.legend()
    plt.savefig(output_plot, dpi=300)
    plt.close()

def save_metrics(path, accuracy, n_support_vectors):
    with open(path, 'w') as f:
        f.write(f"Test Set Accuracy: {accuracy:.4f}\n")
        f.write(f"Number of Support Vectors: {n_support_vectors}\n")

def parse_args():
    parser = argparse.ArgumentParser(description="Run an SVM with a precomputed quantum kernel.",
                                     formatter_class=argparse.RawTextHelpFormatter)
    dummy_group = parser.add_argument_group('Dummy Data Generation')
    dummy_group.add_argument('--create-dummy-data', type=str, metavar='FILE_PATH',
                            help='If specified, creates a dummy CSV dataset at this path and exits.')
    dummy_group.add_argument('--dummy-type', type=str, default='moons', choices=['moons', 'circles', 'blobs'],
                            help='Type of dummy dataset to generate.\n(default: moons)')
    workflow_group = parser.add_argument_group('Main Workflow Arguments')
    workflow_group.add_argument('--data_path', type=str,
                            help='Path to the input data file (e.g., a CSV). Required if not creating dummy data.')
    workflow_group.add_argument('--target-column', type=str,
                            help='Name of the target variable column in the data file. Required if not creating dummy data.')
    workflow_group.add_argument('--output-plot', type=str, default=None,
                            help='Path to save the output plot image. Plot is only generated for 2D data.')
    workflow_group.add_argument('--output-metrics', type=str, default=None,
                            help='Path to save the output metrics file.')
    workflow_group.add_argument('--test-size', type=float, default=0.3,
                            help='Proportion of the dataset to allocate to the test split.')
    workflow_group.add_argument('--random-state', type=int, default=42,
                            help='Random seed for reproducibility of the train/test split.')
    workflow_group.add_argument('--server', action='store_true',
                            help='If set, keeps the process alive for container exec access.')
    return parser.parse_args()

def main(args):
    if args.create_dummy_data:
        create_dummy_dataset(args.create_dummy_data, args.dummy_type)
        return
    if not (args.data_path and args.target_column):
        print("Error: Either --create-dummy-data or both --data_path and --target-column must be specified.")
        return

    X, y = load_data(args.data_path, args.target_column)
    X_scaled, scaler = preprocess_data(X)
    X_train, X_test, y_train, y_test = train_test_split(
        X_scaled, y, test_size=args.test_size, random_state=args.random_state
    )
    gram_train = compute_gram_matrix(X_train, X_train)
    svm = train_svm(gram_train, y_train)
    accuracy, y_pred = evaluate_model(svm, X_test, X_train, y_test)
    print(f"Accuracy on the test set: {accuracy:.4f}")

    if args.output_metrics:
        save_metrics(args.output_metrics, accuracy, len(svm.support_))
        print(f"Metrics saved to: {args.output_metrics}")

    if X.shape[1] == 2 and args.output_plot:
        plot_decision_boundary(X_train, y_train, X_test, y_test, svm, scaler, args.output_plot)
        print(f"Plot saved to: {args.output_plot}")
    elif args.output_plot:
        print("Skipping plot: Input data is not 2-dimensional.")

    if args.server:
        import time
        print("Server mode enabled. Keeping process alive. Press Ctrl+C to exit.")
        try:
            while True:
                time.sleep(600)
        except KeyboardInterrupt:
            print("Exiting server mode.")

if __name__ == '__main__':
    args = parse_args()
    main(args)
