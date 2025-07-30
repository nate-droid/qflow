import React, {
  useState,
  useCallback,
  useMemo,
  useEffect,
  useRef,
} from "react";
import { Routes, Route, NavLink, Outlet, useNavigate } from "react-router-dom";
import cytoscape from "cytoscape";
import cytoscapeDagre from "cytoscape-dagre";
import VisualizerPage from "./pages/VisualizerPage";
// NOTE: The 'cytoscape-dagre' library is expected to be loaded globally in this environment.
// The direct import is removed to resolve a build error where the module cannot be found.
// The registration of the dagre layout is assumed to be handled by the global script.
// Make sure to run: npm install cytoscape dagre d3

cytoscape.use(cytoscapeDagre);

//================================================================================
// --- HELPER & REUSABLE COMPONENTS ---
//================================================================================

const Notification = ({ message, type }) => {
  if (!message) return null;
  const style =
    type === "success"
      ? "bg-green-500/20 text-green-300"
      : "bg-red-500/20 text-red-300";
  return <div className={`p-4 my-4 rounded-md ${style}`}>{message}</div>;
};

const StatusDisplay = ({ statusText }) => (
  <div className="text-slate-400 flex items-center gap-3">
    <svg
      className="animate-spin h-5 w-5 text-indigo-400"
      xmlns="http://www.w3.org/2000/svg"
      fill="none"
      viewBox="0 0 24 24"
    >
      <circle
        className="opacity-25"
        cx="12"
        cy="12"
        r="10"
        stroke="currentColor"
        strokeWidth="4"
      ></circle>
      <path
        className="opacity-75"
        fill="currentColor"
        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
      ></path>
    </svg>
    <span className="font-medium">{statusText}</span>
  </div>
);

// This component ensures its children are only rendered on the client side.
const ClientOnly = ({ children }) => {
  const [hasMounted, setHasMounted] = useState(false);

  useEffect(() => {
    setHasMounted(true);
  }, []);

  if (!hasMounted) {
    return null; // Or a loading spinner
  }

  return children;
};

//================================================================================
// --- VISUALIZER PAGE COMPONENTS ---
//================================================================================

const CytoscapeGraph = ({ elements, onNodeTap, style }) => {
  const containerRef = useRef(null);

  useEffect(() => {
    if (!containerRef.current) return;

    const cy = cytoscape({
      container: containerRef.current,
      elements: elements,
      style: [
        {
          selector: "node",
          style: {
            "background-color": "#1e293b",
            "border-color": "#475569",
            "border-width": 2,
            label: "data(label)",
            color: "#e2e8f0",
            "font-size": "12px",
            "text-valign": "center",
            "text-halign": "center",
            width: "140px",
            height: "50px",
            shape: "round-rectangle",
            "transition-property": "background-color, border-color",
            "transition-duration": "0.3s",
          },
        },
        {
          selector: "edge",
          style: {
            width: 2,
            "line-color": "#64748b",
            "target-arrow-color": "#64748b",
            "target-arrow-shape": "triangle",
            "curve-style": "bezier",
          },
        },
        {
          selector: "node:selected",
          style: { "border-color": "#818cf8", "background-color": "#312e81" },
        },
        { selector: ".status-succeeded", style: { "border-color": "#22c55e" } },
        {
          selector: ".status-running",
          style: {
            "border-color": "#f59e0b",
            "line-style": "dashed",
            "border-dash-pattern": [6, 3],
          },
        },
        { selector: ".status-failed", style: { "border-color": "#ef4444" } },
        { selector: ".status-pending", style: { "border-color": "#64748b" } },
      ],
      layout: { name: "dagre", padding: 32 },
    });

    const handleTap = (evt) => {
      if (onNodeTap) onNodeTap(evt.target.data());
    };
    cy.on("tap", "node", handleTap);

    return () => {
      cy.removeListener("tap", "node", handleTap);
      cy.destroy();
    };
  }, [elements, onNodeTap]);

  return <div ref={containerRef} style={style} />;
};

