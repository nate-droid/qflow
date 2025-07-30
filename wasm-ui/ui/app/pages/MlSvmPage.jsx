import React, { useState } from "react";

export default function MlSvmPage() {
  const [dataFile, setDataFile] = useState(null);
  const [targetColumn, setTargetColumn] = useState("target");
  const [testSize, setTestSize] = useState(0.3);

  const [results, setResults] = useState(null);
  const [error, setError] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  const handleFileChange = (e) => {
    setDataFile(e.target.files[0]);
  };

  const handleRun = async () => {
    if (!dataFile) {
      setError("Please select a CSV data file.");
      return;
    }
    setIsLoading(true);
    setError("");
    setResults(null);

    const formData = new FormData();
    formData.append("data_file", dataFile);
    formData.append("target_column", targetColumn);
    formData.append("test_size", testSize);

    try {
      const response = await fetch("http://localhost:3000/api/ml/svm", {
        method: "POST",
        body: formData,
      });
      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(
          `Server responded with ${response.status}. Message: ${errorText}`,
        );
      }
      const result = await response.json();
      setResults(result);
    } catch (error) {
      setError(error.message);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="p-8">
      <div className="max-w-4xl mx-auto">
        <h2 className="text-2xl font-bold text-white mb-2">
          Run Classical/Quantum SVM
        </h2>
        <p className="text-slate-400 mb-6">
          Upload a CSV, set parameters, and run the workflow.
        </p>

        <div className="space-y-4">
          <div>
            <label htmlFor="ml-data-file" className="block text-sm">
              CSV Data File
            </label>
            <input
              type="file"
              id="ml-data-file"
              onChange={handleFileChange}
              accept=".csv"
              className="bg-slate-800 border border-slate-600 rounded-md py-2 px-3 text-white w-full mt-1 file:mr-4 file:py-2 file:px-4 file:rounded-full file:border-0 file:text-sm file:font-semibold file:bg-indigo-100 file:text-indigo-700 hover:file:bg-indigo-200"
            />
          </div>
          <div>
            <label htmlFor="ml-target-column" className="block text-sm">
              Target Column
            </label>
            <input
              type="text"
              id="ml-target-column"
              value={targetColumn}
              onChange={(e) => setTargetColumn(e.target.value)}
              className="bg-slate-800 border border-slate-600 rounded-md py-2 px-3 text-white w-full mt-1"
            />
          </div>
          <div>
            <label htmlFor="ml-test-size" className="block text-sm">
              Test Size
            </label>
            <input
              type="number"
              id="ml-test-size"
              value={testSize}
              onChange={(e) => setTestSize(parseFloat(e.target.value))}
              step="0.01"
              className="bg-slate-800 border border-slate-600 rounded-md py-2 px-3 text-white w-full mt-1"
            />
          </div>
          <div className="flex justify-end">
            <button
              onClick={handleRun}
              disabled={isLoading}
              className="px-4 py-2 rounded bg-indigo-600 text-white font-semibold hover:bg-indigo-700 disabled:opacity-50 transition-colors"
            >
              {isLoading ? "Running..." : "Run SVM"}
            </button>
          </div>
        </div>

        {error && (
          <div className="mt-6 p-4 rounded-md bg-red-500/20 text-red-300">
            {error}
          </div>
        )}

        {results && (
          <div className="mt-8">
            <h3 className="text-lg font-semibold text-indigo-400 mb-4">
              Results
            </h3>
            <div className="bg-slate-950 rounded-md p-3 text-sm font-code text-slate-300">
              <pre>{results.metrics}</pre>
            </div>
            <img
              id="ml-plot"
              src={`data:image/png;base64,${results.plot_base64}`}
              alt="SVM Decision Boundary"
              className="max-w-full h-auto rounded mt-4 bg-gray-200"
            />
          </div>
        )}
      </div>
    </div>
  );
}
