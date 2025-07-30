import React, { useState, useCallback, useMemo, useEffect } from "react";

// --- Helper Components ---

// Icon for a specific gate
const GateIcon = ({ gate, theta }) => {
  const gateStyles = {
    H: "bg-yellow-500 border-yellow-600",
    X: "bg-red-500 border-red-600",
    Y: "bg-green-500 border-green-600",
    Z: "bg-blue-500 border-blue-600",
    RX: "bg-purple-500 border-purple-600",
    RY: "bg-pink-500 border-pink-600",
    RZ: "bg-orange-500 border-orange-600",
  };
  const style = gateStyles[gate] || "bg-gray-400 border-gray-500";
  return (
    <div
      className={`w-8 h-8 rounded-md flex items-center justify-center text-white font-bold text-xs border-b-2 ${style} flex-col`}
      title={theta !== undefined ? `${gate}(${theta})` : gate}
    >
      <span>
        {gate}
        {theta !== undefined ? (
          <span style={{ fontSize: "0.7em" }}>
            <sub>θ</sub>
          </span>
        ) : null}
      </span>
      {theta !== undefined && (
        <span style={{ fontSize: "0.7em" }}>{Number(theta).toFixed(2)}</span>
      )}
    </div>
  );
};

// CNOT gate's control part
const CnotControl = () => (
  <div className="w-8 h-8 flex items-center justify-center">
    <div className="w-4 h-4 bg-blue-500 rounded-full border-2 border-blue-300"></div>
  </div>
);

// CNOT gate's target part
const CnotTarget = () => (
  <div className="w-8 h-8 flex items-center justify-center relative">
    <div className="w-8 h-8 border-2 border-blue-500 rounded-full flex items-center justify-center">
      <div className="w-0.5 h-5 bg-blue-500 absolute"></div>
      <div className="w-5 h-0.5 bg-blue-500 absolute"></div>
    </div>
  </div>
);

// --- Main Application Components ---