const DetailsPanel = ({ node, onShowResults }) => {
  if (!node) {
    return (
      <div className="text-center text-slate-400 pt-10">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          className="mx-auto h-10 w-10 text-slate-500"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          strokeWidth="1.5"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            d="M9.879 7.519c1.171-1.025 3.071-1.025 4.242 0 1.172 1.025 1.172 2.687 0 3.712-.203.179-.43.326-.67.442-.745.361-1.45.999-1.45 1.827v.75M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-9 5.25h.008v.008H12v-.008z"
          />
        </svg>
        <p className="mt-2">Select a task node to view its details.</p>
      </div>
    );
  }
  return (
    <div className="space-y-5">
      <div className="bg-slate-900/50 p-4 rounded-lg border border-slate-700">
        <h3 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">
          Status
        </h3>
        <p className="font-code text-sm font-semibold">{node.status}</p>
      </div>
      {node.circuit && (
        <div className="bg-slate-900/50 p-4 rounded-lg border border-slate-700">
          <h3 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">
            Circuit (OpenQASM)
          </h3>
          <pre className="bg-slate-950 rounded-md p-3 text-sm font-code text-indigo-300 overflow-x-auto">
            <code>{node.circuit}</code>
          </pre>
        </div>
      )}
      <div className="bg-slate-900/50 p-4 rounded-lg border border-slate-700">
        <h3 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">
          Parameters
        </h3>
        <pre className="bg-slate-950 rounded-md p-3 text-sm font-code text-amber-300 overflow-x-auto">
          <code>{JSON.stringify(node.params, null, 2)}</code>
        </pre>
      </div>
      {node.status === "Succeeded" && (
        <button
          onClick={() => onShowResults(node)}
          className="w-full py-2 bg-green-600 rounded-md hover:bg-green-700 transition-colors font-semibold"
        >
          View Results
        </button>
      )}
    </div>
  );
};

//================================================================================
// --- CIRCUIT SIMULATOR COMPONENTS ---
//================================================================================

const GateIcon = ({ gate }) => {
  const gateStyles = {
    H: "bg-yellow-500 border-yellow-600",
    X: "bg-red-500 border-red-600",
    Y: "bg-green-500 border-green-600",
    Z: "bg-blue-500 border-blue-600",
  };
  const style = gateStyles[gate] || "bg-gray-400 border-gray-500";
  return (
    <div
      className={`w-8 h-8 rounded-md flex items-center justify-center text-white font-bold text-sm border-b-2 ${style}`}
    >
      {gate}
    </div>
  );
};

const CnotControl = () => (
  <div className="w-8 h-8 flex items-center justify-center">
    <div className="w-4 h-4 bg-blue-500 rounded-full border-2 border-blue-300"></div>
  </div>
);

const CnotTarget = () => (
  <div className="w-8 h-8 flex items-center justify-center relative">
    <div className="w-8 h-8 border-2 border-blue-500 rounded-full flex items-center justify-center">
      <div className="w-0.5 h-5 bg-blue-500 absolute"></div>
      <div className="w-5 h-0.5 bg-blue-500 absolute"></div>
    </div>
  </div>
);

const PhaseDisk = ({ re, im, size = 24 }) => {
  const probability = re * re + im * im;
  if (probability < 1e-6)
    return (
      <div style={{ width: size, height: size }} className="flex-shrink-0" />
    );
  const angle = Math.atan2(im, re) * (180 / Math.PI);
  return (
    <div className="flex-shrink-0" title={`Phase: ${angle.toFixed(1)}°`}>
      <svg
        width={size}
        height={size}
        viewBox="0 0 24 24"
        className="text-teal-400"
      >
        <circle
          cx="12"
          cy="12"
          r="10"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          opacity="0.3"
        />
        <line
          x1="12"
          y1="12"
          x2="22"
          y2="12"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          transform={`rotate(${angle} 12 12)`}
        />
      </svg>
    </div>
  );
};

const GatePalette = ({
  selectedGate,
  setSelectedGate,
  addQubit,
  removeQubit,
  numQubits,
  clearCircuit,
  onRun,
  isSimulating,
  wasmLoaded,
}) => {
  const gates = ["H", "X", "Y", "Z", "CNOT"];
  return (
    <div className="p-4 bg-gray-800 rounded-lg shadow-lg text-white">
      <h2 className="text-xl font-bold mb-4 text-center border-b border-gray-600 pb-2">
        Controls
      </h2>
      <div className="mb-6">
        <h3 className="font-semibold mb-2 text-center">Qubits</h3>
        <div className="flex items-center justify-center space-x-4 bg-gray-700 p-2 rounded-md">
          <button
            onClick={removeQubit}
            className="px-3 py-1 bg-red-600 rounded-md hover:bg-red-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            disabled={numQubits <= 1}
          >
            -
          </button>
          <span className="font-mono text-lg">{numQubits}</span>
          <button
            onClick={addQubit}
            className="px-3 py-1 bg-green-600 rounded-md hover:bg-green-700 transition-colors disabled:opacity-50"
            disabled={numQubits >= 8}
          >
            +
          </button>
        </div>
      </div>
      <div>
        <h3 className="font-semibold mb-3 text-center">Gates</h3>
        <div className="grid grid-cols-3 gap-3">
          {gates.map((gate) => (
            <button
              key={gate}
              onClick={() => setSelectedGate(gate)}
              className={`p-2 rounded-md transition-all duration-200 border-2 ${selectedGate === gate ? "border-cyan-400 bg-gray-600" : "border-transparent bg-gray-700 hover:bg-gray-600"}`}
            >
              {gate}
            </button>
          ))}
        </div>
      </div>
      <div className="mt-6 space-y-2">
        <button
          onClick={onRun}
          disabled={!wasmLoaded || isSimulating}
          className="w-full py-2 bg-green-600 rounded-md hover:bg-green-700 transition-colors font-semibold disabled:opacity-50 disabled:cursor-wait"
        >
          {isSimulating
            ? "Simulating..."
            : wasmLoaded
              ? "Run Simulation"
              : "Loading Engine..."}
        </button>
        <button
          onClick={clearCircuit}
          className="w-full py-2 bg-indigo-600 rounded-md hover:bg-indigo-700 transition-colors font-semibold"
        >
          Clear Circuit
        </button>
      </div>
    </div>
  );
};

