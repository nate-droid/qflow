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

// NOTE: The 'cytoscape-dagre' library is expected to be loaded globally in this environment.
// The direct import is removed to resolve a build error where the module cannot be found.
// The registration of the dagre layout is assumed to be handled by the global script.
// Make sure to run: npm install cytoscape dagre d3

cytoscape.use(cytoscapeDagre);

import VisualizerPage from "./pages/VisualizerPage";
import CircuitSimulatorPage from "./pages/CircuitSimulatorPage";
import MlSvmPage from "./pages/MlSvmPage";
import QasmSubmitterPage from "./pages/QasmSubmitterPage";
import QcbmSubmitterPage from "./pages/QcbmSubmitterPage";
import SvmExperimentPage from "./pages/SvmExperimentPage";

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