const GatePalette = ({
  selectedGate,
  setSelectedGate,
  theta,
  setTheta,
  addQubit,
  removeQubit,
  numQubits,
  clearCircuit,
  onRun,
  isSimulating,
  wasmLoaded,
}) => {
  const gates = ["H", "X", "Y", "Z", "RX", "RY", "RZ", "CNOT"];

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
        {(selectedGate === "RX" ||
          selectedGate === "RY" ||
          selectedGate === "RZ") && (
          <div className="mt-4 flex flex-col items-center">
            <label htmlFor="theta-input" className="mb-1 text-sm font-medium">
              θ (radians)
            </label>
            <input
              id="theta-input"
              type="number"
              step="any"
              min="0"
              max={Math.PI * 2}
              className="w-24 px-2 py-1 rounded bg-gray-700 border border-gray-500 text-white"
              value={theta}
              onChange={(e) => {
                const val = Number(e.target.value);
                if (!isNaN(val)) setTheta(val);
              }}
              placeholder="e.g. 3.14"
            />
            <span className="text-xs text-gray-400 mt-1">
              Set θ for rotation
            </span>
          </div>
        )}
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
  theta,
}) => {
  const [cnotState, setCnotState] = useState({
    isConnecting: false,
    controlQubit: null,
    momentIndex: null,
  });

  // Popover state for editing theta
  const [thetaEdit, setThetaEdit] = useState(null); // {momentIndex, qubitIndex, theta, left, top}
  const popoverRef = React.useRef();

  // Close popover on outside click or Escape
  useEffect(() => {
    if (!thetaEdit) return;
    function handle(e) {
      if (popoverRef.current && !popoverRef.current.contains(e.target)) {
        setThetaEdit(null);
      }
    }
    function handleEsc(e) {
      if (e.key === "Escape") setThetaEdit(null);
    }
    document.addEventListener("mousedown", handle);
    document.addEventListener("keydown", handleEsc);
    return () => {
      document.removeEventListener("mousedown", handle);
      document.removeEventListener("keydown", handleEsc);
    };
  }, [thetaEdit]);

  const handleCellClick = (qubitIndex, momentIndex) => {
    // --- CNOT Gate Logic ---
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

          if (!gateAtControl && !gateAtTarget) {
            newMoments[momentIndex].push({
              type: "CNOT",
              control: cnotState.controlQubit,
              target: qubitIndex,
            });
            setMoments(newMoments);
          } else {
            console.warn("Cannot place CNOT over an existing gate.");
          }
        }
        setCnotState({
          isConnecting: false,
          controlQubit: null,
          momentIndex: null,
        });
        setSelectedGate(null);
      }
    }
    // --- Single Qubit Gate Logic ---
    else if (selectedGate) {
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
        if (existingGate.type === selectedGate) {
          newMoments[momentIndex].splice(existingGateIndex, 1);
        } else {
          newMoments[momentIndex][existingGateIndex] = {
            type: selectedGate,
            qubit: qubitIndex,
          };
        }
      } else {
        // If rotation gate, include theta
        if (
          selectedGate === "RX" ||
          selectedGate === "RY" ||
          selectedGate === "RZ"
        ) {
          // Only place if theta is a valid number
          if (typeof theta === "number" && !isNaN(theta)) {
            newMoments[momentIndex].push({
              type: selectedGate,
              qubit: qubitIndex,
              theta,
            });
          } else {
            alert("Please enter a valid θ (theta) value for rotation gates.");
            return;
          }
        } else {
          newMoments[momentIndex].push({
            type: selectedGate,
            qubit: qubitIndex,
          });
        }
      }
      setMoments(newMoments);
    }
  };

  const numMoments = 20;
  const cellHeight = 64; // Increased for more space
  const cellWidth = 48;

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

        if (singleQubitGate) {
          if (
            singleQubitGate.type === "RX" ||
            singleQubitGate.type === "RY" ||
            singleQubitGate.type === "RZ"
          ) {
            gateComponent = (
              <GateIcon
                gate={singleQubitGate.type}
                theta={singleQubitGate.theta}
              />
            );
          } else {
            gateComponent = <GateIcon gate={singleQubitGate.type} />;
          }
        } else if (cnotGate) {
          if (cnotGate.control === q) {
            gateComponent = <CnotControl />;
          } else if (cnotGate.target === q) {
            gateComponent = <CnotTarget />;
          }
        }

        const isPendingCnotControl =
          cnotState.isConnecting &&
          cnotState.controlQubit === q &&
          cnotState.momentIndex === m;

        cells.push(
          <div
            key={`${q}-${m}`}
            className={`w-12`}
            style={{
              width: `${cellWidth}px`,
              height: `${cellHeight}px`,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              borderRadius: "0.375rem",
              transition: "background-color 0.2s",
              cursor: "pointer",
              backgroundColor: isPendingCnotControl
                ? "rgba(59,130,246,0.18)"
                : undefined,
            }}
            onClick={(e) => {
              // If RX/RY/RZ, open popover for theta editing
              if (
                singleQubitGate &&
                ["RX", "RY", "RZ"].includes(singleQubitGate.type)
              ) {
                const rect = e.currentTarget.getBoundingClientRect();
                setThetaEdit({
                  momentIndex: m,
                  qubitIndex: q,
                  theta: singleQubitGate.theta,
                  left: rect.left + window.scrollX,
                  top: rect.top + window.scrollY,
                });
                e.stopPropagation();
              } else {
                handleCellClick(q, m);
              }
            }}
            title={
              singleQubitGate &&
              ["RX", "RY", "RZ"].includes(singleQubitGate.type)
                ? "Click to edit θ"
                : undefined
            }
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
          const height = (bottomQubit - topQubit) * cellHeight;

          lines.push(
            <div
              key={`${momentIndex}-${gateIndex}`}
              className="absolute bg-blue-500 w-0.5"
              style={{
                left: `${momentIndex * cellWidth + 23.5}px`,
                top: `${topQubit * cellHeight + 32}px`,
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
        style={{ minWidth: `${numMoments * cellWidth}px` }}
      >
        <div className="absolute inset-0 z-0">{cnotLines}</div>
        <div
          className="relative z-10 grid"
          style={{
            gridTemplateColumns: `repeat(${numMoments}, ${cellWidth}px)`,
            gridTemplateRows: `repeat(${numQubits}, ${cellHeight}px)`,
          }}
        >
          {Array.from({ length: numQubits }).map((_, qIndex) => (
            <div
              key={`line-${qIndex}`}
              className="absolute h-0.5 bg-gray-500"
              style={{
                top: `${qIndex * cellHeight + 31.5}px`,
                left: "16px",
                right: "16px",
                zIndex: -1,
              }}
            />
          ))}
          {gridCells}
        </div>
        {thetaEdit && (
          <div
            ref={popoverRef}
            style={{
              position: "absolute",
              left: thetaEdit.left - 24,
              top: thetaEdit.top + 48,
              zIndex: 50,
              background: "#1f2937",
              border: "1px solid #4b5563",
              borderRadius: "0.5rem",
              padding: "1rem",
              boxShadow: "0 2px 8px rgba(0,0,0,0.25)",
              minWidth: "160px",
            }}
          >
            <div className="mb-2 text-white text-sm font-semibold">
              Edit θ for{" "}
              {["RX", "RY", "RZ"].includes(
                moments[thetaEdit.momentIndex]?.find(
                  (g) => g.qubit === thetaEdit.qubitIndex,
                )?.type,
              )
                ? moments[thetaEdit.momentIndex].find(
                    (g) => g.qubit === thetaEdit.qubitIndex,
                  ).type
                : ""}
            </div>
            <input
              type="number"
              step="any"
              min="0"
              max={Math.PI * 2}
              value={thetaEdit.theta}
              onChange={(e) => {
                const val = Number(e.target.value);
                if (!isNaN(val)) setThetaEdit({ ...thetaEdit, theta: val });
              }}
              className="w-full px-2 py-1 rounded bg-gray-700 border border-gray-500 text-white mb-2"
              autoFocus
            />
            <div className="flex justify-end space-x-2">
              <button
                className="px-3 py-1 rounded bg-green-600 hover:bg-green-700 text-white text-xs"
                onClick={() => {
                  // Save theta to the correct gate
                  const newMoments = [...moments];
                  const moment = newMoments[thetaEdit.momentIndex];
                  const gateIdx = moment.findIndex(
                    (g) =>
                      g.qubit === thetaEdit.qubitIndex &&
                      ["RX", "RY", "RZ"].includes(g.type),
                  );
                  if (gateIdx !== -1) {
                    moment[gateIdx] = {
                      ...moment[gateIdx],
                      theta: thetaEdit.theta,
                    };
                    setMoments(newMoments);
                  }
                  setThetaEdit(null);
                }}
              >
                Save
              </button>
              <button
                className="px-3 py-1 rounded bg-gray-600 hover:bg-gray-700 text-white text-xs"
                onClick={() => setThetaEdit(null)}
              >
                Cancel
              </button>
            </div>
          </div>
        )}
        {Array.from({ length: numQubits }).map((_, qIndex) => (
          <div
            key={`label-${qIndex}`}
            className="absolute text-gray-400 font-mono text-sm"
            style={{ left: "-40px", top: `${qIndex * cellHeight + 20}px` }}
          >
            q{qIndex}:
          </div>
        ))}
      </div>
    </div>
  );
};

const SimulationOutput = ({ result, numQubits, isSimulating }) => {
  if (isSimulating) {
    return (
      <div className="mt-6 p-4 bg-gray-800 rounded-lg shadow-lg text-white text-center">
        <p className="text-lg font-semibold animate-pulse">Simulating...</p>
      </div>
    );
  }

  if (!result) {
    return (
      <div className="mt-6 p-4 bg-gray-800 rounded-lg shadow-lg text-white text-center">
        <p className="text-gray-400">Run a simulation to see the results.</p>
      </div>
    );
  }

  if (result.error) {
    return (
      <div className="mt-6 p-4 bg-red-900 border border-red-700 rounded-lg shadow-lg text-white">
        <h3 className="text-lg font-bold text-red-300">Simulation Error</h3>
        <p className="font-mono text-sm mt-2 text-red-200">{result.error}</p>
      </div>
    );
  }

  const basisStates = Array.from(
    { length: 1 << numQubits },
    (_, i) => `|${i.toString(2).padStart(numQubits, "0")}⟩`,
  );

  return (
    <div className="mt-6 p-4 bg-gray-800 rounded-lg shadow-lg text-white">
      <h3 className="text-xl font-bold mb-4 text-center">Simulation Results</h3>
      <div className="space-y-2 font-mono text-sm max-h-60 overflow-y-auto pr-2">
        {result.probabilities.map((prob, i) => (
          <div key={i} className="flex items-center gap-4">
            <span className="text-cyan-300">{basisStates[i]}</span>
            <div className="flex-grow bg-gray-700 rounded-full h-4">
              <div
                className="bg-cyan-500 h-4 rounded-full transition-all duration-500"
                style={{ width: `${prob * 100}%` }}
              />
            </div>
            <span className="w-20 text-right">{(prob * 100).toFixed(2)}%</span>
          </div>
        ))}
      </div>
    </div>
  );
};

// --- App Component ---
export default function CircuitSimulatorPage() {
  const [numQubits, setNumQubits] = useState(3);
  const [moments, setMoments] = useState([]);
  const [selectedGate, setSelectedGate] = useState(null);
  const [wasm, setWasm] = useState(null);
  const [simResult, setSimResult] = useState(null);
  const [isSimulating, setIsSimulating] = useState(false);
  const [theta, setTheta] = useState(Math.PI / 2); // Default theta for rotation gates

  // QASM export modal state
  const [isQasmModalOpen, setIsQasmModalOpen] = useState(false);
  const [qasmOutput, setQasmOutput] = useState("");
  const [isCopying, setIsCopying] = useState(false);

  // Effect to load the WASM module
  useEffect(() => {
    // Renamed for clarity
    const loadAndInitializeWasm = async () => {
      try {
        // 1. Import the JavaScript glue code generated by wasm-pack
        const wasmModule = await import("quantum-simulator-wasm");

        // 2. IMPORTANT: Call and await the default export.
        //    This is an async function that fetches and initializes the
        //    actual .wasm file.
        await wasmModule.default();

        // 3. Now that initialization is complete, store the module's
        //    functions in your component's state.
        setWasm(wasmModule);
      } catch (err) {
        console.error("Error loading and initializing WASM module:", err);
      }
    };

    loadAndInitializeWasm();
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
          moment.filter((gate) => {
            if (gate.type === "CNOT") {
              return gate.control < newNumQubits && gate.target < newNumQubits;
            }
            return gate.qubit < newNumQubits;
          }),
        )
        .filter((moment) => moment && moment.length > 0);
      setMoments(newMoments);
    }
    setNumQubits(newNumQubits);
    setSimResult(null); // Results are invalid when qubits change
  };

  const handleRunSimulation = () => {
    if (!wasm || isSimulating) return;
    setIsSimulating(true);
    setSimResult(null);

    const circuitPayload = {
      numQubits: numQubits,
      moments: moments.filter((m) => m && m.length > 0),
    };
    const circuitJson = JSON.stringify(circuitPayload);

    // Use a timeout to allow the UI to update to the "loading" state
    setTimeout(() => {
      try {
        const resultJson = wasm.run_simulation(circuitJson);
        const result = JSON.parse(resultJson);
        setSimResult(result);
      } catch (e) {
        console.error("Error running simulation:", e);
        setSimResult({ error: e.message });
      } finally {
        setIsSimulating(false);
      }
    }, 100);
  };

  // QASM Export logic
  const handleExportQasm = () => {
    if (!wasm) return;
    // Sanitize theta for RX/RY/RZ gates before export
    const sanitizedMoments = moments.map((moment) =>
      moment
        ? moment.map((gate) => {
            if (
              (gate.type === "RX" ||
                gate.type === "RY" ||
                gate.type === "RZ") &&
              (typeof gate.theta !== "number" || isNaN(gate.theta))
            ) {
              // Default to pi/2 if invalid
              return { ...gate, theta: Math.PI / 2 };
            }
            return gate;
          })
        : moment,
    );
    const circuitPayload = {
      numQubits: numQubits,
      moments: sanitizedMoments.filter((m) => m && m.length > 0),
    };
    const circuitJson = JSON.stringify(circuitPayload);
    try {
      const qasmStr = wasm.compile_circuit_to_qasm(circuitJson);
      setQasmOutput(qasmStr);
      setIsQasmModalOpen(true);
    } catch (e) {
      setQasmOutput("// Error generating QASM: " + e.message);
      setIsQasmModalOpen(true);
    }
  };

  const handleCopyQasm = async () => {
    if (!qasmOutput) return;
    setIsCopying(true);
    try {
      await navigator.clipboard.writeText(qasmOutput);
    } catch (e) {
      // fallback: select and copy via execCommand (not implemented here)
    }
    setTimeout(() => setIsCopying(false), 1000);
  };

  return (
    <div className="bg-gray-900 text-white min-h-screen flex flex-col items-center justify-center p-4 font-sans">
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
              theta={theta}
              setTheta={setTheta}
              addQubit={() => handleQubitChange(numQubits + 1)}
              removeQubit={() => handleQubitChange(numQubits - 1)}
              numQubits={numQubits}
              clearCircuit={clearCircuit}
              onRun={handleRunSimulation}
              isSimulating={isSimulating}
              wasmLoaded={!!wasm}
            />
            <button
              onClick={handleExportQasm}
              disabled={!wasm}
              className="w-full mt-4 py-2 bg-cyan-700 rounded-md hover:bg-cyan-800 transition-colors font-semibold disabled:opacity-50"
            >
              Export QASM
            </button>
          </aside>

          <div className="flex-grow flex flex-col">
            <div
              className="bg-gray-800 p-6 rounded-lg shadow-lg"
              style={{ "--num-qubits": numQubits }}
            >
              <CircuitGrid
                numQubits={numQubits}
                moments={moments}
                setMoments={setMoments}
                selectedGate={selectedGate}
                setSelectedGate={setSelectedGate}
                theta={theta}
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

      {/* QASM Export Modal */}
      {isQasmModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-60">
          <div className="bg-gray-900 rounded-lg shadow-2xl max-w-2xl w-full mx-4 p-6 relative">
            <button
              className="absolute top-2 right-2 text-gray-400 hover:text-white text-2xl font-bold"
              onClick={() => setIsQasmModalOpen(false)}
              aria-label="Close"
            >
              ×
            </button>
            <h2 className="text-xl font-bold mb-4 text-cyan-300 text-center">
              Exported OpenQASM
            </h2>
            <div className="mb-4">
              <pre className="bg-gray-800 rounded p-4 text-green-200 text-sm max-h-72 overflow-auto whitespace-pre-wrap">
                {qasmOutput}
              </pre>
            </div>
            <div className="flex justify-end gap-2">
              <button
                onClick={handleCopyQasm}
                className="px-4 py-2 bg-cyan-700 rounded hover:bg-cyan-800 font-semibold transition-colors"
              >
                {isCopying ? "Copied!" : "Copy to Clipboard"}
              </button>
              <button
                onClick={() => setIsQasmModalOpen(false)}
                className="px-4 py-2 bg-gray-700 rounded hover:bg-gray-800 font-semibold transition-colors"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