const CircuitGrid = ({
  numQubits,
  moments,
  setMoments,
  selectedGate,
  setSelectedGate,
}) => {
  const [cnotState, setCnotState] = useState({
    isConnecting: false,
    controlQubit: null,
    momentIndex: null,
  });
  const handleCellClick = (qubitIndex, momentIndex) => {
    if (selectedGate === "CNOT") {
      if (!cnotState.isConnecting) {
        setCnotState({
          isConnecting: true,
          controlQubit: qubitIndex,
          momentIndex,
        });
      } else {
        if (
          cnotState.momentIndex === momentIndex &&
          cnotState.controlQubit !== qubitIndex
        ) {
          const newMoments = [...moments];
          if (!newMoments[momentIndex]) newMoments[momentIndex] = [];
          const gateAtControl = newMoments[momentIndex].find(
            (g) =>
              g.qubit === cnotState.controlQubit ||
              (g.type === "CNOT" &&
                (g.control === cnotState.controlQubit ||
                  g.target === cnotState.controlQubit)),
          );
          const gateAtTarget = newMoments[momentIndex].find(
            (g) =>
              g.qubit === qubitIndex ||
              (g.type === "CNOT" &&
                (g.control === qubitIndex || g.target === qubitIndex)),
          );
          if (!gateAtControl && !gateAtTarget)
            newMoments[momentIndex].push({
              type: "CNOT",
              control: cnotState.controlQubit,
              target: qubitIndex,
            });
          setMoments(newMoments);
        }
        setCnotState({
          isConnecting: false,
          controlQubit: null,
          momentIndex: null,
        });
        setSelectedGate(null);
      }
    } else if (selectedGate) {
      const newMoments = [...moments];
      if (!newMoments[momentIndex]) newMoments[momentIndex] = [];
      const existingGateIndex = newMoments[momentIndex].findIndex(
        (g) =>
          g.qubit === qubitIndex ||
          (g.type === "CNOT" &&
            (g.control === qubitIndex || g.target === qubitIndex)),
      );
      if (existingGateIndex !== -1) {
        const existingGate = newMoments[momentIndex][existingGateIndex];
        if (existingGate.type === selectedGate)
          newMoments[momentIndex].splice(existingGateIndex, 1);
        else
          newMoments[momentIndex][existingGateIndex] = {
            type: selectedGate,
            qubit: qubitIndex,
          };
      } else {
        newMoments[momentIndex].push({ type: selectedGate, qubit: qubitIndex });
      }
      setMoments(newMoments);
    }
  };
  const numMoments = 20;
  const gridCells = useMemo(() => {
    const cells = [];
    for (let q = 0; q < numQubits; q++) {
      for (let m = 0; m < numMoments; m++) {
        const moment = moments[m] || [];
        let gateComponent = null;
        const singleQubitGate = moment.find((g) => g.qubit === q);
        const cnotGate = moment.find(
          (g) => g.type === "CNOT" && (g.control === q || g.target === q),
        );
        if (singleQubitGate)
          gateComponent = <GateIcon gate={singleQubitGate.type} />;
        else if (cnotGate)
          gateComponent =
            cnotGate.control === q ? <CnotControl /> : <CnotTarget />;
        const isPendingCnotControl =
          cnotState.isConnecting &&
          cnotState.controlQubit === q &&
          cnotState.momentIndex === m;
        cells.push(
          <div
            key={`${q}-${m}`}
            className={`w-12 h-12 flex items-center justify-center rounded-md transition-colors duration-200 cursor-pointer ${isPendingCnotControl ? "bg-blue-500/30" : "hover:bg-gray-700/50"}`}
            onClick={() => handleCellClick(q, m)}
          >
            {gateComponent}
          </div>,
        );
      }
    }
    return cells;
  }, [numQubits, moments, cnotState, selectedGate]);
  const cnotLines = useMemo(() => {
    const lines = [];
    moments.forEach((moment, momentIndex) => {
      if (!moment) return;
      moment.forEach((gate, gateIndex) => {
        if (gate.type === "CNOT") {
          const topQubit = Math.min(gate.control, gate.target);
          const bottomQubit = Math.max(gate.control, gate.target);
          const height = (bottomQubit - topQubit) * 48;
          lines.push(
            <div
              key={`${momentIndex}-${gateIndex}`}
              className="absolute bg-blue-500 w-0.5"
              style={{
                left: `${momentIndex * 48 + 23.5}px`,
                top: `${topQubit * 48 + 24}px`,
                height: `${height}px`,
              }}
            />,
          );
        }
      });
    });
    return lines;
  }, [moments]);
  return (
    <div className="flex-grow p-4 bg-gray-900 rounded-lg overflow-x-auto">
      <div
        className="relative inline-block"
        style={{ minWidth: `${numMoments * 48}px` }}
      >
        <div className="absolute inset-0 z-0">{cnotLines}</div>
        <div
          className="relative z-10 grid"
          style={{
            gridTemplateColumns: `repeat(${numMoments}, 48px)`,
            gridTemplateRows: `repeat(${numQubits}, 48px)`,
          }}
        >
          {Array.from({ length: numQubits }).map((_, qIndex) => (
            <div
              key={`line-${qIndex}`}
              className="absolute h-0.5 bg-gray-500"
              style={{
                top: `${qIndex * 48 + 23.5}px`,
                left: "16px",
                right: "16px",
                zIndex: -1,
              }}
            />
          ))}
          {gridCells}
        </div>
        {Array.from({ length: numQubits }).map((_, qIndex) => (
          <div
            key={`label-${qIndex}`}
            className="absolute text-gray-400 font-mono text-sm"
            style={{ left: "-40px", top: `${qIndex * 48 + 14}px` }}
          >
            q{qIndex}:
          </div>
        ))}
      </div>
    </div>
  );
};

