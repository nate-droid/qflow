import numpy as np
import matplotlib.pyplot as plt
from sklearn.datasets import make_moons
from sklearn.svm import SVC
from sklearn.model_selection import train_test_split
from quantum_kernel_lib import quantum_kernel

# Generate a dataset
X, y = make_moons(n_samples=100, noise=0.3, random_state=42)
X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.3, random_state=42)

# Build the Gram matrix using the Rust quantum kernel
def compute_gram_matrix(X1, X2):
    gram_matrix = np.zeros((X1.shape[0], X2.shape[0]))
    for i, x1 in enumerate(X1):
        for j, x2 in enumerate(X2):
            gram_matrix[i, j] = quantum_kernel(x1, x2)
    return gram_matrix

print("Computing the Gram matrix for the training data...")
gram_train = compute_gram_matrix(X_train, X_train)

# Train the SVM with the precomputed kernel
print("Training the SVM...")
svm = SVC(kernel='precomputed')
svm.fit(gram_train, y_train)

# Create a mesh to plot the decision boundary
h = .02  # step size in the mesh
x_min, x_max = X[:, 0].min() - .5, X[:, 0].max() + .5
y_min, y_max = X[:, 1].min() - .5, X[:, 1].max() + .5
xx, yy = np.meshgrid(np.arange(x_min, x_max, h),
                     np.arange(y_min, y_max, h))

# To plot the decision boundary, we need to compute the kernel
# between each point in the mesh and the support vectors.
print("Computing the decision boundary...")
mesh_data = np.c_[xx.ravel(), yy.ravel()]
gram_test = compute_gram_matrix(mesh_data, X_train[svm.support_])

# Make predictions on the mesh
Z = svm.decision_function(gram_test)
Z = Z.reshape(xx.shape)

# Plot the results
print("Plotting the results...")
plt.figure(figsize=(10, 8))
plt.contourf(xx, yy, Z, cmap=plt.cm.coolwarm, alpha=0.8)
plt.scatter(X_train[:, 0], X_train[:, 1], c=y_train, cmap=plt.cm.coolwarm, edgecolors='k')
plt.scatter(svm.support_vectors_[:, 0], svm.support_vectors_[:, 1],
            s=100, facecolors='none', edgecolors='k', linewidths=2)
plt.title('Quantum Kernel SVM Decision Boundary')
plt.xlabel('Feature 1')
plt.ylabel('Feature 2')
plt.show()

# Evaluate the model on the test set
print("\nEvaluating the model on the test set...")
gram_test_eval = compute_gram_matrix(X_test, X_train)
y_pred = svm.predict(gram_test_eval)
accuracy = np.mean(y_pred == y_test)
print(f"Accuracy on the test set: {accuracy:.4f}")
