import React, { useState } from "react";

// Notification component with Tailwind styling
const Notification = ({ message, type }) => {
  if (!message) return null;
  const baseClasses = "p-4 mb-4 rounded-md";
  const typeClasses =
    type === "success"
      ? "bg-green-500/20 text-green-300"
      : "bg-red-500/20 text-red-300";
  return <div className={`${baseClasses} ${typeClasses}`}>{message}</div>;
};

export default function QasmSubmitterPage() {
  const [workflowName, setWorkflowName] = useState("my-bell-state-exp");
  const [namespace, setNamespace] = useState("default");
  const [qasmCode, setQasmCode] = useState(
    'OPENQASM 2.0;\ninclude "qelib1.inc";\n\nqreg q[2];\ncreg c[2];\n\nh q[0];\ncx q[0], q[1];\nmeasure q -> c;',
  );
  const [notification, setNotification] = useState({ message: "", type: "" });
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmit = async () => {
    setIsSubmitting(true);
    setNotification({ message: "", type: "" });

    // Simulate API call
    setTimeout(() => {
      setNotification({
        message: "Workflow submitted successfully!",
        type: "success",
      });
      setIsSubmitting(false);
    }, 1500);
  };

  return (
    <div className="bg-slate-900 text-white min-h-screen p-8">
      <div className="max-w-4xl mx-auto rounded-lg shadow-lg bg-slate-800/80 border border-slate-700 p-8">
        <h2 className="text-2xl font-bold mb-2">Submit OpenQASM Workflow</h2>
        <p className="text-slate-400 mb-6">
          Paste your OpenQASM 2.0 code below to create and run a new workflow.
        </p>

        <Notification message={notification.message} type={notification.type} />

        <div className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label
                htmlFor="qasm-workflow-name"
                className="block text-sm font-medium text-slate-300 mb-1"
              >
                Workflow Name
              </label>
              <input
                type="text"
                id="qasm-workflow-name"
                value={workflowName}
                onChange={(e) => setWorkflowName(e.target.value)}
                className="bg-slate-900 border border-slate-600 rounded-md py-2 px-3 text-white w-full"
              />
            </div>
            <div>
              <label
                htmlFor="qasm-namespace"
                className="block text-sm font-medium text-slate-300 mb-1"
              >
                Namespace
              </label>
              <input
                type="text"
                id="qasm-namespace"
                value={namespace}
                onChange={(e) => setNamespace(e.target.value)}
                className="bg-slate-900 border border-slate-600 rounded-md py-2 px-3 text-white w-full"
              />
            </div>
          </div>
          <div>
            <label
              htmlFor="qasm-input"
              className="block text-sm font-medium text-slate-300 mb-1"
            >
              OpenQASM 2.0 Code
            </label>
            <textarea
              id="qasm-input"
              rows="15"
              value={qasmCode}
              onChange={(e) => setQasmCode(e.target.value)}
              className="w-full bg-slate-950 border border-slate-600 rounded-md p-3 font-mono text-indigo-300"
            ></textarea>
          </div>
          <div className="flex justify-end">
            <button
              onClick={handleSubmit}
              disabled={isSubmitting}
              className="py-2 px-4 bg-indigo-600 text-white rounded-md hover:bg-indigo-700 font-semibold disabled:opacity-50"
            >
              {isSubmitting ? "Submitting..." : "Submit Workflow"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