const SimulationOutput = ({ result, numQubits, isSimulating }) => {
  if (isSimulating)
    return (
      <div className="mt-6 p-4 bg-gray-800 rounded-lg shadow-lg text-white text-center">
        <p className="text-lg font-semibold animate-pulse">Simulating...</p>
      </div>
    );
  if (!result)
    return (
      <div className="mt-6 p-4 bg-gray-800 rounded-lg shadow-lg text-white text-center">
        <p className="text-gray-400">Run a simulation to see the results.</p>
      </div>
    );
  if (result.error)
    return (
      <div className="mt-6 p-4 bg-red-900 border border-red-700 rounded-lg shadow-lg text-white">
        <h3 className="text-lg font-bold text-red-300">Simulation Error</h3>
        <p className="font-mono text-sm mt-2 text-red-200">{result.error}</p>
      </div>
    );
  const basisStates = Array.from(
    { length: 1 << numQubits },
    (_, i) => `|${i.toString(2).padStart(numQubits, "0")}⟩`,
  );
  return (
    <div className="mt-6 p-4 bg-gray-800 rounded-lg shadow-lg text-white">
      <h3 className="text-xl font-bold mb-4 text-center">Simulation Results</h3>
      <div className="space-y-2 font-mono text-sm max-h-60 overflow-y-auto pr-2">
        {result.probabilities.map((prob, i) => {
          const [re, im] = result.stateVector[i] || [0, 0];
          return (
            <div key={i} className="flex items-center gap-4">
              <span className="text-cyan-300 w-24 flex-shrink-0">
                {basisStates[i]}
              </span>
              <PhaseDisk re={re} im={im} />
              <div className="flex-grow bg-gray-700 rounded-full h-4">
                <div
                  className="bg-cyan-500 h-4 rounded-full transition-all duration-500"
                  style={{ width: `${prob * 100}%` }}
                />
              </div>
              <span className="w-20 text-right flex-shrink-0">
                {(prob * 100).toFixed(2)}%
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
};

//================================================================================
// --- PAGE COMPONENTS ---
//================================================================================

function SvmExperimentPage() {
  const [formState, setFormState] = useState({
    dsGenerator: "make_moons",
    dsSamples: 100,
    dsNoise: 0.1,
    dsTestSize: 0.3,
    kernelImage: "upcloud/quantum-svm:latest",
    trainerC: 1.0,
    outputModelName: "qsvm-model",
    outputPlotName: "qsvm-decision-boundary",
  });
  const [statusText, setStatusText] = useState("");
  const [error, setError] = useState("");
  const [results, setResults] = useState(null);

  const handleInputChange = (e) => {
    const { id, value, type } = e.target;
    setFormState((p) => ({
      ...p,
      [id]: type === "number" ? parseFloat(value) : value,
    }));
  };

  const handleRunExperiment = async () => {
    setStatusText("Submitting workflow...");
    setError("");
    setResults(null);
    console.log("Submitting Payload:", formState);
    setTimeout(() => {
      setStatusText("Workflow running...");
      setTimeout(() => {
        setStatusText("");
        const mockPlotUrl =
          "https://placehold.co/600x400/1e293b/e2e8f0?text=Decision+Boundary";
        const mockMetrics = { accuracy: 0.95, precision: 0.94, recall: 0.96 };
        setResults({
          plotUrl: mockPlotUrl,
          metricsText: JSON.stringify(mockMetrics, null, 2),
        });
      }, 4000);
    }, 2000);
  };

  return (
    <div className="p-8">
      <div className="max-w-4xl mx-auto">
        <h2 className="text-2xl font-bold text-white mb-2">
          Run a New Quantum SVM Experiment
        </h2>
        <p className="text-slate-400 mb-6">
          Specify parameters and click "Run Experiment" to submit.
        </p>
        <div className="space-y-8">
          <div className="p-6 bg-slate-800 rounded-lg border border-slate-700">
            <h3 className="text-lg font-semibold text-indigo-400 mb-4">
              Dataset Configuration
            </h3>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label htmlFor="dsGenerator" className="block text-sm">
                  Generator
                </label>
                <input
                  type="text"
                  id="dsGenerator"
                  value={formState.dsGenerator}
                  onChange={handleInputChange}
                  className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3 mt-1"
                />
              </div>
              <div>
                <label htmlFor="dsSamples" className="block text-sm">
                  Samples
                </label>
                <input
                  type="number"
                  id="dsSamples"
                  value={formState.dsSamples}
                  onChange={handleInputChange}
                  className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3 mt-1"
                />
              </div>
              <div>
                <label htmlFor="dsNoise" className="block text-sm">
                  Noise
                </label>
                <input
                  type="number"
                  id="dsNoise"
                  step="0.05"
                  value={formState.dsNoise}
                  onChange={handleInputChange}
                  className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3 mt-1"
                />
              </div>
              <div>
                <label htmlFor="dsTestSize" className="block text-sm">
                  Test Size
                </label>
                <input
                  type="number"
                  id="dsTestSize"
                  step="0.05"
                  value={formState.dsTestSize}
                  onChange={handleInputChange}
                  className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3 mt-1"
                />
              </div>
            </div>
          </div>
          <div className="grid md:grid-cols-2 gap-8">
            <div className="p-6 bg-slate-800 rounded-lg border border-slate-700">
              <h3 className="text-lg font-semibold text-indigo-400 mb-4">
                Kernel
              </h3>
              <label htmlFor="kernelImage" className="block text-sm">
                Container Image
              </label>
              <input
                type="text"
                id="kernelImage"
                value={formState.kernelImage}
                onChange={handleInputChange}
                className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3 mt-1"
              />
            </div>
            <div className="p-6 bg-slate-800 rounded-lg border border-slate-700">
              <h3 className="text-lg font-semibold text-indigo-400 mb-4">
                Trainer
              </h3>
              <label htmlFor="trainerC" className="block text-sm">
                C (Regularization)
              </label>
              <input
                type="number"
                id="trainerC"
                value={formState.trainerC}
                onChange={handleInputChange}
                className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3 mt-1"
              />
            </div>
          </div>
          <div className="pt-4 flex justify-end items-center gap-6">
            {statusText && <StatusDisplay statusText={statusText} />}
            <button
              onClick={handleRunExperiment}
              disabled={!!statusText}
              className="py-2 px-4 bg-indigo-600 text-white rounded-md hover:bg-indigo-700 font-semibold disabled:opacity-50"
            >
              Run Experiment
            </button>
          </div>
        </div>
        {error && <Notification message={error} type="error" />}
        {results && (
          <div className="mt-12">
            <h2 className="text-2xl font-bold text-white mb-6">Results</h2>
            <div className="space-y-8">
              <div>
                <h3 className="text-lg font-semibold text-indigo-400 mb-4">
                  Decision Boundary Plot
                </h3>
                <div className="p-4 bg-slate-800 rounded-lg border border-slate-700">
                  <img
                    src={results.plotUrl}
                    alt="Decision Boundary Plot"
                    className="max-w-full h-auto rounded"
                  />
                </div>
              </div>
              <div>
                <h3 className="text-lg font-semibold text-indigo-400 mb-4">
                  Performance Metrics
                </h3>
                <div className="p-4 bg-slate-950 rounded-lg border border-slate-700">
                  <pre className="font-code text-sm text-slate-300">
                    {results.metricsText}
                  </pre>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function QcbmSubmitterPage() {
  const [formState, setFormState] = useState({
    workflowName: "my-qcbm-training",
    namespace: "default",
    taskName: "qcbm-training-task",
    image: "qcbm-runner:latest",
    ansatz: "MyAnsatz",
    trainingData: "/path/to/data1.csv\n/path/to/data2.csv",
    optName: "Adam",
    optEpochs: 100,
    optLr: 0.01,
    optParams: "[0.1, 0.2, 0.3, 0.4]",
  });
  const [notification, setNotification] = useState({ message: "", type: "" });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const navigate = useNavigate();

  const handleInputChange = (e) => {
    const { id, value } = e.target;
    setFormState((p) => ({ ...p, [id]: value }));
  };

  const handleSubmit = async () => {
    setIsSubmitting(true);
    setNotification({ message: "", type: "" });
    console.log("Submitting QCBM Workflow:", formState);
    setTimeout(() => {
      setIsSubmitting(false);
      setNotification({
        message: `Workflow "${formState.workflowName}" created! Redirecting...`,
        type: "success",
      });
      setTimeout(() => navigate("/visualizer"), 2000);
    }, 1500);
  };

  return (
    <div className="p-8">
      <div className="max-w-4xl mx-auto">
        <h2 className="text-2xl font-bold text-white mb-2">
          Submit QCBM Workflow
        </h2>
        <p className="text-slate-400 mb-6">
          Define a QCBM task and its parameters to create a new workflow.
        </p>
        <Notification message={notification.message} type={notification.type} />
        <div className="space-y-6">
          <div className="p-6 bg-slate-800 rounded-lg border border-slate-700">
            <h3 className="text-lg font-semibold text-indigo-400 mb-4">
              Workflow Details
            </h3>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label htmlFor="workflowName" className="block text-sm">
                  Workflow Name
                </label>
                <input
                  type="text"
                  id="workflowName"
                  value={formState.workflowName}
                  onChange={handleInputChange}
                  className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3 mt-1"
                />
              </div>
              <div>
                <label htmlFor="namespace" className="block text-sm">
                  Namespace
                </label>
                <input
                  type="text"
                  id="namespace"
                  value={formState.namespace}
                  onChange={handleInputChange}
                  className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3 mt-1"
                />
              </div>
            </div>
          </div>
          <div className="p-6 bg-slate-800 rounded-lg border border-slate-700">
            <h3 className="text-lg font-semibold text-indigo-400 mb-4">
              Task Specification
            </h3>
            <div className="space-y-4">
              <label htmlFor="taskName" className="block text-sm">
                Task Name
              </label>
              <input
                type="text"
                id="taskName"
                value={formState.taskName}
                onChange={handleInputChange}
                className="bg-slate-900 border border-slate-600 rounded-md w-full py-2 px-3"
              />
              <label htmlFor="trainingData" className="block text-sm">
                Training Data (one per line)
              </label>
              <textarea
                id="trainingData"
                value={formState.trainingData}
                onChange={handleInputChange}
                rows="3"
                className="w-full bg-slate-950 font-code p-3 rounded-md"
              ></textarea>
            </div>
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

function QasmSubmitterPage() {
  const [workflowName, setWorkflowName] = useState("my-bell-state-exp");
  const [namespace, setNamespace] = useState("default");
  const [qasmCode, setQasmCode] = useState(
    'OPENQASM 2.0;\\ninclude "qelib1.inc";\\n\\nqreg q[2];\\ncreg c[2];\\n\\nh q[0];\\ncx q[0], q[1];\\nmeasure q -> c;',
  );
  const [notification, setNotification] = useState({ message: "", type: "" });
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmit = async () => {
    setIsSubmitting(true);
    setNotification({ message: "", type: "" });
    console.log("Submitting:", { workflowName, namespace, qasmCode });
    setTimeout(() => {
      setNotification({
        message: "Workflow submitted successfully!",
        type: "success",
      });
      setIsSubmitting(false);
    }, 1500);
  };

  return (
    <div className="p-8">
      <div className="max-w-4xl mx-auto">
        <h2 className="text-2xl font-bold text-white mb-2">
          Submit OpenQASM Workflow
        </h2>
        <p className="text-slate-400 mb-6">
          Paste your OpenQASM 2.0 code below to run a new workflow.
        </p>
        <Notification message={notification.message} type={notification.type} />
        <div className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label htmlFor="qasm-workflow-name" className="block text-sm">
                Workflow Name
              </label>
              <input
                type="text"
                id="qasm-workflow-name"
                value={workflowName}
                onChange={(e) => setWorkflowName(e.target.value)}
                className="bg-slate-800 border border-slate-600 rounded-md py-2 px-3 text-white w-full mt-1"
              />
            </div>
            <div>
              <label htmlFor="qasm-namespace" className="block text-sm">
                Namespace
              </label>
              <input
                type="text"
                id="qasm-namespace"
                value={namespace}
                onChange={(e) => setNamespace(e.target.value)}
                className="bg-slate-800 border border-slate-600 rounded-md py-2 px-3 text-white w-full mt-1"
              />
            </div>
          </div>
          <div>
            <label htmlFor="qasm-input" className="block text-sm">
              OpenQASM 2.0 Code
            </label>
            <textarea
              id="qasm-input"
              rows="15"
              value={qasmCode}
              onChange={(e) => setQasmCode(e.target.value)}
              className="w-full bg-slate-950 border border-slate-600 rounded-md p-3 font-code text-indigo-300 mt-1"
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

function MlSvmPage() {
  const [dataFile, setDataFile] = useState(null);
  const [targetColumn, setTargetColumn] = useState("target");
  const [testSize, setTestSize] = useState(0.3);
  const [results, setResults] = useState(null);
  const [error, setError] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  const handleFileChange = (e) => setDataFile(e.target.files[0]);

  const handleRun = async () => {
    if (!dataFile) {
      setError("Please select a CSV data file.");
      return;
    }
    setIsLoading(true);
    setError("");
    setResults(null);
    console.log("Submitting ML SVM job with file:", dataFile.name);
    setTimeout(() => {
      const mockResults = {
        metrics:
          "Classification Report:\n              precision    recall  f1-score   support...",
        plot_base_64:
          "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
      };
      setResults(mockResults);
      setIsLoading(false);
    }, 2000);
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
              className="py-2 px-4 bg-indigo-600 text-white rounded-md hover:bg-indigo-700 font-semibold disabled:opacity-50"
            >
              {isLoading ? "Running..." : "Run SVM"}
            </button>
          </div>
        </div>
        {error && <Notification message={error} type="error" />}
        {results && (
          <div className="mt-8">
            <h3 className="text-lg font-semibold text-indigo-400 mb-4">
              Results
            </h3>
            <div className="bg-slate-950 rounded-md p-3 text-sm font-code text-slate-300">
              <pre>{results.metrics}</pre>
            </div>
            <img
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

function CircuitSimulatorPage() {
  const [numQubits, setNumQubits] = useState(3);
  const [moments, setMoments] = useState([]);
  const [selectedGate, setSelectedGate] = useState(null);
  const [wasm, setWasm] = useState(null);
  const [simResult, setSimResult] = useState(null);
  const [isSimulating, setIsSimulating] = useState(false);

  useEffect(() => {
    const loadWasm = async () => {
      try {
        const mockWasm = {
          run_simulation: (circuitJson) => {
            const circuit = JSON.parse(circuitJson);
            if (
              circuit.numQubits === 2 &&
              circuit.moments.length === 2 &&
              circuit.moments[0]?.[0]?.type === "H" &&
              circuit.moments[0]?.[0]?.qubit === 0 &&
              circuit.moments[1]?.[0]?.type === "CNOT"
            ) {
              return JSON.stringify({
                stateVector: [
                  [0.7071, 0],
                  [0, 0],
                  [0, 0],
                  [0.7071, 0],
                ],
                probabilities: [0.5, 0, 0, 0.5],
              });
            }
            const numStates = 1 << circuit.numQubits;
            const probabilities = new Array(numStates).fill(0);
            if (numStates > 0) probabilities[0] = 1.0;
            const stateVector = new Array(numStates).fill([0, 0]);
            if (numStates > 0) stateVector[0] = [1, 0];
            return JSON.stringify({ stateVector, probabilities });
          },
        };
        setTimeout(() => setWasm(mockWasm), 500);
      } catch (err) {
        console.error("Error loading WASM module:", err);
      }
    };
    loadWasm();
  }, []);

  const clearCircuit = useCallback(() => {
    setMoments([]);
    setSelectedGate(null);
    setSimResult(null);
  }, []);
  const handleQubitChange = (newNumQubits) => {
    if (newNumQubits < numQubits) {
      const newMoments = moments
        .map((moment) =>
          moment.filter((gate) =>
            gate.type === "CNOT"
              ? gate.control < newNumQubits && gate.target < newNumQubits
              : gate.qubit < newNumQubits,
          ),
        )
        .filter((moment) => moment && moment.length > 0);
      setMoments(newMoments);
    }
    setNumQubits(newQubits);
    setSimResult(null);
  };
  const handleRunSimulation = () => {
    if (!wasm || isSimulating) return;
    setIsSimulating(true);
    setSimResult(null);
    const circuitPayload = {
      numQubits: numQubits,
      moments: moments.filter((m) => m && m.length > 0),
    };
    setTimeout(() => {
      try {
        const resultJson = wasm.run_simulation(JSON.stringify(circuitPayload));
        setSimResult(JSON.parse(resultJson));
      } catch (e) {
        setSimResult({ error: e.message });
      } finally {
        setIsSimulating(false);
      }
    }, 100);
  };

  return (
    <div className="bg-gray-900 text-white flex flex-col items-center justify-center p-4 font-sans h-full">
      <div className="w-full max-w-7xl mx-auto">
        <header className="text-center mb-6">
          <h1 className="text-4xl font-bold text-cyan-300">
            Quantum Circuit Simulator
          </h1>
          <p className="text-gray-400 mt-2">
            Design a circuit, then press "Run Simulation" to see the final state
            probabilities.
          </p>
        </header>
        <main className="flex flex-col md:flex-row gap-6">
          <aside className="w-full md:w-64 flex-shrink-0">
            <GatePalette
              selectedGate={selectedGate}
              setSelectedGate={setSelectedGate}
              addQubit={() => handleQubitChange(numQubits + 1)}
              removeQubit={() => handleQubitChange(numQubits - 1)}
              numQubits={numQubits}
              clearCircuit={clearCircuit}
              onRun={handleRunSimulation}
              isSimulating={isSimulating}
              wasmLoaded={!!wasm}
            />
          </aside>
          <div className="flex-grow flex flex-col">
            <div className="bg-gray-800 p-6 rounded-lg shadow-lg">
              <CircuitGrid
                numQubits={numQubits}
                moments={moments}
                setMoments={setMoments}
                selectedGate={selectedGate}
                setSelectedGate={setSelectedGate}
              />
            </div>
            <SimulationOutput
              result={simResult}
              numQubits={numQubits}
              isSimulating={isSimulating}
            />
          </div>
        </main>
      </div>
    </div>
  );
}

//================================================================================
// --- APP LAYOUT & ROUTER ---
//================================================================================

const AppLayout = () => (
  <div className="flex flex-col h-screen bg-slate-900 text-slate-200">
    <header className="bg-slate-900/70 backdrop-blur-sm border-b border-slate-700/80 p-4 shadow-lg z-20 flex justify-between items-center shrink-0">
      <div className="flex items-center gap-8">
        <h1 className="text-xl font-bold text-white">Quantum UI</h1>
        <nav className="flex items-center gap-x-1 sm:gap-x-3 md:gap-x-6 text-xs sm:text-sm font-medium text-slate-400">
          <NavLink
            to="/visualizer"
            className={({ isActive }) =>
              `py-2 px-1 md:px-2 nav-link ${isActive ? "active" : ""}`
            }
          >
            Visualizer
          </NavLink>
          <NavLink
            to="/svm"
            className={({ isActive }) =>
              `py-2 px-1 md:px-2 nav-link ${isActive ? "active" : ""}`
            }
          >
            SVM Exp
          </NavLink>
          <NavLink
            to="/qcbm"
            className={({ isActive }) =>
              `py-2 px-1 md:px-2 nav-link ${isActive ? "active" : ""}`
            }
          >
            Submit QCBM
          </NavLink>
          <NavLink
            to="/qasm"
            className={({ isActive }) =>
              `py-2 px-1 md:px-2 nav-link ${isActive ? "active" : ""}`
            }
          >
            Submit QASM
          </NavLink>
          <NavLink
            to="/ml-svm"
            className={({ isActive }) =>
              `py-2 px-1 md:px-2 nav-link ${isActive ? "active" : ""}`
            }
          >
            ML SVM
          </NavLink>
          <NavLink
            to="/simulator"
            className={({ isActive }) =>
              `py-2 px-1 md:px-2 nav-link ${isActive ? "active" : ""}`
            }
          >
            Simulator
          </NavLink>
        </nav>
      </div>
    </header>
    <div className="flex-1 overflow-y-auto relative">
      <Outlet />
    </div>
  </div>
);

export default function App() {
  return (
    <>
      <style>{`
                .nav-link {
                    border-bottom: 2px solid transparent;
                    transition: color 0.2s, border-color 0.2s;
                    cursor: pointer;
                }
                .nav-link:hover {
                    color: #e2e8f0; /* slate-200 */
                }
                .nav-link.active {
                    color: #818cf8; /* indigo-400 */
                    border-bottom-color: #818cf8; /* indigo-400 */
                }
            `}</style>
      <ClientOnly>
        <Routes>
          <Route path="/" element={<AppLayout />}>
            <Route index element={<VisualizerPage />} />
            <Route path="visualizer" element={<VisualizerPage />} />
            <Route path="svm" element={<SvmExperimentPage />} />
            <Route path="qcbm" element={<QcbmSubmitterPage />} />
            <Route path="qasm" element={<QasmSubmitterPage />} />
            <Route path="ml-svm" element={<MlSvmPage />} />
            <Route path="simulator" element={<CircuitSimulatorPage />} />
          </Route>
        </Routes>
      </ClientOnly>
    </>
  );
}
